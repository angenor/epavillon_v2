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
import type {
  SaveDraftPayload,
  SaveDraftResult,
  SubmitProposalPayload,
  SubmitProposalResult,
} from '~/types/proposal-form'
import type { AdminDashboard } from '~/types/admin-dashboard'
import type { FeatureFlag } from '~/types/platform'
import type {
  AssignReviewerPayload,
  BulkResult,
  ChangeStatusPayload,
  ProposalFacet,
  ProposalListScreen,
} from '~/types/admin-proposals'
import type {
  DecideMembershipPayload,
  InviteMemberPayload,
  InviteMemberResult,
} from '~/types/organization-workspace'
import type { Uuid } from '~/types/shared'
import { createProposalReviewApi } from './api/proposal-review'
import { createPlannerApi } from './api/planner'
import { createAdminEventsApi } from './api/admin-events'
import { createAdminOrganizationsApi } from './api/admin-organizations'
import { createAdminUsersApi } from './api/admin-users'
import { createAdminIncidentsApi } from './api/admin-incidents'
import { createOrganizationWorkspaceApi } from './api/organization-workspace'

/** Erreur d'accès, à traduire par l'écran « accès refusé ». */
export class ForbiddenError extends Error {
  constructor(message = "Cette édition n'est pas dans votre périmètre d'administration.") {
    super(message)
    this.name = 'ForbiddenError'
  }
}

/** Périmètre sans aucun droit : la valeur sûre par défaut. */
export const NO_ADMIN_SCOPE: AdministeredEvents = { is_global: false, event_ids: [] }

const MOCK_LATENCY_MS = 120

