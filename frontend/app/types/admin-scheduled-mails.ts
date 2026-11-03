/**
 * COURRIELS PROGRAMMÉS (04/11) : l'onglet « Programmés » de Courriels.
 *
 * Miroir de `engagement::domain::programmes` et de
 * `engagement.scheduled_email_texts` (110_engagement.sql). Un type est un
 * rappel de journée (tous décalages) ou un rappel d'activité à un décalage ;
 * un envoi réunit les destinataires d'un même rappel pour une même cible à la
 * même heure.
 */

import type { EventId, Uuid } from './shared'

/** `activity` : rappel d'une activité ; `day` : rappel d'une journée. */
export type ScheduledMailKind = 'activity' | 'day'

export type ScheduledMailLocale = 'fr' | 'en'

/** Un objet et un message, texte brut (une ligne vide sépare deux paragraphes). */
export interface ScheduledMailText {
  subject: string
  body: string
}

export interface ScheduledMailTexts {
  fr: ScheduledMailText
  en: ScheduledMailText
}

export interface ScheduledMailBatchRow {
  /** `activity.<séance>.<décalage>` ou `day.<journée>.<décalage>.<début>`. */
  key: string
  /** `day` ou `activity.<décalage>`. */
  type_key: string
  kind: ScheduledMailKind
  /** En minutes avant le début. */
  offset_minutes: number
  target_id: Uuid
  /** Début de la journée visé ; nul pour une activité. */
  target_starts_at: string | null
  /** L'activité ou la journée, dans la langue de l'écran. */
  title: string
  run_at: string
  recipients: number
  has_own_text: boolean
}

export interface ScheduledMailType {
  key: string
  kind: ScheduledMailKind
  /** Nul pour le rappel de journée, qui couvre tous ses décalages. */
  offset_minutes: number | null
  count: number
  next_run_at: string
  has_custom_text: boolean
  batches: ScheduledMailBatchRow[]
}

export interface ScheduledMailScreen {
  /** L'heure du serveur au relevé : le décompte s'y cale. */
  observed_at: string
  total: number
  next: ScheduledMailBatchRow | null
  types: ScheduledMailType[]
}

export interface ScheduledMailBatch {
  observed_at: string
  batch: ScheduledMailBatchRow
  /** Les courriels de tout le type, dans l'édition. */
  type_recipients: number
  /** Le texte du type, écrit ou par défaut. */
  type_text: ScheduledMailTexts
  type_text_is_custom: boolean
  /** Nul : l'envoi suit le texte du type. */
  own_text: ScheduledMailTexts | null
  variables: string[]
  /** La langue du premier destinataire. */
  preview_locale: ScheduledMailLocale
}

export interface ScheduledMailTypeTextPayload extends ScheduledMailTexts {
  event_id: EventId
}

export interface ScheduledMailPreviewPayload extends ScheduledMailText {
  locale: ScheduledMailLocale
}

export interface ScheduledMailPreview {
  locale: ScheduledMailLocale
  subject: string
  /** Le courriel complet, valeurs surlignées. */
  html: string
}
