/**
 * Schéma `engagement` — dérivé de `docs/database/110_engagement.sql`
 * (§ 1 et 2 notifications, § 5 modèles de messages, § 6 rappels, § 7 suppression).
 *
 * QUATRE SUJETS, ET LEURS ÉCRANS NE SONT PAS AU MÊME STADE. Le CALENDRIER DES
 * RAPPELS d'une séance est consommé par l'espace organisation (A5). Les
 * NOTIFICATIONS, les MODÈLES DE MESSAGES et la LISTE DE SUPPRESSION sont servis
 * par l'API depuis B6 et attendent leurs écrans (B7) : leurs formes sont écrites
 * ici parce que l'API les NOMME, et qu'un nom sans définition ne documente rien.
 * Restent dehors, faute de route : commentaires publics, messagerie directe,
 * infolettres.
 *
 * LES RAPPELS : DEUX TABLES, ET LA DISTINCTION EST TOUT :
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
  Email,
  EventId,
  I18nText,
  IsoDateTime,
  PersonId,
  RegistrationId,
  SessionId,
  Slug,
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

// ---------------------------------------------------------------------------
// Notifications
// ---------------------------------------------------------------------------

/** ENUM `engagement.notification_criticality`. */
export type NotificationCriticality = 'critical' | 'important' | 'normal' | 'low'

/** Table `engagement.notifications` — `110` § 2. */
export interface Notification {
  id: Uuid
  /** Code de `engagement.notification_types`, grammaire `module.objet.fait`. */
  type_code: string
  /** Deux modes d'alimentation : soit le texte est figé ici au moment du fait,
   *  soit il se rend depuis `variables` et le modèle du type. */
  title: I18nText | null
  body: I18nText | null
  variables: Record<string, unknown>
  /** CHEMIN RELATIF Nuxt (contrainte `^/`), jamais une adresse absolue : les
   *  domaines de préproduction ne doivent pas fuiter dans les données. */
  link_path: string | null
  /** Les trois vont ensemble ou pas du tout (`num_nonnulls(…) IN (0, 3)`). */
  subject_schema: string | null
  subject_table: string | null
  subject_id: Uuid | null
  /** « 3 nouveaux commentaires » plutôt que trois lignes. Vaut 1 pour une
   *  notification seule. */
  group_count: number
  read_at: IsoDateTime | null
  created_at: IsoDateTime
}

/**
 * La liste ET le compte, dans la même réponse — `NotificationFeed`.
 * Deux appels donneraient deux chiffres mesurés à deux instants, et un badge qui
 * contredit la liste qu'il coiffe.
 */
export interface NotificationFeed {
  items: Notification[]
  /** TOUTES les non lues, jamais seulement celles de la page rendue. */
  unread_count: number
}

/**
 * Une ligne de l'écran des préférences — `NotificationPreferenceRow`.
 *
 * C'est LE CATALOGUE CROISÉ AVEC LES ARBITRAGES, canal par canal, jamais les
 * seuls arbitrages : l'absence de ligne enregistrée signifie « les canaux par
 * défaut du type », pas « aucun avis ».
 */
export interface NotificationPreferenceRow {
  type_code: string
  label: I18nText
  description: I18nText | null
  /** Code de `platform.modules` — c'est par lui que l'écran groupe ses lignes. */
  module_code: string
  criticality: NotificationCriticality
  channel: NotificationChannel
  is_enabled: boolean
  /** Faux pour un type critique : la préférence est bien enregistrée, mais
   *  l'expédition l'ignore. Sans ce champ, l'écran montrerait un interrupteur
   *  éteint pour un avis qui part quand même. */
  is_overridable: boolean
}

// ---------------------------------------------------------------------------
// Modèles de messages
// ---------------------------------------------------------------------------

/** Une ligne de la liste des modèles — `MessageTemplateRow`, table
 *  `engagement.message_templates` (`110` § 5) plus son décompte de révisions. */
export interface MessageTemplateRow {
  id: Uuid
  /** `platform.slug` — `registration-confirmed`, `session-reminder`. */
  key: Slug
  label: I18nText
  /** Code de `engagement.notification_types` ; nul pour une infolettre, qui ne
   *  sert aucun type. */
  type_code: string | null
  /** Nulle tant qu'aucune révision n'est publiée : le type part alors avec le
   *  texte de secours du module, et la trace d'expédition le dit. */
  current_version: number | null
  is_active: boolean
  /** Décompte, pas une colonne. */
  version_count: number
  updated_at: IsoDateTime
}

/** Table `engagement.template_versions` — `110` § 5. */
export interface TemplateVersion {
  id: Uuid
  template_id: Uuid
  version: number
  subject: I18nText
  /** ASSAINI À L'ÉCRITURE, langue par langue — jamais à l'affichage. */
  body_html: I18nText
  /** Repli texte brut, exigé par les bons clients de messagerie. */
  body_text: I18nText | null
  /** Variables attendues par le gabarit : `{{prenom}}`, `{{titre_session}}`… */
  variables: string[]
  published_at: IsoDateTime | null
  created_by: PersonId | null
  created_at: IsoDateTime
}

/** Le détail d'un modèle — `TemplateDetail`. */
export interface TemplateDetail {
  template: MessageTemplateRow
  /** De la plus récente à la plus ancienne. Rien n'est jamais effacé : publier
   *  fait avancer un pointeur, et republier une révision antérieure est le
   *  retour arrière. */
  versions: TemplateVersion[]
  /** La révision réellement servie. Nulle tant qu'aucune n'est publiée. */
  current: TemplateVersion | null
  /** `notification_types.expected_variables` du TYPE servi, pas du modèle : ce
   *  que l'émetteur s'engage à fournir, et ce contre quoi une publication est
   *  refusée. */
  promised_variables: string[]
}

// ---------------------------------------------------------------------------
// Délivrabilité
// ---------------------------------------------------------------------------

/** ENUM `engagement.suppression_reason`. */
export type SuppressionReason =
  | 'hard_bounce'
  | 'complaint'
  | 'unsubscribe'
  | 'invalid_address'
  | 'manual'

/**
 * Une adresse écartée du circuit — `EmailSuppression`, table
 * `engagement.email_suppressions`. Elle se consulte AVANT toute mise en file, et
 * vaut pour tous les modules émetteurs.
 */
export interface EmailSuppression {
  email: Email
  reason: SuppressionReason
  detail: string | null
  /** Nulle = définitive. Une valeur lève d'elle-même une suppression temporaire
   *  — une boîte pleine — sans intervention. */
  expires_at: IsoDateTime | null
  suppressed_at: IsoDateTime
  suppressed_by: PersonId | null
}
