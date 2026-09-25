/**
 * La file des experts de Guide Négo — `GET /admin/negotiation/queue`. Champ pour
 * champ comme `negotiation/src/domain/admin_file.rs` ; contrat dans
 * `specs/013-guide-nego-faq-lexique/contracts/api-admin-savoir.md`.
 *
 * Aucun auteur n'est jamais rendu : ni celui qui signale, ni l'expert qui clôt (R9).
 * Les sortes `questions` et `proposals` s'ajoutent aux phases 8 et 11.
 */

import type { AdminFaqFeedback, AdminFaqReport, AdminKnowledgeStatus } from './admin-negotiation-savoir'
import type { IsoDate, Uuid } from './shared'

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

/** Les groupes vont du signalement ouvert le plus ancien au plus récent. */
export interface ExpertQueue {
  kind: ExpertQueueKind
  counts: ExpertQueueCounts
  reports: ExpertQueueReportGroup[]
}

export type FaqReportOutcome = 'revised' | 'confirmed' | 'dismissed'

/** `POST /admin/negotiation/queue/reports/{id}/close` — ne touche jamais l'entrée (FR-018). */
export interface AdminFaqReportCloseInput {
  outcome: FaqReportOutcome
}
