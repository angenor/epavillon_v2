/**
 * RÉGIE DU JOUR (02/11) : contrats de l'écran de back-office `/admin/regie`.
 *
 * Dérivé de `docs/database/075_programme_sessions.sql` § 1 (colonnes
 * d'annulation et bornes du direct) et § 9 (`programme.session_presence`,
 * `programme.control_room_actions`). Les formes Rust vivent dans
 * `backend/crates/modules/programme/src/domain/regie.rs`.
 *
 * Chaque geste rend un `RegieGesture` : de quoi afficher le bandeau « Défaire »
 * (heure du courriel aux inscrits, fin de la reprise du direct).
 */

import type { I18nText, IsoDate, IsoDateTime, OrganizationId, TimeZoneName, Url, Uuid } from './shared'
import type { AttachedImage } from './media'
import type { SessionStatus } from './programme/session'

export type RegieActionKind = 'presence_confirmed' | 'session_cancelled' | 'live_started' | 'live_ended'

export type RegieJournalKind = RegieActionKind | 'incident_published'

export type RegiePresenceState = 'confirmed' | 'absent' | 'unknown'

export type RegieNoticeState = 'none' | 'pending' | 'sent' | 'withdrawn'

export interface RegieLiveElsewhere {
  session_id: Uuid
  /** Nul quand la séance est hors du périmètre du lecteur. */
  title: I18nText | null
  same_event: boolean
}

export interface RegieReason {
  code: string
  label: I18nText
  description: I18nText | null
}

export interface RegieOrganization {
  id: OrganizationId
  name: string
  acronym: string | null
  logo: AttachedImage | null
}

export interface RegiePresence {
  state: RegiePresenceState
  noted_at: IsoDateTime | null
  noted_by: string | null
}

export interface RegieMeeting {
  /** `live.meeting_status`. */
  status: string
  sync_status: string
  ready: boolean
  present_count: number | null
}

export interface RegieDocument {
  id: Uuid
  proposal_id: Uuid
  title: I18nText
  filename: string | null
  mime_type: string | null
  byte_size: number | null
  url: Url | null
  uploaded_at: IsoDateTime
}

export interface RegieNoticeStatus {
  state: RegieNoticeState
  due_at: IsoDateTime | null
  sent_at: IsoDateTime | null
  recipients: number
}

export interface RegieCancellation {
  cancelled_at: IsoDateTime | null
  reason_code: string | null
  reason_label: I18nText | null
  /** Note interne, jamais publiée. */
  note: string | null
  /** Nul pour une annulation venue du dossier : elle ne se rétablit pas d'ici. */
  action_id: Uuid | null
  notice: RegieNoticeStatus
}

export interface RegieSession {
  session_id: Uuid
  title: I18nText
  starts_at: IsoDateTime
  ends_at: IsoDateTime
  status: SessionStatus
  is_published: boolean
  is_streamed: boolean
  is_joint: boolean
  room_name: I18nText | null
  cover: AttachedImage | null
  organizations: RegieOrganization[]
  registrants: number
  presence: RegiePresence
  meeting: RegieMeeting | null
  documents: RegieDocument[]
  live_started_at: IsoDateTime | null
  live_ended_at: IsoDateTime | null
  resumable_until: IsoDateTime | null
  cancellation: RegieCancellation | null
  last_action_id: Uuid | null
}

export interface RegieJournalEntry {
  id: Uuid
  kind: RegieJournalKind
  session_id: Uuid | null
  session_title: I18nText | null
  session_starts_at: IsoDateTime | null
  incident_message: I18nText | null
  incident_kind_code: string | null
  acted_at: IsoDateTime
  actor_name: string | null
  actor_is_me: boolean
  undone_at: IsoDateTime | null
  undone_by: string | null
  can_undo: boolean
  notice: RegieNoticeStatus | null
}

export interface RegieScreen {
  event_id: Uuid
  event_title: I18nText
  event_acronym: string | null
  timezone: TimeZoneName
  zone_label: string | null
  days: IsoDate[]
  day: IsoDate
  today: IsoDate
  now: IsoDateTime
  live_elsewhere: RegieLiveElsewhere | null
  sessions: RegieSession[]
  journal: RegieJournalEntry[]
  reasons: RegieReason[]
}

export interface RegieCancelPayload {
  reason_code: string
  note: string | null
  notify_registrants: boolean
}

export interface RegieUndoPayload {
  notify_registrants: boolean
}

export interface RegieGesture {
  action_id: Uuid
  kind: RegieActionKind
  session_id: Uuid
  acted_at: IsoDateTime
  notify_at: IsoDateTime | null
  registrants: number
  resumable_until: IsoDateTime | null
  undone_at: IsoDateTime | null
}

/** `GET /admin/sessions/{id}/meeting/host` : lien d'animateur, jamais conservé. */
export interface MeetingHostLink {
  url: Url
}
