/**
 * Les signalements du réseau sur les sessions officielles de Guide Négo.
 * Champ pour champ comme `negotiation/src/domain/reports.rs` ; contrat dans
 * `specs/015-guide-nego-signalements/contracts/api-signalements.md`.
 */

import type { IsoDate, IsoDateTime, Uuid } from './shared'

export type ReportReason = 'cancelled' | 'time' | 'venue' | 'other' | 'unannounced'

// ---------------------------------------------------------------------------
// `POST /negotiation/reports`
// ---------------------------------------------------------------------------

export interface ReportPayload {
  /** Posé par le téléphone : un envoi rejoué rend le même signalement. */
  client_ref: Uuid
  /** Slug de la COP. */
  edition: string
  reason: ReportReason
  /** Requis sauf `unannounced`. */
  session_id?: Uuid
  /** `time`, et début d'une `unannounced`. */
  proposed_start?: IsoDateTime
  /** `venue`, et « Où » d'une `unannounced`. */
  proposed_venue?: string
  /** Requis pour `unannounced`. */
  what?: string
  /** AAAA-MM-JJ, requis pour `unannounced`. */
  day?: IsoDate
  theme?: string | null
  /** 600 caractères au plus. */
  detail?: string
}

// ---------------------------------------------------------------------------
// `GET /negotiation/me/reports?edition=`
// ---------------------------------------------------------------------------

export type RejectReason = 'source_maintains' | 'already_known' | 'not_precise'

export interface MyReport {
  id: Uuid
  client_ref: Uuid
  reason: ReportReason
  session: {
    id: Uuid
    title_en: string
    title_fr: string | null
    start_at: IsoDateTime
    venue: string | null
  } | null
  what: string | null
  proposed_start: IsoDateTime | null
  proposed_venue: string | null
  day: IsoDate | null
  detail: string | null
  /** `validated` seulement une fois publié ; avant, `submitted`. */
  status: 'submitted' | 'validated' | 'rejected'
  submitted_at: IsoDateTime
  decided_at: IsoDateTime | null
  reject_reason: RejectReason | null
  reject_detail: string | null
}

/** Plus récent d'abord. */
export interface MyReports {
  reports: MyReport[]
}
