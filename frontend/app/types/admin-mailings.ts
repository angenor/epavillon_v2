/**
 * Envois de courriels du back-office (A13 bis, 28/10).
 *
 * Miroir de `engagement.mailings`, `engagement.mailing_recipients` et de
 * `engagement.mailing_audience()` (docs/database/110_engagement.sql § 12), et
 * des formes du crate `engagement` (`domain/mailing.rs`).
 */

import type { AttachedImage } from '~/types/media'
import type { EventId, Uuid } from '~/types/shared'

/** `engagement.mailing_status`. */
export type MailingStatus = 'draft' | 'queued' | 'sent' | 'cancelled'

/** `engagement.mailing_recipient_status`. */
export type MailingRecipientStatus = 'queued' | 'sending' | 'sent' | 'failed' | 'suppressed' | 'cancelled'

/**
 * Mise en page (`engagement.mailings.template`, `engagement.mail_templates.layout`) :
 * les modèles par défaut, et la base d'un gabarit personnalisé (29/10).
 */
export type MailingTemplate =
  | 'sobre'
  | 'institutionnel'
  | 'annonce'
  | 'banniere'
  | 'texte_sur_image'
  | 'institutionnel_logo'
  | 'edition'

/** Les modèles par défaut, dans l'ordre où la fenêtre de rédaction les propose. */
export const MAILING_TEMPLATES: MailingTemplate[] = [
  'sobre',
  'institutionnel',
  'annonce',
  'banniere',
  'texte_sur_image',
  'institutionnel_logo',
  'edition',
]

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
  template: MailingTemplate
  /** Valeur de secours par variable, pour qui n'a pas de compte. */
  variable_fallbacks: Record<string, string>
  reply_button: boolean
  cta_label: string | null
  cta_url: string | null
  /** Gabarit personnalisé ; `template` porte alors sa mise en page. */
  mail_template_id: Uuid | null
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
  /** Adresses joignables qui ne sont celles d'aucun compte. */
  without_account: number
  /** Destinataires qui recevront au moins une valeur de secours. */
  fallback_count: number
  /** Messages remis au serveur d'envoi, copies comprises. */
  envelope_count: number
  /** Gabarit de l'édition à présélectionner : l'envoi écrit à ses déposants. */
  suggested_mail_template_id: Uuid | null
  entries: MailingAudienceEntry[]
}

/** Le courriel tel qu'un destinataire le recevra, gabarit compris. */
export interface MailingPreview {
  to: string
  has_account: boolean
  subject: string
  html: string
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
  event_id?: EventId
  /** Un modèle par défaut ; seul, il retire le gabarit personnalisé. */
  template?: MailingTemplate
  /** Un gabarit personnalisé ; chaîne vide : aucun. */
  mail_template_id?: string
  variable_fallbacks?: Record<string, string>
  reply_button?: boolean
  /** Deux chaînes vides retirent le bouton d'action. */
  cta_label?: string
  cta_url?: string
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

/** « Plus d'options » de la fenêtre de rédaction. */
export interface ComposerOptionsValue {
  replyTo: string
  eventId: Uuid
  template: MailingTemplate
  /** Gabarit personnalisé choisi ; chaîne vide : un modèle par défaut. */
  customTemplateId: string
  fallbacks: Record<string, string>
  replyButton: boolean
  ctaLabel: string
  ctaUrl: string
}

// ---------------------------------------------------------------------------
// Gabarits personnalisés (29/10), `engagement.mail_templates`
// ---------------------------------------------------------------------------

/** Un gabarit personnalisé, tel que `/admin/mail-templates` le rend. */
export interface MailTemplate {
  id: Uuid
  name: string
  layout: MailingTemplate
  /** À défaut, le nom de l'édition de l'envoi. */
  title: string | null
  /** À défaut, les dates et le lieu de l'édition. */
  subtitle: string | null
  /** Voile sombre sous le texte posé sur l'image. */
  gradient: boolean
  /** Nul : commun à toutes les éditions. */
  event_id: EventId | null
  preselect_for_submitters: boolean
  created_by_name: string | null
  created_at: string
  updated_at: string
  /** Rôle `background` (6:1), tel que `media.attached_image()` le rend. */
  background: AttachedImage | null
  logo: AttachedImage | null
  /** Faux : commun à toutes les éditions, sans la permission globale. */
  can_edit: boolean
}

/** Écriture complète d'un gabarit. */
export interface MailTemplatePayload {
  name: string
  layout: MailingTemplate
  title: string | null
  subtitle: string | null
  gradient: boolean
  event_id: EventId | null
  preselect_for_submitters: boolean
}

/** L'aperçu d'un gabarit, rendu par la fonction du worker sur un texte d'exemple. */
export interface MailTemplatePreview {
  subject: string
  html: string
}
