/**
 * Le client HTTP de la plateforme : une seule façon d'atteindre l'API.
 *
 * Il porte les quatre choses qu'aucun écran ne doit avoir à connaître :
 *
 *  1. LA SESSION VOYAGE SEULE. L'API pose deux cookies `HttpOnly` que le site ne
 *     peut pas lire ; `credentials: 'include'` suffit à les renvoyer. En rendu
 *     serveur, le navigateur n'a pas parlé à l'API : c'est Nuxt qui appelle, et
 *     il faut lui repasser le cookie reçu — sans quoi toute page rendue côté
 *     serveur s'affiche déconnectée avant de se corriger à l'hydratation.
 *
 *  2. LA ROTATION DU JETON, une fois, et jamais deux en parallèle. Le jeton
 *     d'accès dure un quart d'heure, la session douze heures : sans rotation,
 *     quelqu'un qui lit un dossier vingt minutes se ferait éjecter en cliquant.
 *     **Elle n'a lieu que dans le navigateur** — le cookie de rafraîchissement
 *     est limité au path `/api/auth`, donc il n'atteint jamais le serveur Nuxt,
 *     qui vit sur un autre port. Ce n'est pas un manque : c'est ce qui borne le
 *     dégât d'une fuite, et le rendu serveur n'a rien à rafraîchir.
 *
 *  3. LES ERREURS SONT TRADUITES UNE FOIS POUR TOUTES en `ApiRequestError` ou
 *     `ApiUnreachableError` (voir `utils/api-error.ts`). Aucun écran ne regarde
 *     un `status` brut.
 *
 *  4. LE MODE DÉGRADÉ. Une API injoignable n'est pas une error d'écran : c'est
 *     un état de la plateforme, partagé, qu'un bandeau annonce une seule fois au
 *     lieu de vingt messages d'error identiques.
 */
import type { ComputedRef } from 'vue'
import { ApiRequestError, ApiUnreachableError, ForbiddenError, normalizeApiError } from '~/utils/api-error'

/** Ce que rend la rotation du jeton. Le contrat de `POST /auth/refresh`. */
type RefreshOutcome = { status: 'renewed' | 'expired' }

/**
 * Rotation en cours, partagée par tous les appels du même onglet.
 *
 * Variable de module, et c'est délibéré : trois appels qui échouent en même
 * temps doivent déclencher UNE rotation, pas trois — la deuxième invaliderait le
 * jeton que la première vient de poser, et l'API y verrait un rejeu, ce qui
 * ferme toutes les sessions. Elle n'est touchée que sous `import.meta.client` :
 * en rendu serveur, un état de module serait partagé entre deux visiteurs.
 */
let pendingRotation: Promise<boolean> | null = null

export interface ApiHttp {
  /** L'adresse de l'API. Vide = données simulées. */
  baseURL: string
  /** L'API est-elle configurée ? Faux tant que `NUXT_PUBLIC_API_BASE` est vide. */
  isConfigured: ComputedRef<boolean>
  /** Client brut, sans rotation ni traduction d'erreur. Réservé aux cas non couverts. */
  client: ReturnType<typeof $fetch.create>
  /** Un appel : rotation du jeton comprise, erreurs traduites, mode dégradé tenu. */
  request: <T>(path: string, options?: Record<string, unknown>) => Promise<T>
  /** Tente une rotation du jeton. Vrai si la session continue. */
  refreshSession: () => Promise<boolean>
}

