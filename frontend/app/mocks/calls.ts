/**
 * Données simulées de `event.calls_for_proposals` et `event.call_reviewers`.
 *
 * UN SEUL APPEL PAR ÉDITION, jamais plusieurs — l'index `ux_calls_one_per_event`
 * le garantit en base. Les journées spéciales sont composées APRÈS sélection, à
 * partir du vivier commun : elles n'ouvrent pas leur propre fenêtre de dépôt.
 *
 * L'appel simulé est OUVERT : ouvert le 1er juin 2026, il devait se clore le
 * 31 août et a été prolongé au 30 septembre. L'échéance effective est donc
 * `extended_until`, pas `closes_at` — c'est `event.effective_deadline()` côté
 * base, et l'écran de soumission doit afficher la même. Conserver les deux dates
 * permet de dire « prolongé jusqu'au… », ce que la v1 ne savait pas faire :
 * elle écrasait l'échéance annoncée.
 */

import type { CallForProposals, CallReviewer } from '~/types/event/call'
import { CALL, EVENT, PERSON } from './ids'

export const callsForProposals = [
  {
    id: CALL.cop31,
    event_id: EVENT.cop31,
    code: 'cop31_activites',
    title: {
      fr: "Appel à propositions d'activités — Pavillon de la Francophonie, COP31",
      en: 'Call for activity proposals — Francophonie Pavilion, COP31',
    },
    description: {
      fr: "L'IFDD invite les organisations francophones à proposer des activités pour le pavillon de la Francophonie à Belém. Les propositions peuvent être portées par plusieurs organisations. Chaque dossier est évalué par trois membres du comité de sélection sur six critères pondérés.",
      en: 'IFDD invites French-speaking organizations to propose activities for the Francophonie Pavilion in Belém.',
    },
    status: 'open',
    opens_at: '2026-06-01T00:00:00-03:00',
    closes_at: '2026-08-31T23:59:59-03:00',
    // Prolongation : l'échéance annoncée à l'origine reste lisible à côté.
    extended_until: '2026-09-30T23:59:59-03:00',
    results_expected_at: '2026-11-15',
    max_proposals_per_organization: 4,
    requires_verified_organization: false,
    min_speakers: 2,
    max_speakers: 6,
    default_duration_minutes: 90,
    allowed_formats: ['in_person', 'hybrid', 'online'],
    required_reviews: 3,
    // Évaluation à découvert : le commanditaire souhaite que les membres du
    // comité voient les notes des autres et la moyenne. Valeur par défaut encore
    // en arbitrage, consignée dans `docs/PROGRESSION.md`.
    blind_review: false,
    guidelines_url: 'https://www.ifdd.francophonie.org/cop31/appel-a-propositions',
    created_by: PERSON.bakayoko,
    created_at: '2026-05-12T09:00:00Z',
    updated_at: '2026-08-24T16:30:00Z',
  },
] satisfies CallForProposals[]

// ---------------------------------------------------------------------------
// Comité de sélection
//
// Cette table dit QUI SIÈGE, pas qui a le droit d'accéder : l'autorisation reste
// portée par `identity.role_assignments` (voir `mocks/people.ts`). Les deux se
// répondent — chaque personne listée ici porte le rôle `reviewer` sur la COP31.
// ---------------------------------------------------------------------------

export const callReviewers = [
  {
    call_id: CALL.cop31,
    person_id: PERSON.lemoine,
    is_lead: true,
    workload_cap: 15,
    added_at: '2026-05-20T10:00:00Z',
  },
  {
    call_id: CALL.cop31,
    person_id: PERSON.duchesne,
    is_lead: false,
    workload_cap: 20,
    added_at: '2026-05-20T10:05:00Z',
  },
  {
    call_id: CALL.cop31,
    person_id: PERSON.benAmor,
    is_lead: false,
    workload_cap: 12,
    added_at: '2026-05-20T10:07:00Z',
  },
  {
    call_id: CALL.cop31,
    person_id: PERSON.kabore,
    is_lead: false,
    workload_cap: 12,
    added_at: '2026-05-20T10:09:00Z',
  },
  {
    call_id: CALL.cop31,
    person_id: PERSON.rasoanaivo,
    is_lead: false,
    // Sans plafond déclaré : le back-office doit afficher « non renseigné » et
    // non « 0 », qui se lirait comme « aucune proposition à évaluer ».
    workload_cap: null,
    added_at: '2026-05-22T08:15:00Z',
  },
] satisfies CallReviewer[]
