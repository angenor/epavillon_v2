/**
 * Schéma `engagement`, partie « rappels » — dérivé de
 * `docs/database/110_engagement.sql` § 6.
 *
 * PÉRIMÈTRE VOLONTAIREMENT ÉTROIT. Le module Engagement compte notifications,
 * modèles de messages, courriels, commentaires publics, messagerie directe et
 * infolettres. Un seul de ces sujets est consommé par le jalon : le CALENDRIER
 * DES RAPPELS d'une séance, que l'espace organisation (A5) doit rendre. Le reste
 * viendra avec ses écrans, dans ce même fichier ou à côté — pas par avance.
 *
 * DEUX TABLES, ET LA DISTINCTION EST TOUT :
 *   `reminder_rules`      la POLITIQUE — ce que l'administrateur a programmé ;
 *   `scheduled_reminders` la MATÉRIALISATION — une ligne par destinataire et par
 *                         décalage, dont la clé unique interdit le double envoi.
 *
 * LES QUATRE DÉCALAGES SONT CUMULÉS, ce n'est pas un choix parmi quatre :
 * `{2 days, 1 day, 1 hour, 30 minutes}` est le défaut du modèle, et les quatre
 * rappels partent. L'écran qui n'en montrerait qu'un laisserait croire à un
 * réglage là où il y a une règle.
 */

import type {
  EventId,
  IsoDateTime,
  PersonId,
  RegistrationId,
  SessionId,
  Uuid,
} from './shared'

/** ENUM `engagement.notification_channel`. */
export type NotificationChannel = 'in_app' | 'email' | 'push'

/** ENUM `engagement.reminder_status`. */
export type ReminderStatus = 'pending' | 'queued' | 'sent' | 'skipped' | 'cancelled'

/**
 * Décalage avant le début, sérialisé depuis un `interval` PostgreSQL.
 *
 * En MINUTES et non en texte : `'1 day'` et `'24 hours'` sont le même intervalle
 * pour la base et deux chaînes différentes pour un `Map`, ce qui suffirait à
 * afficher deux fois le même rappel. L'écran formate ensuite selon la langue —
 * « 2 jours avant », « 30 minutes avant ».
 */
export type OffsetMinutes = number

/**
 * Table `engagement.reminder_rules` — `110` § 6.
 * Portée exclusive : soit une édition entière, soit une séance précise
 * (`ck_reminder_rules_scope`). Une règle de séance prend le pas sur celle de son
 * édition, sans cumul — pour que l'administrateur sache ce qui va partir.
 */
export interface ReminderRule {
  id: Uuid
  event_id: EventId | null
  session_id: SessionId | null
  /** Décalages CUMULÉS avant le début. Défaut du modèle : 2 j, 1 j, 1 h, 30 min. */
  offsets: OffsetMinutes[]
  channels: NotificationChannel[]
  /** Code de `engagement.notification_types`. */
  type_code: string
  template_id: Uuid | null
  is_active: boolean
  created_by: PersonId | null
  created_at: IsoDateTime
  updated_at: IsoDateTime
}

/**
 * Table `engagement.scheduled_reminders` — `110` § 6.
 * Une ligne par (séance, personne, canal, décalage) : la clé unique rend le
 * double envoi structurellement impossible, quel que soit le nombre de rejeux.
 */
export interface ScheduledReminder {
  id: Uuid
  rule_id: Uuid | null
  session_id: SessionId
  person_id: PersonId
  registration_id: RegistrationId | null
  channel: NotificationChannel
  offset_before: OffsetMinutes
  scheduled_for: IsoDateTime
  status: ReminderStatus
  job_id: Uuid | null
  sent_at: IsoDateTime | null
  /** `suppressed`, `channel_disabled`, `session_cancelled`. */
  skip_reason: string | null
  created_at: IsoDateTime
}