export function createApiHttp(): ApiHttp {
  const config = useRuntimeConfig()
  const baseURL = String(config.public.apiBase ?? '')
  const isConfigured = computed(() => baseURL.length > 0)

  const status = useApiStatus()
  const witness = useSessionWitness()

  /**
   * La locale vient de l'instance i18n de l'application, PAS de `useI18n()` :
   * ce fichier est consommé depuis des stores initialisés par des middlewares
   * de navigation, hors de tout composant, où `useI18n()` lève.
   */
  const { $i18n } = useNuxtApp()

  // En rendu serveur, le cookie reçu du navigateur doit repartir vers l'API :
  // `credentials: 'include'` ne vaut que dans un navigateur.
  const incomingCookie = import.meta.server ? useRequestHeaders(['cookie']).cookie : undefined

  const client = $fetch.create({
    baseURL,
    credentials: 'include',
    // Une seule reprise, et seulement sur ce qui se rejoue sans effet de bord.
    // Un 500 n'y est PAS : rien ne dit que l'écriture n'a pas eu lieu.
    retry: 1,
    retryStatusCodes: [408, 425, 429, 502, 503, 504],
    onRequest({ options }) {
      // L'API résout les colonnes `platform.i18n_text` selon cet en-tête.
      options.headers.set('Accept-Language', $i18n.locale.value)
      if (incomingCookie) options.headers.set('cookie', incomingCookie)
    },
  })

  /**
   * Rotation du jeton. Rend vrai si la session continue.
   *
   * Le refus de rotation n'est PAS une erreur : `POST /auth/refresh` rend 200
   * avec `{ status: "expired" }` quand la session est finie, et efface ses
   * cookies au passage. Seul le rejeu d'un jeton déjà consommé sort en 401 —
   * l'API ayant alors fermé toutes les sessions, il n'y a rien à retenter.
   */
  async function rotate(): Promise<boolean> {
    try {
      const outcome = await client<RefreshOutcome>('/auth/refresh', { method: 'POST', body: {}, retry: 0 })
      if (outcome.status === 'renewed') return true
    } catch {
      // 401 sur rejeu, ou API muette : dans les deux cas la session est perdue.
    }
    witness.value = null
    return false
  }

  async function refreshSession(): Promise<boolean> {
    if (!isConfigured.value || import.meta.server) return false
    pendingRotation ??= rotate().finally(() => {
      pendingRotation = null
    })
    return pendingRotation
  }

  async function request<T>(path: string, options: Record<string, unknown> = {}): Promise<T> {
    try {
      const response = await client<T>(path, options)
      status.reportRecovery()
      return response
    } catch (raw) {
      const error = normalizeApiError(raw)

      if (error instanceof ApiUnreachableError) {
        status.reportOutage(error.reason)
        throw error
      }
      status.reportRecovery()

      // Jeton d'accès périmé : une rotation, un seul rejeu. Un `path` commençant
      // par `/auth/` est exclu — rafraîchir pour rejouer une connexion n'aurait
      // aucun sens, et `/auth/refresh` s'appellerait lui-même.
      const recoverable =
        error instanceof ApiRequestError &&
        (error.code === 'UNAUTHENTICATED' || error.code === 'IDENTITY_SESSION_EXPIRED') &&
        !path.startsWith('/auth/')

      if (recoverable && (await refreshSession())) {
        try {
          const response = await client<T>(path, options)
          status.reportRecovery()
          return response
        } catch (second) {
          throw refus(normalizeApiError(second))
        }
      }

      throw refus(error)
    }
  }

  return { baseURL, isConfigured, client, request, refreshSession }
}

/**
 * Le refus générique devient l'état « accès refusé » de l'écran ; les refus qui
 * NOMMENT leur raison restent des messages.
 *
 * `FORBIDDEN` ne dit rien de plus que « vous n'avez pas les droits » — c'est
 * exactement ce que l'écran d'accès refusé raconte, en mieux. Mais « seul un
 * référent de cette organisation peut effectuer cette action » ou « ce dossier
 * ne vous est pas confié : vous pouvez le lire, pas le noter » disent ce qu'il
 * faut faire ensuite, et un écran plein écran les ferait disparaître.
 */
function refus(error: ApiRequestError | ApiUnreachableError) {
  if (error instanceof ApiRequestError && error.code === 'FORBIDDEN') {
    return new ForbiddenError(error.message)
  }
  return error
}

/**
 * Lecture des données simulées, avec sa latence.
 *
 * L'ATTENTE EST DÉLIBÉRÉE : sans elle, les états de chargement — squelettes,
 * boutons désactivés — ne se voient jamais en développement, et finissent par ne
 * plus être écrits. Elle ne vaut que dans le navigateur : la retarder au rendu
 * serveur ralentirait la première réponse sans que personne ne le voie.
 *
 * L'import est DYNAMIQUE : configurée, l'application n'embarque pas les données
 * simulées dans son paquet.
 */
export async function readMocks<T>(
  fromMocks: (m: typeof import('~/mocks')) => T | Promise<T>,
  latencyMs: number,
): Promise<T> {
  const mocks = await import('~/mocks')
  if (import.meta.client && latencyMs > 0) {
    await new Promise((resolve) => setTimeout(resolve, latencyMs))
  }
  return fromMocks(mocks)
}
