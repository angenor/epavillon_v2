/**
 * La file des experts de Guide Négo — `GET /admin/negotiation/queue`. Champ pour
 * champ comme `negotiation/src/domain/admin_file.rs` ; contrat dans
 * `specs/013-guide-nego-faq-lexique/contracts/api-admin-savoir.md`.
 *
 * Aucun auteur n'est jamais rendu : ni celui qui signale ou qui questionne, ni
 * l'expert qui clôt ou qui répond (R9). La sorte `proposals` s'ajoute à la phase 11.
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
 */
export interface ExpertQueue {
  kind: ExpertQueueKind
  counts: ExpertQueueCounts
  reports: ExpertQueueReportGroup[]
  questions: AdminQuestion[]
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
