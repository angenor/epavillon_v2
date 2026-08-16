/**
 * Schéma `event`, partie 4 — appel à propositions, grille d'évaluation, comité.
 * Dérivé de `docs/database/060_events.sql` § 5, 6 et 7.
 *
 * CARDINALITÉ : une édition porte ZÉRO ou UN appel, jamais plusieurs. Zéro pour
 * une COP sans pavillon ; un pour une COP climat classique. La règle est tenue
 * par l'index `ux_calls_one_per_event`, pas par l'application. Les journées
 * thématiques sont composées APRÈS sélection, à partir du vivier commun : elles
 * n'ouvrent pas leur propre fenêtre de soumission.
 */

import type {
  CallId,
  CriterionId,
  EventId,
  I18nText,
  IsoDate,
  IsoDateTime,
  Numeric,
  PersonId,
  Url,
} from '../shared'
import type { ParticipationMode } from './edition'

// ---------------------------------------------------------------------------
// Appel à propositions
// ---------------------------------------------------------------------------

/** ENUM `event.call_status`. */
export type CallStatus = 'draft' | 'open' | 'closed' | 'under_review' | 'published' | 'cancelled'

/** Table `event.calls_for_proposals` — `060_events.sql` § 5. */
export interface CallForProposals {
  id: CallId
  event_id: EventId
  code: string
  title: I18nText
  description: I18nText | null
  status: CallStatus
  opens_at: IsoDateTime
  closes_at: IsoDateTime
  /** Prolongation, conservée à part pour garder trace de l'échéance annoncée à
   *  l'origine. L'échéance effective est `extended_until ?? closes_at` —
   *  c'est `event.effective_deadline()` côté base. */
  extended_until: IsoDateTime | null
  results_expected_at: IsoDate | null
  max_proposals_per_organization: number | null
  requires_verified_organization: boolean
  min_speakers: number
  max_speakers: number
  default_duration_minutes: number
  allowed_formats: ParticipationMode[]
  /** Nombre de revues indépendantes exigé avant décision. */
  required_reviews: number
  /** Évaluation en aveugle : un révisionniste ne voit les notes des autres
   *  qu'après avoir soumis la sienne. */
  blind_review: boolean
  guidelines_url: Url | null
  created_by: PersonId | null
  created_at: IsoDateTime
  updated_at: IsoDateTime
}

// ---------------------------------------------------------------------------
// Grille d'évaluation
// ---------------------------------------------------------------------------

/**
 * Table `event.review_criteria` — `060_events.sql` § 6.
 * La note finale est la somme pondérée de ces critères, définis par appel : la
 * v1 se contentait d'une note libre sur 20, sans justification opposable.
 */
export interface ReviewCriterion {
  id: CriterionId
  call_id: CallId
  code: string
  label: I18nText
  description: I18nText | null
  max_score: Numeric
  weight: Numeric
  /** Critère éliminatoire : une note nulle disqualifie la proposition, quelle
   *  que soit la moyenne générale. */
  is_knockout: boolean
  sort_order: number
}

// ---------------------------------------------------------------------------
// Comité de sélection
// ---------------------------------------------------------------------------

/**
 * Table `event.call_reviewers` — `060_events.sql` § 7.
 * Qui siège sur cet appel. L'AUTORISATION reste portée par
 * `identity.role_assignments` (portée `event`) : cette table dit la composition,
 * pas le droit d'accès.
 */
export interface CallReviewer {
  call_id: CallId
  person_id: PersonId
  is_lead: boolean
  /** Plafond indicatif de propositions confiées à ce membre. */
  workload_cap: number | null
  added_at: IsoDateTime
}
