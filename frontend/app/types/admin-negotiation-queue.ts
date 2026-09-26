/**
 * La file des experts de Guide Négo — `GET /admin/negotiation/queue`. Champ pour
 * champ comme `negotiation/src/domain/admin_file.rs` ; contrat dans
 * `specs/013-guide-nego-faq-lexique/contracts/api-admin-savoir.md`.
 *
 * Aucun auteur n'est jamais rendu : ni celui qui signale ou qui questionne, ni
 * l'expert qui clôt, qui répond ou qui tranche une proposition (R9).
 */

import type { AdminFaqFeedback, AdminFaqReport, AdminKnowledgeStatus } from './admin-negotiation-savoir'
import type { ExpertQuestionStatus } from './negotiation-savoir'
import type { IsoDate, IsoDateTime, Uuid } from './shared'

export type ExpertQueueKind = 'reports' | 'questions' | 'proposals'

/** Ce qui attend, par sorte. */
export interface ExpertQueueCounts {
  reports: number
  questions: number
  proposals: number
}

/** L'entrée signalée, question résolue dans la langue demandée. */
export interface ExpertQueueFaqRef {
  id: Uuid
  question: string
  status: AdminKnowledgeStatus
  verified_on: IsoDate | null
}

/** Les signalements ouverts d'une entrée, du plus ancien au plus récent. */
export interface ExpertQueueReportGroup {
  entry: ExpertQueueFaqRef
  feedback: AdminFaqFeedback
  reports: AdminFaqReport[]
}

/** Une question aux experts, sans son auteure ni l'expert qui répond. */
export interface AdminQuestion {
  id: Uuid
  theme_code: string
  theme_label: string
  body: string
  consent_to_faq: boolean
  status: ExpertQuestionStatus
  answer: string | null
  answered_at: IsoDateTime | null
  faq_entry_id: Uuid | null
  created_at: IsoDateTime
}

/**
 * Remplie selon `kind`. `reports` : les groupes vont du signalement ouvert le plus
 * ancien au plus récent. `questions` : celles qui attendent, la plus ancienne
 * d'abord, puis celles répondues depuis trente jours et pas encore promues.
 * `proposals` : les termes proposés en attente, le plus ancien d'abord.
 */
export interface ExpertQueue {
  kind: ExpertQueueKind
  counts: ExpertQueueCounts
  reports: ExpertQueueReportGroup[]
  questions: AdminQuestion[]
  proposals: AdminProposal[]
}

export type GlossaryProposalStatus = 'pending' | 'accepted' | 'rejected'

/** Ce qu'un auteur a écrit, sans l'auteur. */
export interface AdminProposalContext {
  context: string | null
  created_at: IsoDateTime
}

/** Une entrée du lexique proche du terme proposé (`similarity ≥ 0,4`), tous statuts. */
export interface AdminProposalNearby {
  id: Uuid
  slug: string
  term: string
  status: AdminKnowledgeStatus
  similarity: number
}

/** Un terme proposé : toutes les propositions du même terme normalisé s'y regroupent. */
export interface AdminProposal {
  id: Uuid
  term: string
  status: GlossaryProposalStatus
  authors_count: number
  /** Un par auteur, le plus ancien d'abord. */
  contexts: AdminProposalContext[]
  /** La plus proche d'abord, cinq au plus. */
  nearby: AdminProposalNearby[]
  glossary_entry_id: Uuid | null
  glossary_entry_slug: string | null
  rejection_reason: string | null
  created_at: IsoDateTime
  handled_at: IsoDateTime | null
}

/** `POST /admin/negotiation/queue/proposals/{id}/reject`. */
export interface AdminProposalRejectInput {
  reason: string
}

/** `POST /admin/negotiation/queue/questions/{id}/answer` — met en file le courriel. */
export interface AdminQuestionAnswerInput {
  answer: string
}

/** `POST /admin/negotiation/queue/questions/{id}/promote` — brouillon de FAQ, sans auteur. */
export interface AdminQuestionPromoteInput {
  section_code: string
}

export type FaqReportOutcome = 'revised' | 'confirmed' | 'dismissed'

/** `POST /admin/negotiation/queue/reports/{id}/close` — ne touche jamais l'entrée (FR-018). */
export interface AdminFaqReportCloseInput {
  outcome: FaqReportOutcome
}
