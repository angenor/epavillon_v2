/**
 * Envois de courriels du back-office (A13 bis, 28/10).
 *
 * Miroir de `engagement.mailings`, `engagement.mailing_recipients` et de
 * `engagement.mailing_audience()` (docs/database/110_engagement.sql § 12), et
 * des formes du crate `engagement` (`domain/mailing.rs`).
 */

import type { EventId, Uuid } from '~/types/shared'

/** `engagement.mailing_status`. */
export type MailingStatus = 'draft' | 'queued' | 'sent' | 'cancelled'

/** `engagement.mailing_recipient_status`. */
export type MailingRecipientStatus = 'queued' | 'sending' | 'sent' | 'failed' | 'suppressed' | 'cancelled'

/** Les familles de destinataires lues par `mailing_audience()`. */
export type MailingCriterionKind = 'submitters' | 'speakers' | 'registrants' | 'roles'

/** D'où vient une adresse : un critère, ou la saisie libre. */
export type MailingSource = MailingCriterionKind | 'addresses'

export interface MailingCriterion {
  kind: MailingCriterionKind
  /** Vide : tous les dossiers sauf les brouillons. */
  proposal_statuses: string[]
  /** Vide : inscrits, en attente et présents. */
  registration_statuses: string[]
  /** Vide : tous les rôles attribués sur l'édition. */
  role_codes: string[]
  session_id: Uuid | null
}

export interface MailingCounts {
  total: number
  queued: number
  sending: number
  sent: number
  failed: number
  suppressed: number
  cancelled: number
}

export interface MailingSummary {
  id: Uuid
  event_id: EventId
  status: MailingStatus
  subject: string
  created_by_name: string | null
  queued_by_name: string | null
  queued_at: string | null
  completed_at: string | null
  created_at: string
  updated_at: string
  counts: MailingCounts
  attachment_count: number
}

export interface MailingListScreen {
  rows: MailingSummary[]
  team_cc: string[]
  variables: string[]
}

export interface MailingRecipient {
  id: Uuid
  email: string
  person_id: Uuid | null
  first_name: string | null
  last_name: string | null
  organization: string | null
  sources: MailingSource[]
  status: MailingRecipientStatus
  last_error: string | null
  sent_at: string | null
}

export interface Mailing {
  id: Uuid
  event_id: EventId
  status: MailingStatus
  subject: string
  /** HTML assaini par l'API. */
  body_html: string
  audience: MailingCriterion[]
  extra_addresses: string[]
  cc: string[]
  bcc: string[]
  reply_to: string | null
  duplicated_from: Uuid | null
  created_by_name: string | null
  queued_by_name: string | null
  queued_at: string | null
  completed_at: string | null
  cancelled_at: string | null
  created_at: string
  updated_at: string
  counts: MailingCounts
  /** Vide tant que l'envoi est un brouillon. */
  recipients: MailingRecipient[]
  attachments: MailingAttachment[]
  attachments_total_bytes: number
  /** `media.attachable_roles.max_total_byte_size` : nul, aucun plafond. */
  attachments_max_bytes: number | null
}

/** Une pièce jointe : rattachement `attachment` de `media.attachments`, objet privé. */
export interface MailingAttachment {
  attachment_id: Uuid
  asset_id: Uuid
  filename: string
  mime_type: string
  byte_size: number
  /** `media.asset_status` : seul `ready` peut partir. */
  status: string
}

export interface MailingAudienceEntry {
  email: string
  person_id: Uuid | null
  first_name: string | null
  last_name: string | null
  organization: string | null
  sources: MailingSource[]
  is_suppressed: boolean
}

export interface MailingAudience {
  total: number
  sendable: number
  suppressed: number
  without_first_name: number
  /** Messages remis au serveur d'envoi, copies comprises. */
  envelope_count: number
  entries: MailingAudienceEntry[]
}

export interface MailingTestResult {
  to: string
}

/** Écriture partielle d'un brouillon : un champ absent reste tel quel. */
export interface MailingPayload {
  subject?: string
  body_html?: string
  audience?: MailingCriterion[]
  extra_addresses?: string[]
  cc?: string[]
  bcc?: string[]
  /** Chaîne vide : retirer l'adresse de réponse. */
  reply_to?: string
}

export interface CreateMailingPayload extends MailingPayload {
  event_id: EventId
}

/** Une ligne de la liste des destinataires, avant l'envoi comme après. */
export interface MailingRecipientRow {
  email: string
  name: string | null
  organization: string | null
  sources: MailingSource[]
  /** Nul avant l'envoi. */
  status: MailingRecipientStatus | null
  suppressed: boolean
  error: string | null
  sentAt: string | null
}
