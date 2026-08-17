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
 */

import type { AdministeredEvents, Person } from '~/types/identity'
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
import type { Uuid } from '~/types/shared'

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
       * `identity.administered_events()` : périmètre d'administration d'une
       * personne. Renvoie TOUJOURS une valeur pleine — jamais `null` — de sorte
       * que « aucun droit » et « administrateur d'une édition » ne se confondent.
       */
      administeredEvents: (personId: Uuid): Promise<AdministeredEvents> =>
        call(`/people/${personId}/administered-events`, (m) => {
          const now = Date.now()
          const active = m.roleAssignments.filter(
            (r) =>
              r.person_id === personId &&
              r.revoked_at === null &&
              (r.valid_until === null || Date.parse(r.valid_until) > now),
          )
          const adminRoles = ['super_admin', 'admin', 'programmer']
          return {
            is_global: active.some((r) => adminRoles.includes(r.role_code) && r.scope_type === 'global'),
            event_ids: [
              ...new Set(
                active
                  .filter((r) => adminRoles.includes(r.role_code) && r.scope_type === 'event' && r.scope_id !== null)
                  .map((r) => r.scope_id as Uuid),
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
