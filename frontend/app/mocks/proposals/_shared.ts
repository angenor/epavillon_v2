/**
 * Fabrique commune aux quatre fichiers de propositions.
 *
 * Elle ne produit AUCUNE donnée : elle porte les champs qui valent la même chose
 * pour les quarante dossiers (l'édition, l'appel, les compteurs de suppression)
 * et les règles que la base impose de toute façon —
 * `reference_code` = sigle de l'édition + numéro sur cinq chiffres,
 * `submitted_at` obligatoire dès que le dossier n'est plus un brouillon
 * (`ck_proposals_submitted_at`).
 *
 * Tout ce qui distingue un dossier d'un autre — titre, résumé, objectifs,
 * porteur, créneau souhaité, note — est écrit à la main dans les fichiers de
 * statut, qui sont ce qu'un développeur va lire.
 *
 * LE DÉCOUPAGE PAR STATUT n'est pas arbitraire : il suit ce que les écrans
 * consultent. L'espace organisation lit les brouillons, la fiche d'évaluation
 * lit les dossiers en cours de revue, le planificateur ne connaît que les
 * dossiers retenus.
 */

import type { I18nText, IsoDateTime, Numeric } from '~/types/shared'
import type { Proposal, ProposalStatus } from '~/types/programme/proposal'
import type { ParticipationMode } from '~/types/event/edition'
import { CALL, EVENT } from '../ids'

export interface ProposalDraftFields {
  id: string
  /** Numéro de dossier : `COP31-00001` … `COP31-00040`. */
  ref: number
  organization: string
  submittedBy: string
  contact?: string | null
  title: I18nText
  slug: string
  summary?: I18nText | null
  objectives: I18nText
  presentation: I18nText
  outcomes?: I18nText | null
  audience?: I18nText | null
  format: ParticipationMode
  /** Code de la taxonomie `activity_category`. */
  category: string | null
  /** Codes de `reference.locales`. */
  languages?: string[]
  country?: string | null
  preferredStart?: IsoDateTime | null
  preferredEnd?: IsoDateTime | null
  duration?: number | null
  requestedSessions?: number
  constraints?: string | null
  status: ProposalStatus
  createdAt: IsoDateTime
  updatedAt?: IsoDateTime
  submittedAt?: IsoDateTime | null
  decidedAt?: IsoDateTime | null
  decisionReason?: string | null
  decidedBy?: string | null
  averageScore?: Numeric | null
  weightedScore?: Numeric | null
  reviewCount?: number
  knockedOut?: boolean
  viewCount?: number
}

export function proposal(fields: ProposalDraftFields): Proposal {
  const submitted_at = fields.submittedAt ?? null
  if (fields.status !== 'draft' && submitted_at === null) {
    // La base refuserait la ligne (`ck_proposals_submitted_at`) : mieux vaut le
    // savoir en écrivant le mock qu'au raccordement de l'API.
    throw new Error(`Proposition ${fields.ref} : un dossier « ${fields.status} » doit porter submitted_at.`)
  }

  return {
    id: fields.id,
    reference_code: `COP31-${String(fields.ref).padStart(5, '0')}`,
    call_id: CALL.cop31,
    event_id: EVENT.cop31,
    organization_id: fields.organization,
    submitted_by: fields.submittedBy,
    contact_person_id: fields.contact ?? fields.submittedBy,
    title: fields.title,
    slug: fields.slug,
    summary: fields.summary ?? null,
    objectives: fields.objectives,
    detailed_presentation: fields.presentation,
    expected_outcomes: fields.outcomes ?? null,
    target_audience: fields.audience ?? null,
    format: fields.format,
    activity_type_code: fields.category,
    language_codes: fields.languages ?? ['fr'],
    country_id: fields.country ?? null,
    preferred_start_at: fields.preferredStart ?? null,
    preferred_end_at: fields.preferredEnd ?? null,
    duration_minutes: fields.duration ?? 90,
    requested_sessions: fields.requestedSessions ?? 1,
    scheduling_constraints: fields.constraints ?? null,
    status: fields.status,
    submitted_at,
    decided_at: fields.decidedAt ?? null,
    decision_reason: fields.decisionReason ?? null,
    decided_by: fields.decidedBy ?? null,
    average_score: fields.averageScore ?? null,
    weighted_score: fields.weightedScore ?? null,
    review_count: fields.reviewCount ?? 0,
    is_knocked_out: fields.knockedOut ?? false,
    view_count: fields.viewCount ?? 0,
    deleted_at: null,
    deleted_by: null,
    deleted_reason: null,
    created_at: fields.createdAt,
    updated_at: fields.updatedAt ?? fields.createdAt,
  }
}
