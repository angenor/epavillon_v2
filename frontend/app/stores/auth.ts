import { defineStore } from 'pinia'
import type { Person } from '~/types/identity'
import type { LoginPayload, LoginResult, RegisterPayload } from '~/types/auth'
import type { Uuid } from '~/types/shared'

/**
 * Session de la personne connectée.
 *
 * CE STORE N'EST PAS UN CONTRÔLE DE SÉCURITÉ, et il faut le lire ainsi. Il porte
 * l'état de l'INTERFACE — qui est connecté, ce qu'on affiche, où l'on redirige.
 * L'autorisation réelle appartient à l'API : `identity.has_permission()`, testée
 * par permission et jamais par nom de rôle. Le jour où l'API existe (prompt B1),
 * la session vivra dans un cookie `HttpOnly` que ce fichier ne pourra même pas
 * lire, et c'est très bien.
 *
 * POURQUOI UN COOKIE, ET PAS `localStorage`. Même raison que pour le thème (voir
 * `preferences.ts`) : le rendu serveur doit savoir, dès la première réponse, si
 * quelqu'un est connecté. Sans cela, la page de connexion s'affiche une fraction
 * de seconde à une personne déjà authentifiée, puis se remplace — ce clignotement
 * est exactement ce que le rendu serveur est censé éviter.
 *
 * CE QUE LE COOKIE CONTIENT AUJOURD'HUI : un identifiant de personne, en clair,
 * lisible et falsifiable. C'est acceptable parce que rien de sensible n'est
 * servi — les données sont simulées — et parce que la bascule est déjà écrite :
 * `useApi().auth.session()` prend cet identifiant tant que l'API n'est pas
 * configurée, et l'ignorera ensuite.
 */

/** Nom du cookie de session simulée. Remplacé par le cookie `HttpOnly` de l'API. */
const SESSION_COOKIE = 'epavillon_session'

/** Trente jours avec « rester connecté », la journée sinon. */
const REMEMBERED_MAX_AGE = 60 * 60 * 24 * 30
const SESSION_MAX_AGE = 60 * 60 * 12

export const useAuthStore = defineStore('auth', () => {
  const api = useApi()

  /**
   * Référence de LECTURE du cookie, sans durée : elle sert à savoir qui est
   * connecté et à effacer la session. L'écriture passe par `writeSession()`,
   * qui choisit la durée — deux instances de `useCookie` pour un même nom, l'une
   * pour lire, l'autre pour écrire avec ses options, est la façon prévue par
   * Nuxt de faire varier `maxAge` d'un même cookie.
   */
  const personId = useCookie<Uuid | null>(SESSION_COOKIE, {
    default: () => null,
    sameSite: 'lax',
    path: '/',
  })

  /** Pose le cookie de session avec la durée qu'a choisie la personne. */
  function writeSession(id: Uuid, remember: boolean): void {
    const cookie = useCookie<Uuid | null>(SESSION_COOKIE, {
      default: () => null,
      sameSite: 'lax',
      path: '/',
      maxAge: remember ? REMEMBERED_MAX_AGE : SESSION_MAX_AGE,
    })
    cookie.value = id
    personId.value = id
  }

  const person = ref<Person | null>(null)
  const isLoading = ref(false)
  /** Échec du chargement de la session — l'écran affiche son état d'erreur. */
  const loadError = ref<Error | null>(null)
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
  async function ensureLoaded(): Promise<void> {
    if (isResolved.value || isLoading.value) return
    isLoading.value = true
    loadError.value = null
    try {
      person.value = await api.auth.session(personId.value)
      // Cookie orphelin — la personne n'existe plus : on le retire plutôt que de
      // laisser l'interface hésiter à chaque navigation.
      if (person.value === null) personId.value = null
      isResolved.value = true
    } catch (error) {
      loadError.value = error instanceof Error ? error : new Error(String(error))
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
      writeSession(result.person.id, payload.remember_me)
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
