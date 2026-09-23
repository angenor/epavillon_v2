import { defineStore } from 'pinia'
import type { Person } from '~/types/identity'
import type { LoginPayload, LoginResult, RegisterPayload } from '~/types/auth'
import type { Uuid } from '~/types/shared'
import type { LoadFailure } from '~/utils/api-error'

/**
 * Session de la personne connectée.
 *
 * CE STORE N'EST PAS UN CONTRÔLE DE SÉCURITÉ, et il faut le lire ainsi. Il porte
 * l'état de l'INTERFACE — qui est connecté, ce qu'on affiche, où l'on redirige.
 * L'autorisation réelle appartient à l'API : `identity.has_permission()`, testée
 * par permission et jamais par nom de rôle. Depuis la bascule, la session vit
 * dans deux cookies `HttpOnly` que ce fichier ne peut même pas lire, et c'est
 * très bien.
 *
 * POURQUOI UN COOKIE, ET PAS `localStorage`. Même raison que pour le thème (voir
 * `preferences.ts`) : le rendu serveur doit savoir, dès la première réponse, si
 * quelqu'un est connecté. Sans cela, la page de connexion s'affiche une fraction
 * de seconde à une personne déjà authentifiée, puis se remplace — ce clignotement
 * est exactement ce que le rendu serveur est censé éviter.
 *
 * CE QUE LE COOKIE CONTIENT : un identifiant de personne, en clair, lisible et
 * falsifiable. Il n'autorise RIEN. Sur l'API réelle, la session vit dans deux
 * cookies `HttpOnly` que ce fichier ne peut même pas lire, et `/auth/me`
 * n'accepte aucun identifiant du client — c'est la session qui dit qui parle.
 * Ce cookie-ci ne sert donc plus qu'à deux choses : retrouver la personne dans
 * les données simulées, et TÉMOIGNER qu'une session a existé sur ce navigateur.
 *
 * Ce témoignage n'est pas décoratif. Le jeton d'accès dure un quart d'heure ; le
 * cookie qui permet de le renouveler est limité au chemin `/api/auth` et
 * n'atteint jamais le serveur Nuxt. Une page rendue côté serveur après ce délai
 * s'affiche donc déconnectée alors que la session court toujours. Le témoin dit
 * au navigateur de tenter une rotation — et, pour quelqu'un qui ne s'est jamais
 * connecté, de ne pas la tenter.
 */

/** Trente jours avec « rester connecté », la journée sinon. */
const REMEMBERED_MAX_AGE = 60 * 60 * 24 * 30
const SESSION_MAX_AGE = 60 * 60 * 12
/**
 * Quatre-vingt-dix jours depuis l'application, **et le témoin doit suivre**.
 *
 * Sans cela, le témoin tombe au bout de douze heures alors que la session court
 * encore trois mois : `ensureLoaded()` n'ose plus tenter la rotation, et
 * l'application se croit déconnectée sans l'être. Rien n'échoue, et la personne
 * ressaisit son mot de passe pour rien — au pire moment, en salle.
 */
const APP_MAX_AGE = 60 * 60 * 24 * 90

