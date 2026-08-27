/**
 * Point d'entrée UNIQUE vers les données.
 *
 * RÈGLE DE PROJET : aucune page n'importe un mock, aucune page n'appelle
 * `$fetch` directement. Tout passe par ici. Chaque méthode déclare AU MÊME
 * ENDROIT sa route d'API et sa source simulée : le jour où l'API répond, il n'y
 * a rien à réécrire dans les écrans, et rien à chercher dans ce fichier.
 *
 * BASCULE. Tant que `NUXT_PUBLIC_API_BASE` est vide, les données viennent de
 * `app/mocks/` ; dès qu'elle est renseignée, les mêmes appels partent vers
 * l'API. L'import des mocks est DYNAMIQUE : configurée, l'application ne les
 * embarque pas dans son paquet.
 *
 * PÉRIMÈTRE D'ADMINISTRATION. Les listes du back-office prennent un `scope`
 * (`identity.administered_events()`) et le respectent, y compris quand
 * l'utilisateur forge une URL. Ce filtre sera doublé côté API — il ne s'agit pas
 * ici d'un contrôle de sécurité mais du comportement attendu de l'écran, qui
 * doit refuser plutôt qu'afficher une liste vide.
 *
 *   { is_global: true,  event_ids: [] }      administrateur de la plateforme
 *   { is_global: false, event_ids: [id…] }   administrateur des éditions listées
 *   { is_global: false, event_ids: [] }      aucun droit → accès refusé
 *
 * LATENCE SIMULÉE. En développement sur mocks, chaque appel attend brièvement :
 * sans cela, les états de chargement — squelettes, désactivation des boutons —
 * ne se voient jamais et finissent par ne plus être écrits.
 *
 * DÉCOUPAGE. Un écran dont les appels dépassent la centaine de lignes sort dans
 * `composables/api/`, monté ici par une fabrique qui reçoit `call` et `send` —
 * c'est le cas de la fiche d'évaluation (`review`), du planificateur
 * (`planner`) et de la gestion des événements (`adminEvents`) — ces deux derniers
 * recevant en plus `assertEventInScope`. Rien ne change pour les
 * pages : elles appellent toujours `useApi()`, et la bascule vers l'API réelle,
 * la latence simulée et l'en-tête `Accept-Language` valent aussi là-bas. Ce
 * découpage suit la règle du projet — par ÉCRAN — et tient ce fichier sous le
 * garde-fou de mille lignes de `CLAUDE.md`.
 */

import type { AdministeredEvents, EffectivePermission, Person } from '~/types/identity'
import type { EventStatus } from '~/types/event/edition'
import type { PublicCall } from '~/types/event/call'
import type {
  LoginPayload,
  LoginResult,
  PasswordResetRequestResult,
  PasswordResetResult,
  RegisterPayload,
  RegisterResult,
  ResendVerificationResult,
  TokenCheckResult,
  VerifyEmailResult,
} from '~/types/auth'
import type { SimilarOrganization } from '~/types/org'
import type {
  CreateOrganizationPayload,
  CreateOrganizationResult,
  EmailDomainMatch,
  JoinOrganizationPayload,
  JoinOrganizationResult,
  OrganizationSearchQuery,
} from '~/types/organization-join'
import type { AdminDashboard } from '~/types/admin-dashboard'
import type { RoleAssignmentView } from '~/types/admin-users'
import type { ResolvedFeatureFlag } from '~/types/platform'
import type {
  DecideMembershipPayload,
  InviteMemberPayload,
  InviteMemberResult,
} from '~/types/organization-workspace'
import type { RegistrationRow } from '~/types/programme/registration'
import type { Uuid } from '~/types/shared'
import { ApiRequestError, ForbiddenError } from '~/utils/api-error'
import { createApiHttp, readMocks } from './api/http'
import { createProposalReviewApi } from './api/proposal-review'
import { createProposalsApi } from './api/proposals'
import { createPlannerApi } from './api/planner'
import { createAdminEventsApi } from './api/admin-events'
import { createAdminOrganizationsApi } from './api/admin-organizations'
import { createInvitationApi } from './api/invitation'
import { createAdminUsersApi } from './api/admin-users'
import { createAdminIncidentsApi } from './api/admin-incidents'
import { createOrganizationWorkspaceApi } from './api/organization-workspace'
import { createHomeApi } from './api/home'
import { createAdminShowcaseApi } from './api/admin-showcase'
import { createMediaApi } from './api/media'

// `ForbiddenError` vit désormais dans `utils/api-error.ts`, avec les deux autres
// erreurs de la couche d'accès : le client HTTP doit pouvoir la lever sur un
// `FORBIDDEN` de l'API, et l'importer d'ici lui ferait remonter tout ce fichier.
// Elle y est auto-importée par Nuxt, comme tout `app/utils/` — la réexporter ici
// en ferait un second nom pour la même classe, que l'auto-import signale.

/** Périmètre sans aucun droit : la valeur sûre par défaut. */
export const NO_ADMIN_SCOPE: AdministeredEvents = { is_global: false, event_ids: [] }

const MOCK_LATENCY_MS = 120