export function useApi() {
  const config = useRuntimeConfig()
  const baseURL = String(config.public.apiBase ?? '')
  /**
   * La locale vient de l'instance i18n de l'application, PAS de `useI18n()`.
   *
   * `useI18n()` exige d'être appelé au sommet d'un `setup` et lève sinon
   * « Must be called at the top of a `setup` function ». Or ce composable est
   * consommé depuis un store initialisé par un middleware de navigation
   * (`stores/auth.ts`, chargé par `middleware/guest.ts`) : hors de tout
   * composant. L'instance injectée, elle, est disponible partout où le contexte
   * Nuxt l'est — et c'est la même.
   */
  const { $i18n } = useNuxtApp()
  const locale = $i18n.locale

  const isConfigured = computed(() => baseURL.length > 0)

  const client = $fetch.create({
    baseURL,
    // Les cookies de session sont posés par l'API : la requête doit les porter.
    credentials: 'include',
    retry: 1,
    retryStatusCodes: [408, 409, 425, 429, 500, 502, 503, 504],
    onRequest({ options }) {
      // L'API résout les colonnes `platform.i18n_text` selon cet en-tête.
      options.headers.set('Accept-Language', locale.value)
    },
  })

  type Mocks = typeof import('~/mocks')

  /**
   * Un appel de données : sa route d'API et, tant qu'elle n'existe pas, la
   * lecture équivalente dans les mocks.
   */
  async function call<T>(path: string, fromMocks: (m: Mocks) => T | Promise<T>, query?: Record<string, unknown>): Promise<T> {
    if (isConfigured.value) {
      return client<T>(path, { query })
    }

    const mocks = await import('~/mocks')
    if (import.meta.client && MOCK_LATENCY_MS > 0) {
      await new Promise((resolve) => setTimeout(resolve, MOCK_LATENCY_MS))
    }
    return fromMocks(mocks)
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
      return client<T>(path, { method, body: body as Record<string, unknown> })
    }

    const mocks = await import('~/mocks')
    if (import.meta.client && MOCK_LATENCY_MS > 0) {
      // Une écriture attend plus longtemps qu'une lecture : c'est pendant ce
      // temps-là que le bouton doit se désactiver et afficher son témoin. Sans
      // cette latence, l'état de chargement d'un formulaire ne se voit jamais et
      // finit par ne plus être écrit.
      await new Promise((resolve) => setTimeout(resolve, MOCK_LATENCY_MS * 3))
    }
    return fromMocks(mocks)
  }

  /** Refuse l'accès à une édition hors périmètre, plutôt que de rendre une liste vide. */
  function assertEventInScope(eventId: Uuid, scope: AdministeredEvents): void {
    if (!scope.is_global && !scope.event_ids.includes(eventId)) {
      throw new ForbiddenError()
    }
  }

  return {
    baseURL,
    /** L'API est-elle configurée ? Faux tant que la variable n'est pas posée. */
    isConfigured,
    /** Client HTTP préconfiguré. Réservé aux cas non couverts ci-dessous. */
    client,
    ForbiddenError,
    assertEventInScope,

    // -----------------------------------------------------------------------
    // Authentification (A1)
    //
    // Les cinq écrans d'authentification passent par ici, et par rien d'autre.
    // Deux règles s'y jouent, l'une et l'autre invisibles depuis les pages :
    //
    //  · DISCRÉTION — `register` et `requestPasswordReset` rendent TOUJOURS la
    //    même réponse, adresse connue ou non. Rien dans le contrat ne permet
    //    d'écrire un écran bavard, même par inadvertance.
    //  · SESSION — l'API pose un cookie `HttpOnly` que le navigateur renvoie
    //    seul (`credentials: 'include'`, plus haut). Tant qu'elle n'existe pas,
    //    le store d'authentification tient un cookie ordinaire et passe
    //    l'identifiant à `session()` ; ce paramètre disparaîtra au prompt B7,
    //    l'API lisant sa propre session.
    // -----------------------------------------------------------------------
    auth: {
      login: (payload: LoginPayload): Promise<LoginResult> =>
        send('/auth/login', payload, (m) => m.authenticate(payload)),

      logout: (): Promise<{ status: 'signed_out' }> =>
        send('/auth/logout', {}, () => ({ status: 'signed_out' as const })),

      /** Personne connectée, ou `null` si la session n'existe plus. */
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
      /** `platform.feature_flags`, toutes clés confondues. */
      featureFlags: (): Promise<FeatureFlag[]> =>
        call('/platform/feature-flags', (m) => m.featureFlags),
    },

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
       */
      byEmailDomain: (email: string): Promise<EmailDomainMatch | null> =>
        call('/organizations/by-email-domain', (m) => m.organizationForEmail(email), { email }),

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

      names: (id: Uuid) =>
        call(`/organizations/${id}/names`, (m) => m.organizationNames.filter((n) => n.organization_id === id)),
      domains: (id: Uuid) =>
        call(`/organizations/${id}/domains`, (m) => m.organizationDomains.filter((d) => d.organization_id === id)),
      members: (id: Uuid) =>
        call(`/organizations/${id}/members`, (m) => m.memberships.filter((x) => x.organization_id === id)),
      /** Doublons présumés en attente de décision (A11). */
      duplicates: () =>
        call('/organizations/duplicates', (m) => m.duplicateCandidates.filter((d) => d.decision === null)),
    },

    // -----------------------------------------------------------------------
    // Personnes et habilitations
    // -----------------------------------------------------------------------
    identity: {
      people: () => call('/people', (m) => m.people),
      byId: (id: Uuid) => call(`/people/${id}`, (m) => m.people.find((p) => p.id === id) ?? null),
      roleAssignments: (personId: Uuid) =>
        call(`/people/${personId}/roles`, (m) => m.roleAssignments.filter((r) => r.person_id === personId)),

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
       * VISUEL DE L'ÉDITION — `media.attached_image('event','events',id,'banner')`.
       *
       * Un appel à part, et c'en est un de trop : `event.events` ne porte pas son
       * image, le rattachement média étant polymorphe. La couverture d'une
       * séance, elle, est résolue EN BASE par `v_public_schedule`. Obligation
       * inscrite au prompt B3 : la réponse de `GET /events/:slug` embarque sa
       * bannière résolue, et cet appel disparaît.
       */
      banner: (eventId: Uuid) =>
        call(`/events/${eventId}/banner`, (m) => m.attachedImage('event', 'events', eventId, 'banner')),
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
      call: (eventId: Uuid) =>
        call(`/events/${eventId}/call`, (m) => m.callsForProposals.find((c) => c.event_id === eventId) ?? null),
    },

    calls: {
      criteria: (callId: Uuid) =>
        call(`/calls/${callId}/criteria`, (m) => m.reviewCriteria.filter((c) => c.call_id === callId)),
      reviewers: (callId: Uuid) =>
        call(`/calls/${callId}/reviewers`, (m) => m.callReviewers.filter((r) => r.call_id === callId)),
    },

    // -----------------------------------------------------------------------
    // Propositions
    // -----------------------------------------------------------------------
    proposals: {
      /** Liste du back-office (A7) : la vue prête à l'emploi, filtrée par périmètre. */
      dashboard: (eventId: Uuid, scope: AdministeredEvents) => {
        assertEventInScope(eventId, scope)
        return call(
          '/proposals/dashboard',
          (m) => m.proposalDashboard().filter((row) => row.event_id === eventId),
          { event_id: eventId },
        )
      },

      /**
       * TOUT L'ÉCRAN DE LA LISTE EN UNE RÉPONSE (A7) — les lignes de la vue, les
       * facettes des filtres avec leur décompte, les dossiers que la personne
       * n'a pas encore ouverts, le fuseau de l'édition et l'échéance de l'appel.
       *
       * UNE COMPOSITION, PAS SIX LECTURES. Les facettes se comptent sur le même
       * jeu de lignes que la liste : demandées à part, elles seraient mesurées à
       * un autre instant, et le « Retenu (17) » du filtre finirait par ne plus
       * correspondre aux lignes affichées.
       *
       * LE FILTRAGE ET LE TRI RESTENT CÔTÉ ÉCRAN tant que les données sont
       * simulées : quarante lignes tiennent en mémoire, et un tri serveur sur
       * quarante lignes serait un aller-retour pour rien. Au raccordement (B7),
       * ces paramètres deviendront ceux de la requête — c'est pourquoi ils vivent
       * dans l'URL et non dans un état de composant.
       */
      list: (eventId: Uuid, scope: AdministeredEvents, personId: Uuid | null): Promise<ProposalListScreen | null> => {
        assertEventInScope(eventId, scope)
        return call('/proposals/list', (m) => m.proposalListScreen(eventId, personId), {
          event_id: eventId,
        })
      },

      /** Composition du comité de l'appel, avec la charge de chacun. */
      committee: (eventId: Uuid, scope: AdministeredEvents): Promise<ProposalFacet[]> => {
        assertEventInScope(eventId, scope)
        return call('/proposals/committee', (m) => m.committeeOf(eventId), { event_id: eventId })
      },

      /**
       * LA MACHINE À ÉTATS, LUE ET NON RÉÉCRITE —
       * `programme.proposal_transitions_allowed`. L'écran n'affiche que les
       * transitions déclarées, avec leur permission et leur exigence de motif.
       * Ajouter un chemin en base ajoute une action, sans toucher au code.
       */
      transitionRules: () =>
        call('/proposals/transitions', (m) => m.proposalTransitionsAllowed),

      /**
       * ACTION GROUPÉE — confier une sélection à un membre du comité.
       *
       * La réponse dit ce qui A ÉTÉ FAIT et, dossier par dossier, ce qui ne l'a
       * pas été : déjà affecté, déporté, introuvable. Une action de masse qui ne
       * rend qu'un nombre laisse croire à un succès complet.
       */
      assignReviewer: (actorId: Uuid | null, payload: AssignReviewerPayload): Promise<BulkResult> =>
        send('/proposals/assignments', payload, (m) => m.assignReviewer(payload, actorId)),

      /**
       * ACTION GROUPÉE — changer le statut d'une sélection. Les transitions
       * refusées par `proposal_transitions_allowed` sont écartées avec leur
       * motif, comme le ferait le trigger.
       */
      changeStatus: (actorId: Uuid | null, payload: ChangeStatusPayload): Promise<BulkResult> =>
        send('/proposals/status', payload, (m) => m.changeProposalStatus(payload, actorId)),
      byId: (id: Uuid) => call(`/proposals/${id}`, (m) => m.allProposals.find((p) => p.id === id) ?? null),
      /** Dossiers d'une organisation, brouillons compris (A5). */
      forOrganization: (organizationId: Uuid) =>
        call(
          '/proposals',
          (m) => m.allProposals.filter((p) => p.organization_id === organizationId),
          { organization_id: organizationId },
        ),
      organizations: (id: Uuid) =>
        call(`/proposals/${id}/organizations`, (m) => m.proposalOrganizations.filter((o) => o.proposal_id === id)),
      speakers: (id: Uuid) =>
        call(`/proposals/${id}/speakers`, (m) => m.proposalSpeakers.filter((s) => s.proposal_id === id)),
      documents: (id: Uuid) =>
        call(`/proposals/${id}/documents`, (m) => m.proposalDocuments.filter((d) => d.proposal_id === id)),
      /** Fil d'échanges, filtré sur ce que le lecteur a le droit de voir. */
      comments: (id: Uuid, viewerId: Uuid, isCommittee: boolean) =>
        call(`/proposals/${id}/comments`, (m) =>
          m.proposalComments.filter((c) => {
            if (c.proposal_id !== id || c.deleted_at !== null) return false
            if (c.visibility === 'private') return c.author_id === viewerId
            if (c.visibility === 'committee') return isCommittee
            return true
          }),
        ),
      history: (id: Uuid) =>
        call(`/proposals/${id}/transitions`, (m) => m.proposalTransitions.filter((t) => t.proposal_id === id)),
      /**
       * OÙ L'ON DÉPOSE AUJOURD'HUI, et ce que l'organisation a déjà déposé.
       *
       * Le formulaire de soumission (A4) ne choisit pas son édition : il y en a
       * au plus une dont l'appel est ouvert (`ux_calls_one_per_event` et
       * `event.is_call_open()`). Ce contexte porte aussi le décompte du plafond
       * `max_proposals_per_organization`, que le trigger de recevabilité
       * appliquera de toute façon — l'écran doit pouvoir le dire AVANT sept
       * étapes de saisie, pas après.
       */
      formContext: (personId: Uuid, organizationIds: Uuid[]) =>
        call('/proposals/form-context', (m) => m.proposalFormContext(personId, organizationIds), {
          organization_ids: organizationIds.join(','),
        }),

      /** Brouillon en cours de la personne, pour reprendre où elle s'est arrêtée. */
      myDraft: (personId: Uuid) =>
        call('/proposals/draft', (m) => m.draftProposalOf(personId)),

      /**
       * UN DOSSIER EXISTANT, RECOMPOSÉ EN BROUILLON — c'est ce qui permet de le
       * MODIFIER (arbitrage du commanditaire du 17/08 : « tant que l'événement
       * n'est pas terminé, il peut modifier »).
       *
       * La recomposition n'est pas un `SELECT` : le formulaire travaille sur une
       * structure d'écran — français, heures murales, clés de liste — quand la
       * base range la même chose dans cinq tables. Elle appartient donc à l'API
       * (prompt B4), pas à la page : deux écrans qui la referaient chacun de
       * leur côté divergeraient sur le premier champ ajouté.
       */
      forEdit: (proposalId: Uuid) =>
        call(`/proposals/${proposalId}/draft`, (m) => m.editableProposal(proposalId)),

      /**
       * ENREGISTREMENT AUTOMATIQUE. Le premier appel CRÉE la ligne et rend son
       * numéro de dossier — `tg_assign_reference_code` s'exécute à l'insertion,
       * pas au dépôt. Les suivants ne font que dater.
       */
      saveDraft: (personId: Uuid, payload: SaveDraftPayload): Promise<SaveDraftResult> =>
        send(
          payload.proposal_id ? `/proposals/${payload.proposal_id}` : '/proposals',
          payload,
          // Un dossier DÉJÀ EN BASE se met à jour sans changer d'état : corriger
          // n'est pas déposer, et un dossier en évaluation ne repart pas au
          // comité parce qu'on a rectifié une faute de frappe.
          (m) => m.saveExistingProposal(payload) ?? m.saveProposalDraft(personId, payload),
          payload.proposal_id ? 'PUT' : 'POST',
        ),

      /**
       * RENVOI AU COMITÉ d'un dossier corrigé — `changes_requested → submitted`.
       *
       * Distinct du dépôt : la fenêtre de l'appel ne s'y applique pas. Le comité
       * demande ses corrections APRÈS la clôture, et le trigger de recevabilité
       * les refusait toutes jusqu'à sa correction du 17/08 — laissant
       * l'organisation devant un écran qui réclamait l'impossible.
       */
      resubmit: (payload: SaveDraftPayload): Promise<SubmitProposalResult> =>
        send(`/proposals/${payload.proposal_id}/resubmit`, payload, (m) => {
          const result = m.resubmitProposal(payload)
          if (!result) throw new Error(`Dossier ${payload.proposal_id} introuvable.`)
          return result
        }),

      /**
       * DÉPÔT — la transition `draft → submitted`. Les deux refus possibles sont
       * ceux de `tg_check_submission_eligibility()` : appel clos, plafond
       * atteint. Ils ne sont pas des erreurs de réseau mais des réponses, et
       * l'écran les rend comme telles.
       */
      submit: (personId: Uuid, payload: SubmitProposalPayload): Promise<SubmitProposalResult> =>
        send(`/proposals/${payload.proposal_id}/submit`, payload, (m) =>
          m.submitProposal(personId, payload),
        ),

      /**
       * LA PERSONNE QUI PORTE CETTE ADRESSE, si la plateforme la connaît.
       *
       * Même intention que `organizations.similar()` : ne pas créer une seconde
       * fiche pour quelqu'un qui existe déjà. La clé est l'adresse et rien
       * d'autre — `people.primary_email` est la clé de rapprochement du modèle,
       * et chercher par nom rapprocherait deux homonymes, ce qui est pire qu'un
       * doublon. Aucun appel ne rend l'annuaire entier : une plateforme ne
       * diffuse pas sa liste de contacts pour remplir un formulaire.
       */
      lookupSpeaker: (email: string) =>
        call('/people/lookup', (m) => m.lookupSpeakerByEmail(email), { email }),

      /**
       * L'HISTORIQUE CHAMP PAR CHAMP — `programme.proposal_history()`.
       *
       * Sous-produit du journal d'audit, et non une table entretenue à la main :
       * toute écriture y figure, y compris une correction faite en console. La
       * v1 tenait une table `activity_modifications` alimentée par le code
       * applicatif, qui ne couvrait que ce qui passait par le bon chemin.
       */
      fieldHistory: (id: Uuid) =>
        call(`/proposals/${id}/history`, (m) => m.proposalHistory(id)),

      themes: (id: Uuid) =>
        call(`/proposals/${id}/themes`, (m) => {
          const termIds = new Set(
            m.entityTerms
              .filter((t) => t.entity_table === 'proposals' && t.entity_id === id)
              .map((t) => t.term_id),
          )
          return m.taxonomyTerms.filter((t) => termIds.has(t.id))
        }),
    },

    // -----------------------------------------------------------------------
    // Évaluation
    // -----------------------------------------------------------------------
    reviews: {
      forProposal: (proposalId: Uuid) =>
        call(`/proposals/${proposalId}/reviews`, (m) =>
          m.reviews.filter((r) => r.proposal_id === proposalId && r.submitted_at !== null),
        ),
      /** Brouillon de la personne connectée, y compris non soumis. */
      mine: (proposalId: Uuid, reviewerId: Uuid) =>
        call(`/proposals/${proposalId}/reviews/mine`, (m) =>
          m.reviews.find((r) => r.proposal_id === proposalId && r.reviewer_id === reviewerId) ?? null,
        ),
      scores: (reviewId: Uuid) =>
        call(`/reviews/${reviewId}/scores`, (m) => m.reviewScores.filter((s) => s.review_id === reviewId)),
      assignments: (proposalId: Uuid) =>
        call(`/proposals/${proposalId}/assignments`, (m) =>
          m.reviewAssignments.filter((a) => a.proposal_id === proposalId),
        ),
      /** Charge de travail d'un membre du comité, déports exclus. */
      assignedTo: (reviewerId: Uuid) =>
        call('/reviews/assignments', (m) =>
          m.reviewAssignments.filter((a) => a.reviewer_id === reviewerId && a.recused_at === null),
        ),
    },

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
    review: createProposalReviewApi({ call, send }),

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
      bySlug: (eventId: Uuid, slug: string) =>
        call(`/events/${eventId}/sessions/${slug}`, (m) =>
          m.allSessions.find((s) => s.event_id === eventId && s.slug === slug) ?? null,
        ),
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
      /** Ce qui reste à régler avant de publier le programme. */
      publicationReadiness: (eventId: Uuid, scope: AdministeredEvents) => {
        assertEventInScope(eventId, scope)
        return call('/sessions/publication-readiness', (m) => m.publicationReadiness(eventId), {
          event_id: eventId,
        })
      },
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
    planner: createPlannerApi({ call, send, assertEventInScope }),

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
    adminEvents: createAdminEventsApi({ call, send, assertEventInScope }),

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
    adminOrganizations: createAdminOrganizationsApi({ call, send }),

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
    adminUsers: createAdminUsersApi({ call, send }),

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
    adminIncidents: createAdminIncidentsApi({ call, send, assertEventInScope }),

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
    workspace: createOrganizationWorkspaceApi({ call, send }),

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
      dashboard: (eventId: Uuid, scope: AdministeredEvents): Promise<AdminDashboard | null> => {
        assertEventInScope(eventId, scope)
        return call('/admin/dashboard', (m) => m.adminDashboard(eventId), { event_id: eventId })
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
      operationalHealth: () => call('/admin/health', (m) => m.operationalHealth()),

      /** Compteurs temps réel de la plateforme — `analytics.v_platform_overview`. */
      overview: () => call('/admin/overview', (m) => m.platformOverview()),
    },

    // -----------------------------------------------------------------------
    // Inscriptions
    // -----------------------------------------------------------------------
    registrations: {
      forSession: (sessionId: Uuid, scope: AdministeredEvents) => {
        return call(
          '/registrations',
          (m) => {
            const session = m.allSessions.find((s) => s.id === sessionId)
            if (!session) return []
            assertEventInScope(session.event_id, scope)
            return m.registrations.filter((r) => r.session_id === sessionId)
          },
          { session_id: sessionId },
        )
      },
      /** Ce à quoi une personne est inscrite, annulations comprises. */
      forPerson: (personId: Uuid) =>
        call('/registrations/mine', (m) => m.registrations.filter((r) => r.person_id === personId), {
          person_id: personId,
        }),
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