export const useAuthStore = defineStore('auth', () => {
  const api = useApi()

  /**
   * Référence de LECTURE du cookie, sans durée : elle sert à savoir qui est
   * connecté et à effacer la session. L'écriture passe par `writeSession()`,
   * qui choisit la durée — deux instances de `useCookie` pour un même nom, l'une
   * pour lire, l'autre pour écrire avec ses options, est la façon prévue par
   * Nuxt de faire varier `maxAge` d'un même cookie.
   */
  const personId = useSessionWitness()

  /**
   * La durée du témoin suit celle de la session : c'est l'API qui décide, et le
   * témoin ne fait que ne pas la contredire.
   */
  function witnessMaxAge(payload: LoginPayload): number {
    if (payload.client?.kind === 'app') return APP_MAX_AGE
    return payload.remember_me ? REMEMBERED_MAX_AGE : SESSION_MAX_AGE
  }

  /** Pose le témoin de session avec la durée qu'a choisie la personne. */
  function writeSession(id: Uuid, maxAge: number): void {
    const cookie = useCookie<string | null>(SESSION_WITNESS_COOKIE, {
      default: () => null,
      sameSite: 'lax',
      path: '/',
      maxAge,
    })
    cookie.value = id
    personId.value = id
  }

  const person = ref<Person | null>(null)
  const isLoading = ref(false)
  /** Échec du chargement de la session — l'écran affiche son état d'erreur. */
  const loadError = ref<LoadFailure | null>(null)
  /** La session a-t-elle déjà été résolue une fois ? Évite de la recharger. */
  const isResolved = ref(false)

  const isAuthenticated = computed(() => person.value !== null)

  /**
   * Adresse en attente de vérification, retenue entre l'inscription et l'écran
   * qui annonce l'envoi du lien.
   *
   * PAS DANS L'URL, ET C'EST DÉLIBÉRÉ. `?email=…` mettrait une adresse
   * personnelle dans un lien partageable, dans l'historique du navigateur et
   * dans les journaux du serveur, pour un affichage qui dure trente secondes.
   * Conséquence assumée : un rechargement de la page perd l'adresse, et l'écran
   * de vérification bascule alors sur son état vide, qui explique quoi faire.
   */
  const pendingVerificationEmail = ref<string | null>(null)

  /**
   * Charge la personne connectée, une seule fois. Idempotent : les middlewares,
   * le layout et les pages l'appellent tous sans se coordonner.
   */
  let inFlight: Promise<void> | null = null

  async function ensureLoaded(): Promise<void> {
    if (isResolved.value) return
    // Un second appelant attend le chargement en cours : rendre la main avant la
    // fin lui ferait lire une session non tranchée.
    inFlight ??= load().finally(() => (inFlight = null))
    return inFlight
  }

  async function load(): Promise<void> {
    isLoading.value = true
    loadError.value = null
    try {
      person.value = await api.auth.session(personId.value)

      // Personne inconnue alors qu'un témoin existe : le jeton d'accès a pu
      // expirer sans que la session soit finie. Une rotation, une relecture, et
      // on n'insiste pas. **Une API muette n'est pas une fin de session** : le
      // témoin reste, l'écran dit la panne, et la navigation suivante relit.
      if (person.value === null && personId.value !== null) {
        // Le serveur ne peut pas tourner le jeton — le cookie de rotation ne
        // l'atteint pas : il ne conclut rien, et le navigateur décidera. Conclure
        // ici effaçait le témoin de toute session vieille d'un quart d'heure.
        if (import.meta.server) return
        const issue = await api.rotation()
        if (issue === 'renouvelee') person.value = await api.auth.session(personId.value)
        if (issue === 'injoignable') {
          loadError.value = toLoadFailure(new ApiUnreachableError('network', null))
          return
        }
      }

      // Témoin orphelin — la personne n'existe plus, ou la session est close :
      // on le retire plutôt que de laisser l'interface hésiter à chaque navigation.
      if (person.value === null) personId.value = null
      isResolved.value = true
    } catch (error) {
      loadError.value = toLoadFailure(error)
    } finally {
      isLoading.value = false
    }
  }

  /** Rejoue le chargement après une erreur — bouton « Réessayer » de l'écran. */
  async function retryLoad(): Promise<void> {
    isResolved.value = false
    await ensureLoaded()
  }

  /**
   * Tentative de connexion. Rend l'issue TELLE QUELLE : c'est l'écran qui décide
   * quoi en dire, et le compilateur qui l'oblige à traiter les six branches.
   * Seule `authenticated` ouvre la session.
   */
  async function signIn(payload: LoginPayload): Promise<LoginResult> {
    const result = await api.auth.login(payload)
    if (result.status === 'authenticated') {
      person.value = result.person
      isResolved.value = true
      writeSession(result.person.id, witnessMaxAge(payload))
    }
    return result
  }

  async function signOut(): Promise<void> {
    await api.auth.logout()
    person.value = null
    personId.value = null
    isResolved.value = true
  }

  /**
   * Création de compte. Retient l'adresse pour l'écran suivant et n'ouvre AUCUNE
   * session : tant que l'adresse n'est pas vérifiée, il n'y a pas de connexion.
   */
  async function register(payload: RegisterPayload): Promise<void> {
    const result = await api.auth.register(payload)
    pendingVerificationEmail.value = result.email
  }

  function rememberVerificationTarget(email: string): void {
    pendingVerificationEmail.value = email
  }

  /**
   * Rattachement principal, après une adhésion ACTIVE (prompt A2).
   *
   * CE N'EST PAS L'ÉCRAN QUI DÉCIDE, il ne fait que refléter. En base, la
   * primauté est posée par `org.tg_default_primary_membership` — première
   * adhésion active de la personne — et recopiée dans `people.primary_organization_id`
   * par `org.tg_sync_primary_organization`. On reproduit donc la même règle, et
   * seulement elle : une seconde adhésion ne déplace pas la primauté.
   *
   * Sur données simulées, la mise à jour ne survit pas au rechargement de la
   * page — rien n'est écrit, `session()` relit les mocks. C'est cohérent avec ce
   * que fait déjà l'inscription, et c'est visible plutôt que caché.
   */
  function attachOrganization(organizationId: Uuid): void {
    if (person.value === null || person.value.primary_organization_id !== null) return
    person.value = { ...person.value, primary_organization_id: organizationId }
  }

  return {
    person,
    isAuthenticated,
    isLoading,
    loadError,
    isResolved,
    pendingVerificationEmail,
    ensureLoaded,
    retryLoad,
    signIn,
    signOut,
    register,
    rememberVerificationTarget,
    attachOrganization,
  }
})