export function useApi() {
  const http = createApiHttp()
  const { baseURL, isConfigured, client, request, refreshSession } = http
  const mockData = useMockData()

  type Mocks = typeof import('~/mocks')

  /**
   * Un appel de données : sa route d'API et, tant qu'elle n'existe pas, la
   * lecture équivalente dans les mocks.
   */
  async function call<T>(path: string, fromMocks: (m: Mocks) => T | Promise<T>, query?: Record<string, unknown>): Promise<T> {
    if (isConfigured.value) {
      return request<T>(path, { query })
    }
    return readMocks(fromMocks, MOCK_LATENCY_MS)
  }

  /**
   * Une lecture dont « rien » est une réponse acceptable — le 404 devient `null`.
   *
   * POURQUOI DEUX FONCTIONS PLUTÔT QU'UNE OPTION. Beaucoup de lectures rendent
   * une LISTE : leur donner `null` sur un 404 ferait planter le premier `.map()`
   * de l'écran, loin de l'appel, avec un message qui ne dit rien. La distinction
   * est donc portée par le nom, et se voit à l'endroit où l'on choisit.
   *
   * CE QUE L'API DIT DE SES 404 : « inexistant **ou hors périmètre** —
   * indiscernables ». C'est délibéré, et l'écran doit s'en tenir là : afficher
   * « introuvable » plutôt que de laisser deviner qu'une édition existe ailleurs.
   */
  async function callOrNull<T>(
    path: string,
    fromMocks: (m: Mocks) => T | null | Promise<T | null>,
    query?: Record<string, unknown>,
  ): Promise<T | null> {
    if (!isConfigured.value) return readMocks(fromMocks, MOCK_LATENCY_MS)
    try {
      return await request<T>(path, { query })
    } catch (error) {
      if (error instanceof ApiRequestError && error.code === 'NOT_FOUND') return null
      throw error
    }
  }

  /**
   * Une ÉCRITURE : même principe que `call`, mais avec un verbe et un corps.
   *
   * Déclarée à part plutôt qu'en option de `call` pour une raison de lecture :
   * on voit d'un coup d'œil, dans ce fichier, ce qui interroge la plateforme et
   * ce qui la modifie. Les données simulées étant en lecture seule, la « lecture
   * simulée » d'une écriture calcule la RÉPONSE que l'API rendra — c'est
   * exactement ce dont l'écran a besoin pour se comporter juste.
   */
  async function send<T>(
    path: string,
    body: object,
    fromMocks: (m: Mocks) => T | Promise<T>,
    method: 'POST' | 'PUT' | 'PATCH' | 'DELETE' = 'POST',
  ): Promise<T> {
    if (isConfigured.value) {
      // `object` et non `Record<string, unknown>` : une INTERFACE n'a pas de
      // signature d'index implicite, et `LoginPayload` serait refusé — le même
      // écueil que la contrainte générique de `UiTable`. La conversion est sûre,
      // un corps de requête étant toujours sérialisé en JSON.
      // `retry: 0`, et ce n'est pas négociable : une écriture rejouée après une
      // coupure de passerelle peut créer deux lieux, déposer deux fois le même
      // dossier, ou rendre un échec sur une réinitialisation qui a abouti. Une
      // lecture se rejoue sans dommage, une écriture non.
      return request<T>(path, { method, body: body as Record<string, unknown>, retry: 0 })
    }
    // Une écriture attend plus longtemps qu'une lecture : c'est pendant ce
    // temps-là que le bouton doit se désactiver et afficher son témoin. Sans
    // cette latence, l'état de chargement d'un formulaire ne se voit jamais et
    // finit par ne plus être écrit.
    return readMocks(fromMocks, MOCK_LATENCY_MS * 3)
  }

  /**
   * UNE ÉCRITURE QUI PORTE UN FICHIER — la seule route de l'API qui ne parle pas
   * JSON, et la seule raison pour laquelle cette primitive existe.
   *
   * `send` sérialise son corps en JSON ; un corps composite s'y perdrait. Elle
   * est déclarée à part plutôt qu'en option de `send` pour la même raison que
   * `send` l'est de `call` : on voit d'un coup d'œil ce qui traverse le réseau
   * en octets.
   *
   * L'ORDRE DES CHAMPS COMPTE, et il appartient à l'appelant : la route lit le
   * corps dans l'ordre où il a été écrit, et n'accepte le fichier qu'après ses
   * métadonnées — c'est ce qui lui permet de refuser un type, un poids ou un
   * droit sans avoir lu un octet.
   *
   * `retry: 0`, comme toute écriture : un dépôt rejoué après une coupure de
   * passerelle écrit deux fois le même objet.
   */
  async function sendForm<T>(path: string, form: FormData, fromMocks: (m: Mocks) => T | Promise<T>): Promise<T> {
    if (isConfigured.value) {
      // `body` reste un `FormData` : le client HTTP laisse alors le navigateur
      // poser `Content-Type` avec sa frontière. L'écrire à la main la ferait
      // manquer, et le corps deviendrait illisible pour l'API.
      return request<T>(path, { method: 'POST', body: form, retry: 0 })
    }
    return readMocks(fromMocks, MOCK_LATENCY_MS * 3)
  }

  /**
   * Un écran dont l'API N'EXISTE PAS ENCORE.
   *
   * Trois écrans du jalon sont dans ce cas : les messages d'incident, l'accueil
   * public et sa vitrine administrable. Leurs données vivent bien en base — dans
   * les schémas `live` et `content` — mais aucun crate Rust ne les sert à ce
   * jour. Les faire appeler leur route produirait un 404 : un écran livré et
   * vérifié se mettrait à afficher une panne le jour où l'API est branchée.
   *
   * Ils continuent donc de lire les données simulées, MÊME quand l'API est
   * configurée, et ils le DISENT — `usesMockData` allume un bandeau sur l'écran
   * concerné. Le faux-semblant serait de servir ces données sans le signaler ;
   * l'écran cassé serait de les réclamer à une API qui ne les a pas.
   *
   * Le chemin est passé quand même : il documente la route attendue, et
   * `scripts/check-api-contract.mjs` s'en sert pour lister la dette au lieu de la
   * compter comme une faute.
   */
  async function pending<T>(
    path: string,
    fromMocks: (m: Mocks) => T | Promise<T>,
    kind: 'read' | 'write' = 'read',
  ): Promise<T> {
    if (isConfigured.value) mockData.mark(path, kind)
    // Une écriture garde la latence d'une écriture, même simulée : c'est ce qui
    // fait voir le témoin de soumission d'un formulaire. Sans ce paramètre, les
    // sept écritures des incidents et de la vitrine s'exécutaient trois fois
    // plus vite que partout ailleurs, et leur bouton ne montrait rien.
    return readMocks(fromMocks, kind === 'write' ? MOCK_LATENCY_MS * 3 : MOCK_LATENCY_MS)
  }

  /** Refuse l'accès à une édition hors périmètre, plutôt que de rendre une liste vide. */
  function assertEventInScope(eventId: Uuid, scope: AdministeredEvents): void {
    if (!scope.is_global && !scope.event_ids.includes(eventId)) {
      throw new ForbiddenError()
    }
  }

  /**
   * Ce que reçoit une fabrique d'écran. Déclaré comme VARIABLE et non écrit à
   * chaque appel : passé en littéral, TypeScript refuserait les propriétés
   * qu'une fabrique donnée ne déclare pas, et il faudrait tenir onze listes à
   * jour au lieu d'une.
   */
  const deps = { call, callOrNull, send, sendForm, pending, assertEventInScope }

  return {
    baseURL,
    /** L'API est-elle configurée ? Faux tant que la variable n'est pas posée. */
    isConfigured,
    /** Client HTTP préconfiguré. Réservé aux cas non couverts ci-dessous. */
    client,
    /** Tente une rotation du jeton de session. Utilisé par le store de session. */
    refreshSession,
    ForbiddenError,
    assertEventInScope,
    /** Une lecture dont « rien » est une réponse acceptable — 404 devient `null`. */
    callOrNull,
    /** Un écran dont l'API n'existe pas encore : données d'exemple, et le dire. */
    pending,

    // -----------------------------------------------------------------------
    // Authentification (A1)
    //
    // Les cinq écrans d'authentification passent par ici, et par rien d'autre.
    // Deux règles s'y jouent, l'une et l'autre invisibles depuis les pages :
    //
    //  · DISCRÉTION — `register` et `requestPasswordReset` rendent TOUJOURS la
    //    même réponse, adresse connue ou non. Rien dans le contrat ne permet
    //    d'écrire un écran bavard, même par inadvertance.
    //  · SESSION — l'API pose deux cookies `HttpOnly` que le navigateur renvoie
    //    seuls (`credentials: 'include'`, voir `api/http.ts`). `GET /auth/me`
    //    N'ACCEPTE AUCUN IDENTIFIANT : c'est la session qui dit qui parle. Le
    //    paramètre de `session()` ne sert donc qu'aux données simulées, qui
    //    n'ont pas de session à consulter — il n'est jamais envoyé.
    // -----------------------------------------------------------------------
    auth: {
      login: (payload: LoginPayload): Promise<LoginResult> =>
        send('/auth/login', payload, (m) => m.authenticate(payload)),

      logout: (): Promise<{ status: 'signed_out' }> =>
        send('/auth/logout', {}, () => ({ status: 'signed_out' as const })),

      /**
       * Personne connectée, ou `null` si la session n'existe plus.
       *
       * `GET /auth/me` ne rend JAMAIS 401 — le site l'appelle déconnecté à
       * chaque navigation, et un statut d'erreur y ferait afficher un écran en
       * panne au lieu d'un état déconnecté. L'identifiant reçu ici ne part pas
       * dans la requête : il ne sert qu'à retrouver la personne dans les mocks.
       */
      session: (personId: Uuid | null): Promise<Person | null> =>
        call('/auth/me', (m) =>
          personId === null ? null : (m.people.find((p) => p.id === personId) ?? null),
        ),

      register: (payload: RegisterPayload): Promise<RegisterResult> =>
        send('/auth/register', payload, (m) => m.registerPerson(payload)),

      /** Vérification de l'adresse depuis le lien reçu par courriel. */
      verifyEmail: (token: string): Promise<VerifyEmailResult> =>
        send('/auth/verify-email', { token }, (m) => m.verifyEmailToken(token)),

      /** Renvoi du lien de vérification. Réponse invariable. */
      resendVerification: (email: string): Promise<ResendVerificationResult> =>
        send('/auth/verify-email/resend', { email }, () => ({ status: 'sent' as const })),

      /** Demande de réinitialisation. Réponse invariable, compte existant ou non. */
      requestPasswordReset: (email: string): Promise<PasswordResetRequestResult> =>
        send('/auth/password-reset', { email }, () => ({ status: 'sent' as const })),

      /** Contrôle du jeton AVANT d'afficher le formulaire de nouveau mot de passe. */
      checkPasswordResetToken: (token: string): Promise<TokenCheckResult> =>
        call('/auth/password-reset/check', (m) => m.checkPasswordResetToken(token), { token }),

      resetPassword: (token: string, password: string): Promise<PasswordResetResult> =>
        send('/auth/password-reset/confirm', { token, password }, (m) => m.resetPassword(token)),
    },

    // -----------------------------------------------------------------------
    // Plateforme (A14)
    //
    // Les drapeaux sont lus par le ROUTAGE, pas par un écran : le middleware
    // global `feature-flag` sert la page « En cours de maintenance » à la place
    // d'un module éteint. C'est pour cela que l'appel rend la TABLE ENTIÈRE et
    // non un booléen par clé — une navigation ne peut pas déclencher un appel
    // par module traversé, et le store qui l'enveloppe ne charge qu'une fois.
    // -----------------------------------------------------------------------
    platform: {
      /**
       * Chaque drapeau et son verdict POUR L'APPELANT, toutes clés confondues.
       *
       * La table entière, et non un booléen par clé : une navigation ne peut pas
       * déclencher un appel par module traversé. Mais chaque ligne est déjà
       * TRANCHÉE — le déploiement progressif se calcule en base, jamais ici.
       */
      featureFlags: (): Promise<ResolvedFeatureFlag[]> =>
        call('/platform/feature-flags', (m) =>
          m.featureFlags.map((f) => ({ key: f.key, is_enabled: m.isFeatureEnabled(f.key) })),
        ),
    },

    home: createHomeApi(deps),
    adminShowcase: createAdminShowcaseApi(deps),

    // -----------------------------------------------------------------------
    // Média (B3 — dépôt et rattachement)
    //
    // Le module sert le dépôt depuis B3 ; jusqu'au 26/08 aucun écran ne
    // l'appelait, et les trois déclinaisons d'une édition se rattachaient par
    // un identifiant d'objet saisi à la main — c'est-à-dire par personne.
    // -----------------------------------------------------------------------
    media: createMediaApi(deps),

    // -----------------------------------------------------------------------
    // Référentiel
    // -----------------------------------------------------------------------
    reference: {
      countries: () => call('/reference/countries', (m) => m.countries),
      locales: () => call('/reference/locales', (m) => m.locales),
      /** Termes d'une taxonomie, actifs, dans leur ordre d'affichage. */
      terms: (taxonomy: string) =>
        call(
          `/reference/taxonomies/${taxonomy}/terms`,
          (m) =>
            m.taxonomyTerms
              .filter((t) => t.taxonomy_code === taxonomy && t.is_active)
              .sort((a, b) => a.sort_order - b.sort_order),
        ),
    },

    // -----------------------------------------------------------------------
    // Organisations
    // -----------------------------------------------------------------------
    organizations: {
      // Les deux lectures passent par les fonctions des mocks, et non par le
      // tableau : c'est ce qui rend visible une fiche créée pendant la session
      // de démonstration (voir l'en-tête de `mocks/organization-search.ts`).
      list: () => call('/organizations', (m) => m.organizationsWithSession()),
      byId: (id: Uuid) => call(`/organizations/${id}`, (m) => m.organizationById(id)),

      /**
       * LA recherche d'organisation de la plateforme — `org.find_similar_organizations()`.
       *
       * Écrite pour le rattachement (A2), elle sert aussi le formulaire de
       * soumission (A4, choix des co-organisateurs) et la fusion des doublons
       * (A11). UNE SEULE recherche pour les trois : deux implémentations
       * divergentes rapprocheraient les fiches dans un écran et pas dans l'autre,
       * ce qui est précisément la façon dont naissent les doublons.
       *
       * Elle interroge TOUTES les dénominations — nom légal, sigle, traduction,
       * ancien nom, faute de frappe connue — plus les domaines. « IFDD », « IEPF »
       * et « Institut de la Francophonie… » ramènent la même fiche.
       */
      similar: (query: OrganizationSearchQuery): Promise<SimilarOrganization[]> =>
        call('/organizations/similar', (m) => m.findSimilarOrganizations(query), {
          name: query.name,
          country_id: query.country_id ?? undefined,
          email: query.email ?? undefined,
          website: query.website ?? undefined,
          limit: query.limit,
        }),

      /**
       * Ce que le domaine d'une adresse révèle — `organization_domains`. Rend
       * `null` pour une messagerie grand public : deux ONG ne sont pas la même
       * parce que leurs référents utilisent Gmail (`org.public_email_domains`).
       *
       * L'ADRESSE N'EST PAS ENVOYÉE : l'API lit celle de la session, et rien
       * d'autre. Le paramètre ne sert qu'aux données simulées — le transmettre
       * laisserait croire qu'on peut interroger le domaine de quelqu'un d'autre.
       */
      byEmailDomain: (email: string): Promise<EmailDomainMatch | null> =>
        call('/organizations/by-email-domain', (m) => m.organizationForEmail(email)),

      /**
       * Demande de rattachement. L'issue dépend du DOMAINE de l'adresse, pas de
       * la volonté de l'utilisateur : `joined` si le domaine est vérifié et
       * marqué `auto_join`, `pending` partout ailleurs — un référent doit alors
       * accepter.
       *
       * `personId` disparaîtra au prompt B7 : l'API lit sa propre session.
       */
      join: (personId: Uuid, payload: JoinOrganizationPayload): Promise<JoinOrganizationResult> =>
        send(`/organizations/${payload.organization_id}/members`, payload, (m) =>
          m.joinOrganization(personId, payload),
        ),

      /** Création d'une fiche, en `candidate` : l'IFDD la regardera. */
      create: (personId: Uuid, payload: CreateOrganizationPayload): Promise<CreateOrganizationResult> =>
        send('/organizations', payload, (m) => m.createOrganization(personId, payload)),

      /** Adhésions vivantes d'une personne — actives ou en attente d'un référent. */
      membershipsOf: (personId: Uuid) =>
        call(`/people/${personId}/memberships`, (m) => m.membershipsOfPerson(personId)),

      /**
       * INVITATION D'UN MEMBRE PAR SON ADRESSE (A5).
       *
       * Trois écritures en base derrière un seul appel : la personne, créée si
       * l'adresse est inconnue ; l'adhésion, née `pending` avec `invited_by` et
       * `invited_at` ; le jeton à usage unique qui part par courriel. La
       * DIRECTION portée par l'adhésion est ce qui distingue cette invitation
       * d'une demande spontanée — sans elle, un référent approuverait sa propre
       * invitation et donnerait une adhésion active à qui n'a rien accepté.
       */
      invite: (personId: Uuid, payload: InviteMemberPayload): Promise<InviteMemberResult> =>
        send(`/organizations/${payload.organization_id}/invitations`, payload, (m) =>
          m.inviteMember(personId, payload),
        ),

      /**
       * DÉCISION D'UN RÉFÉRENT sur une DEMANDE d'adhésion — jamais sur une
       * invitation, qui attend la personne et non l'organisation. Un refus
       * révoque l'adhésion au lieu de l'effacer : la v1 supprimait la ligne, et
       * plus personne ne pouvait distinguer une demande refusée d'une demande
       * jamais faite.
       */
      decideMembership: (personId: Uuid, payload: DecideMembershipPayload) =>
        send(
          `/memberships/${payload.membership_id}/decision`,
          payload,
          (m) => m.decideMembership(personId, payload),
          'PUT',
        ),

      // Les dénominations, les domaines, les membres et les doublons ne se
      // lisent PAS ici. Ce sont des lectures de back-office, servies sous
      // `/admin/organizations/…` et montées par `adminOrganizations` : quatre
      // méthodes de plus ici visaient des chemins que l'API n'a jamais exposés,
      // restes d'avant le découpage de l'écran A11.
    },

    // -----------------------------------------------------------------------
    // Personnes et habilitations
    // -----------------------------------------------------------------------
    identity: {
      people: () => call('/people', (m) => m.people),
      byId: (id: Uuid) => call(`/people/${id}`, (m) => m.people.find((p) => p.id === id) ?? null),
      /**
       * Les attributions EN COURS d'une personne, **avec leur portée résolue**.
       *
       * `RoleAssignmentView` et non `RoleAssignment` : une pastille ne porte
       * jamais un rôle sans sa portée, et la résoudre côté site demanderait une
       * lecture par attribution. L'API compose déjà le libellé du rôle, la cible
       * de la portée et qui l'a confiée.
       */
      roleAssignments: (personId: Uuid): Promise<RoleAssignmentView[]> =>
        call(`/people/${personId}/roles`, (m) => m.activeAssignmentsOf(personId)),

      /**
       * `identity.effective_permissions()` — CE QUE CETTE PERSONNE PEUT FAIRE, et
       * sur quelle portée.
       *
       * La règle du projet est de tester une PERMISSION, jamais un nom de rôle.
       * Sans cet appel, un écran ne pouvait que demander « administre-t-elle
       * quelque chose ? », c'est-à-dire tester un rôle sans le dire. Les portées
       * comptent autant que les codes : une permission accordée sur la COP31 ne
       * vaut pas sur la COP30 — voir `utils/permissions.ts`.
       *
       * Ce que le front en fait est de l'AFFICHAGE : montrer ou masquer une
       * action. Le refus, lui, appartient à l'API.
       */
      permissions: (personId: Uuid): Promise<EffectivePermission[]> =>
        call(`/people/${personId}/permissions`, (m) => m.effectivePermissions(personId)),

      /**
       * `identity.administered_events()` : périmètre d'administration d'une
       * personne. Renvoie TOUJOURS une valeur pleine — jamais `null` — de sorte
       * que « aucun droit » et « administrateur d'une édition » ne se confondent.
       *
       * LE CRITÈRE EST UNE PERMISSION, PAS UNE LISTE DE RÔLES, et c'est
       * exactement ce que fait la fonction en base : elle joint
       * `role_permissions` et retient les attributions qui portent
       * `programme.proposal.read_all`. Cette lecture réimplémentait le périmètre
       * avec `['super_admin', 'admin', 'programmer']` codés en dur — un test de
       * rôle déguisé, contraire à la règle du projet, et surtout FAUX : le rôle
       * `reviewer` détient `programme.proposal.read_all` sans figurer dans cette
       * liste. Un membre du comité se voyait donc refuser l'accès à la fiche
       * d'évaluation des dossiers qu'on lui avait confiés (constaté au prompt
       * A8). Ajouter un rôle au catalogue suffit désormais ; ce fichier ne bouge
       * plus.
       */
      administeredEvents: (personId: Uuid): Promise<AdministeredEvents> =>
        call(`/people/${personId}/administered-events`, (m) => {
          const scoped = m
            .effectivePermissions(personId)
            .filter((entry) => entry.permission_code === 'programme.proposal.read_all')

          return {
            is_global: scoped.some((entry) => entry.scope_type === 'global'),
            event_ids: [
              ...new Set(
                scoped
                  .filter((entry) => entry.scope_type === 'event' && entry.scope_id !== null)
                  .map((entry) => entry.scope_id as Uuid),
              ),
            ],
          }
        }),
    },

    // -----------------------------------------------------------------------
    // Événements
    // -----------------------------------------------------------------------
    events: {
      list: (scope: AdministeredEvents = NO_ADMIN_SCOPE) =>
        call('/events', (m) =>
          m.events.filter((e) => scope.is_global || scope.event_ids.includes(e.id)),
        ),
      /** Page publique : aucun filtrage de périmètre, l'édition est publique. */
      bySlug: (slug: string) => call(`/events/${slug}`, (m) => m.events.find((e) => e.slug === slug) ?? null),

      /**
       * LES ÉDITIONS PUBLIQUES, toutes séries confondues — ce qui alimente le
       * sélecteur d'année de la page publique (A3).
       *
       * Le critère est celui du modèle et non une convention d'écran : une
       * édition est publique dès lors qu'elle n'est ni un brouillon ni annulée.
       * Une édition annoncée dont le programme n'est pas encore publié en fait
       * donc partie — sa page existe, elle annonce ses échéances, et c'est
       * précisément là qu'on dépose un dossier.
       */
      publicList: () =>
        call('/events/public', (m) => {
          // Liste déclarée à part : écrite en ligne, elle serait comparée aux
          // seuls statuts que portent les données simulées, et le compilateur
          // refuserait un statut absent du jeu d'essai.
          const hidden: EventStatus[] = ['draft', 'cancelled']
          return m.events
            .filter((e) => !hidden.includes(e.status))
            .sort((a, b) => b.starts_at.localeCompare(a.starts_at))
        }),

      /**
       * Les séries. La page publique s'en sert pour distinguer ce qui relève
       * d'une COP de ce qui n'en relève pas (`webinar_series`, `standalone`) :
       * la distinction est portée par `event.event_series.kind`, jamais par une
       * liste de slugs recopiée dans un composant.
       */
      series: () => call('/event-series', (m) => m.eventSeries),

      /**
       * LES TROIS DÉCLINAISONS DE L'ÉDITION — `banner` 32:9, `cover` 16:9,
       * `thumbnail` 1:1, chacune résolue par `media.attached_image()`.
       *
       * TROIS ROLES, UN SEUL APPEL : les demander séparément ferait trois
       * allers-retours pour un écran qui, de toute façon, choisit celle qui va
       * à sa largeur. Elles arrivent donc ensemble et l'écran arbitre.
       *
       * Un appel à part reste un appel de trop : `event.events` ne porte pas ses
       * images, le rattachement média étant polymorphe. La couverture d'une
       * séance, elle, est résolue EN BASE par `v_public_schedule`. Obligation
       * inscrite au prompt B3 : la réponse de `GET /events/:slug` embarque ses
       * images résolues, et cet appel disparaît.
       */
      images: (eventId: Uuid) =>
        call(`/events/${eventId}/images`, (m) => ({
          banner: m.attachedImage('event', 'events', eventId, 'banner'),
          cover: m.attachedImage('event', 'events', eventId, 'cover'),
          thumbnail: m.attachedImage('event', 'events', eventId, 'thumbnail'),
        })),
      days: (eventId: Uuid) =>
        call(`/events/${eventId}/days`, (m) => m.eventDays.filter((d) => d.event_id === eventId)),
      tracks: (eventId: Uuid) =>
        call(`/events/${eventId}/tracks`, (m) => m.programmeTracks.filter((t) => t.event_id === eventId)),
      rooms: (eventId: Uuid) =>
        call(`/events/${eventId}/rooms`, (m) => {
          const venueIds = new Set(m.venues.filter((v) => v.event_id === eventId).map((v) => v.id))
          return m.rooms.filter((r) => venueIds.has(r.venue_id))
        }),
      venues: (eventId: Uuid) =>
        call(`/events/${eventId}/venues`, (m) => m.venues.filter((v) => v.event_id === eventId)),
      broadcastChannels: (eventId: Uuid) =>
        call(`/events/${eventId}/channels`, (m) =>
          m.broadcastChannels.filter((c) => c.event_id === eventId || c.event_id === null),
        ),
      /** Zéro ou UN appel par édition, jamais plusieurs. */
      /**
       * L'appel de l'édition, **avec sa grille d'évaluation**. Zéro ou un,
       * jamais un tableau — `ux_calls_one_per_event` tient la cardinalité.
       */
      call: (eventId: Uuid): Promise<PublicCall | null> =>
        call(`/events/${eventId}/call`, (m) => {
          const found = m.callsForProposals.find((c) => c.event_id === eventId)
          if (!found) return null
          return { ...found, criteria: m.reviewCriteria.filter((c) => c.call_id === found.id) }
        }),
    },

    // La grille d'évaluation d'un appel ne se lit plus à part : elle arrive
    // avec l'appel lui-même (`events.call`), et la page publique d'une édition
    // y a gagné une vague d'appels en moins. La COMPOSITION du comité, elle,
    // n'est pas publique — le back-office la reçoit dans le détail de l'édition.

    // -----------------------------------------------------------------------
    // Propositions (A4 · A7) — `api/proposals.ts`
    // -----------------------------------------------------------------------
    proposals: createProposalsApi(deps),

    // -----------------------------------------------------------------------
    // Évaluation
    //
    // Il n'y a PLUS de lecture table par table ici. Les revues d'un dossier, la
    // revue en cours de la personne, ses notes et les affectations du comité
    // arrivent toutes dans `ReviewDeskScreen` — voile de l'évaluation en aveugle
    // compris, ce qu'une lecture directe des tables ne saurait pas appliquer.
    // Cinq méthodes vivaient ici et visaient des chemins que l'API ne sert pas.
    // -----------------------------------------------------------------------

    // -----------------------------------------------------------------------
    // Fiche d'évaluation (A8)
    //
    // Les cinq appels de l'écran où le comité décide — la composition du
    // dossier, la notation, le déport, les échanges et la décision. Ils vivent
    // dans `composables/api/proposal-review.ts` et sont montés ici : la page
    // appelle `api.review.desk(…)`, elle n'importe rien d'autre.
    //
    // À DISTINGUER DE `reviews` CI-DESSUS, qui lit les tables une par une
    // (revues d'un dossier, notes d'une revue, charge d'un membre). `review`
    // compose l'ÉCRAN, voile de l'évaluation en aveugle compris.
    // -----------------------------------------------------------------------
    review: createProposalReviewApi(deps),

    // -----------------------------------------------------------------------
    // Sessions
    // -----------------------------------------------------------------------
    sessions: {
      /** Programmation PUBLIQUE : séances publiées seulement. */
      schedule: (eventId: Uuid) =>
        call('/schedule', (m) => m.publicSchedule().filter((row) => row.event_id === eventId), {
          event_id: eventId,
        }),
      /** Vue du planificateur (A9) : publiées ou non, filtrée par périmètre. */
      planner: (eventId: Uuid, scope: AdministeredEvents) => {
        assertEventInScope(eventId, scope)
        return call('/sessions', (m) => m.allSessions.filter((s) => s.event_id === eventId), {
          event_id: eventId,
        })
      },
      /**
       * Détail public d'une séance. L'API rend `{ session, speakers,
       * organizations }` et un 404 quand la séance n'est pas publiée — pas une
       * séance nue, et jamais `null`. Aucun écran ne l'appelle encore : la page
       * publique d'une séance n'est pas au jalon, et le type de son écran se
       * déclarera avec elle.
       */
      speakers: (sessionId: Uuid) =>
        call(`/sessions/${sessionId}/speakers`, (m) => m.sessionSpeakers.filter((s) => s.session_id === sessionId)),
      organizations: (sessionId: Uuid) =>
        call(`/sessions/${sessionId}/organizations`, (m) =>
          m.sessionOrganizations.filter((s) => s.session_id === sessionId),
        ),
      tracks: (sessionId: Uuid) =>
        call(`/sessions/${sessionId}/tracks`, (m) => m.sessionTracks.filter((s) => s.session_id === sessionId)),

      /** Chevauchements SIGNALÉS, jamais bloqués (A9). */
      conflicts: (eventId: Uuid, scope: AdministeredEvents) => {
        assertEventInScope(eventId, scope)
        return call('/sessions/conflicts', (m) => m.detectConflicts(eventId), { event_id: eventId })
      },

      // Le contrôle préalable à la publication n'est PAS ici : il vit dans
      // `planner.readiness`, sur `/admin/planner/readiness`. Deux chemins pour
      // la même lecture, c'est deux réponses qui divergent le jour où l'un des
      // deux évolue — et c'est l'écran A9 qui appelle le bon.
    },

    // -----------------------------------------------------------------------
    // Planificateur de créneaux (A9)
    //
    // Sorti dans `composables/api/planner.ts` — sept appels, dont quatre
    // écritures. AUCUNE NE PEUT ÊTRE REFUSÉE POUR CHEVAUCHEMENT : le modèle ne
    // pose aucune contrainte d'exclusion sur les créneaux, l'équipe arbitre en
    // passant par des états incohérents, et le seul garde-fou dur est la
    // publication du programme.
    // -----------------------------------------------------------------------
    planner: createPlannerApi(deps),

    // -----------------------------------------------------------------------
    // Gestion des événements (A10)
    //
    // Sorti dans `composables/api/admin-events.ts` — la liste des éditions, le
    // formulaire d'une édition et les six onglets, avec leurs quinze écritures.
    //
    // À DISTINGUER DE `events` PLUS HAUT, qui porte les lectures PUBLIQUES d'une
    // édition (page publique, sélecteur d'année, bannière) et ne prend aucun
    // périmètre — une édition annoncée est publique. Ici, tout prend le périmètre
    // d'administration et refuse une édition qui n'y est pas.
    //
    // ET CONTRAIREMENT AU PLANIFICATEUR, ces écritures REFUSENT : les contraintes
    // de `060_events.sql` sont des invariants de données, pas des arbitrages. Un
    // slug en double, une clôture avant l'ouverture, un second appel sur une même
    // édition sont refusés en base et le sont ici — rien à voir avec un
    // chevauchement de créneaux, qui reste toujours écrivable.
    // -----------------------------------------------------------------------
    adminEvents: createAdminEventsApi(deps),

    // -----------------------------------------------------------------------
    // Organisations et fusion des doublons (A11)
    //
    // Sorti dans `composables/api/admin-organizations.ts` — la liste et sa fiche
    // de performance, la file des doublons présumés, l'aperçu et l'exécution
    // d'une fusion, la fiche d'une organisation et ses trois écritures.
    //
    // À DISTINGUER D'`organizations` PLUS HAUT, qui porte la recherche ouverte à
    // toute personne connectée — le rattachement (A2), le choix des
    // co-organisateurs (A4). Ici, tout appartient au back-office.
    //
    // AUCUN `assertEventInScope` : une organisation n'appartient à aucune
    // édition. La règle métier n° 8 s'y applique par l'autre bout — la liste est
    // FILTRÉE sur les éditions administrées (`list(scope)`), et la fusion exige
    // `org.organization.merge` sur la portée GLOBALE, ce que l'écran vérifie par
    // permission. Il n'existe pas de fusion limitée à une COP : elle déplace des
    // rattachements dans toutes les éditions à la fois.
    // -----------------------------------------------------------------------
    adminOrganizations: createAdminOrganizationsApi(deps),

    // -----------------------------------------------------------------------
    // Acceptation d'une invitation (B7)
    //
    // Sorti dans `composables/api/invitation.ts`. Séparé d'`organizations` parce
    // qu'il n'en partage pas la règle : c'est le SEUL appel de ce fichier qui
    // n'exige aucune session — le jeton du lien reçu par courriel est la preuve
    // d'adresse, et la personne invitée n'a le plus souvent pas encore de compte.
    // -----------------------------------------------------------------------
    invitation: createInvitationApi(deps),

    // -----------------------------------------------------------------------
    // Utilisateurs et rôles (A12)
    //
    // Sorti dans `composables/api/admin-users.ts` — la liste et ses facettes, la
    // fiche d'une personne, ce que le panneau d'attribution a le droit d'offrir,
    // les permissions effectives, et les quatre écritures : attribuer, retirer,
    // changer un statut, traiter une demande RGPD.
    //
    // À DISTINGUER D'`identity` PLUS HAUT, qui porte les lectures ÉLÉMENTAIRES
    // dont TOUT le back-office dépend — la personne connectée, ses permissions
    // effectives, son périmètre d'administration. Ici, ce sont des compositions
    // d'écran, et elles écrivent.
    //
    // AUCUN `assertEventInScope` : une personne n'appartient à aucune édition.
    // La liste est filtrée sur les éditions administrées, et chaque écriture
    // exige `identity.role.assign` SUR LA PORTÉE VISÉE — attribuer un rôle
    // global demande la permission globale, ce qu'un compte détaché sur la COP31
    // n'a pas. La file RGPD, elle, ne se filtre pas : une demande d'effacement
    // porte sur la plateforme entière.
    // -----------------------------------------------------------------------
    adminUsers: createAdminUsersApi(deps),

    // -----------------------------------------------------------------------
    // Messages d'incident (A13)
    //
    // LE PÉRIMÈTRE EST VÉRIFIÉ AVANT L'APPEL, comme pour le tableau de bord et
    // le planificateur : un incident se publie SUR une édition, et une édition
    // hors périmètre refuse l'accès plutôt que de rendre une liste vide.
    //
    // Les quatre écritures reçoivent les permissions de l'acteur : elles
    // rejouent `has_permission(acteur, 'live.incident.publish', 'event', …)`
    // tant que l'API n'existe pas. Ce paramètre disparaît au prompt B6.
    // -----------------------------------------------------------------------
    adminIncidents: createAdminIncidentsApi(deps),

    // -----------------------------------------------------------------------
    // Espace organisation (A5)
    //
    // TROIS LECTURES ET TROIS ÉCRITURES, et la ligne de partage est toujours la
    // même : l'organisation voit CE QU'ELLE A DÉPOSÉ et ce qu'on attend d'elle,
    // jamais ce que le comité s'écrit ni qui s'est inscrit à ses séances.
    //
    // AUCUNE VUE DU MODÈLE NE RÉPOND ICI. `v_proposal_dashboard` est faite pour
    // le comité — notes, rang, revues manquantes —, et l'espace organisation
    // n'en montrerait rien. Ces compositions appartiendront donc à l'API
    // (prompt B4), pas à une vue SQL supplémentaire.
    //
    // Sorti dans `composables/api/organization-workspace.ts` au prompt A12, pour
    // tenir ce fichier sous le garde-fou de mille lignes de `CLAUDE.md` : c'est
    // un écran entier, donc l'unité de découpage du projet.
    // -----------------------------------------------------------------------
    workspace: createOrganizationWorkspaceApi(deps),

    // -----------------------------------------------------------------------
    // Back-office (A6)
    //
    // UNE COMPOSITION, PAS NEUF LECTURES. Le tableau de bord ouvre sur cinq
    // familles d'alerte, trois projections analytiques, une vue de santé et les
    // incidents actifs. Lues séparément, elles produisent neuf allers-retours au
    // chargement d'une page consultée vingt fois par jour — et neuf instants de
    // mesure différents dans un même écran, où l'entonnoir et la liste des
    // dossiers finissent par ne plus dire la même chose.
    //
    // LE PÉRIMÈTRE EST VÉRIFIÉ AVANT L'APPEL, comme pour la liste des
    // propositions et le planificateur : une édition hors périmètre REFUSE
    // l'accès plutôt que de rendre un tableau de bord vide, qui se lirait comme
    // « il ne se passe rien » au lieu de « ceci ne vous regarde pas ».
    // -----------------------------------------------------------------------
    admin: {
      /**
       * EN ATTENTE D'API — aucun crate ne porte le schéma `analytics`, dont les
       * cinq projections composent cet écran. Il lit donc des exemples, et le
       * dit : voir `pending()` plus haut.
       */
      dashboard: (eventId: Uuid, scope: AdministeredEvents): Promise<AdminDashboard | null> => {
        assertEventInScope(eventId, scope)
        return pending('/admin/dashboard', (m) => m.adminDashboard(eventId))
      },

      /**
       * SANTÉ OPÉRATIONNELLE SEULE — `analytics.v_operational_health`.
       *
       * Elle est DANS la composition ci-dessus, et disponible à part pour une
       * seule raison : c'est la seule zone de l'écran qui se rafraîchit sans
       * recharger le reste. Une file qui se vide se regarde en direct ; un
       * entonnoir matérialisé, non.
       *
       * ELLE NE DÉPEND D'AUCUNE ÉDITION : elle mesure la plateforme. Ce qu'elle
       * révèle — des courriels en rebond, un outbox en retard — ne dit rien
       * d'une autre COP, et un administrateur détaché doit le voir : les rappels
       * qui ne partent plus sont ceux de SES activités.
       */
      operationalHealth: () => call('/health', (m) => m.operationalHealth()),

      // `overview` a disparu : aucun écran n'affichait ces compteurs, et les
      // seuls qu'on aurait voulus — les doublons d'organisations non arbitrés —
      // arrivent déjà par la revue des doublons du back-office.
    },

    // -----------------------------------------------------------------------
    // Inscriptions
    // -----------------------------------------------------------------------
    registrations: {
      /**
       * La liste NOMINATIVE des inscrits d'une séance.
       *
       * L'API rend `RegistrationRow[]` — l'inscription, la personne et le nom de
       * son organisation, joints par la requête —, et non la ligne nue :
       * afficher une liste d'inscrits demande de les nommer, et les résoudre un
       * par un côté site serait le N+1 que cette composition existe pour éviter.
       */
      forSession: (sessionId: Uuid, scope: AdministeredEvents): Promise<RegistrationRow[]> => {
        return call(
          '/registrations',
          (m) => {
            const session = m.allSessions.find((s) => s.id === sessionId)
            if (!session) return []
            assertEventInScope(session.event_id, scope)
            return m.registrationRowsOf(sessionId)
          },
          { session_id: sessionId },
        )
      },
      /**
       * Ce à quoi la personne connectée est inscrite, annulations comprises.
       *
       * L'identifiant reçu ne part PAS dans la requête : `/registrations/mine`
       * lit sa propre session, et le lui envoyer laisserait croire qu'un écran
       * pourrait lire les inscriptions de quelqu'un d'autre. Il ne sert qu'à
       * retrouver la personne dans les données simulées.
       */
      forPerson: (personId: Uuid) =>
        call('/registrations/mine', (m) => m.registrations.filter((r) => r.person_id === personId)),
      /** Formulaire applicable à une séance, avec ses champs actifs. */
      form: (sessionId: Uuid) =>
        call(`/sessions/${sessionId}/registration-form`, (m) => {
          const session = m.allSessions.find((s) => s.id === sessionId)
          const form =
            m.registrationForms.find((f) => f.id === session?.registration_form_id) ??
            m.registrationForms.find((f) => f.is_default)
          if (!form) return null
          return {
            form,
            fields: m.registrationFormFields
              .filter((f) => f.form_id === form.id && f.is_active)
              .sort((a, b) => a.sort_order - b.sort_order),
          }
        }),
    },
  }
}
