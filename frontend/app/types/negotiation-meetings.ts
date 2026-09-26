/**
 * Les réunions de la Francophonie de Guide Négo, et l'inscription. Champ pour
 * champ comme `negotiation/src/domain/{meetings,meeting_registrations}.rs` ;
 * contrat dans `specs/016-guide-nego-reunions/contracts/api-reunions.md`.
 *
 * Toute heure est un instant ISO 8601 en UTC ; le fuseau de la COP voyage à part.
 */

import type { I18nText, IsoDateTime, Uuid } from './shared'

// ---------------------------------------------------------------------------
// `GET /negotiation/meetings?edition=`
// ---------------------------------------------------------------------------

export interface FrancophoneMeetings {
  edition: { slug: string; timezone: string; city: string | null }
  read_at: IsoDateTime
  server_time: IsoDateTime
  /** Publiées ; un brouillon jamais. */
  meetings: FrancophoneMeeting[]
}

export type FrancophoneMeetingFormat = 'onsite' | 'online' | 'hybrid'

export interface FrancophoneMeeting {
  id: Uuid
  /** Vocabulaire `francophone_meeting_type`. */
  type: { code: string; label: I18nText }
  title: I18nText
  description: I18nText | null
  start_at: IsoDateTime
  end_at: IsoDateTime
  format: FrancophoneMeetingFormat
  venue: string | null
  /** Jamais le lien ici : il vient de `MyMeetingRegistrations.video`. */
  has_video: boolean
  /** « IFDD » ou le nom de l'organisation. */
  organizer: string
  open_access: boolean
  access_audience: I18nText | null
  requires_registration: boolean
  capacity: number | null
  registered_count: number
  waitlist_enabled: boolean
  /** Nulle : sans limite. */
  registration_opens_at: IsoDateTime | null
  registration_closes_at: IsoDateTime | null
  /** « Terminée » : `end_at` passé. */
  status: 'scheduled' | 'cancelled'
  cancellation_reason: string | null
  pavilion_session_id: Uuid | null
}

// ---------------------------------------------------------------------------
// `GET /negotiation/me/meeting-registrations?edition=`
// ---------------------------------------------------------------------------

export type MeetingRegistrationStatus = 'registered' | 'waitlisted' | 'cancelled'

export interface MeetingRegistration {
  meeting_id: Uuid
  status: Exclude<MeetingRegistrationStatus, 'cancelled'>
  waitlist_position: number | null
  client_ref: Uuid | null
  registered_at: IsoDateTime
}

export interface MyMeetingRegistrations {
  registrations: MeetingRegistration[]
  /** Inscrites ; réunions sans inscription pour qui a l'accès. Jamais la liste d'attente. */
  video: { meeting_id: Uuid; url: string }[]
}

// ---------------------------------------------------------------------------
// `PUT /negotiation/me/meeting-registrations/{meeting_id}`
// ---------------------------------------------------------------------------

export interface MeetingRegistrationPayload {
  /** Neuf à chaque geste : un envoi rejoué rend l'état courant. */
  client_ref: Uuid
}

export interface MeetingRegistrationState {
  /** `cancelled` : rejeu d'un geste suivi d'une désinscription. */
  status: MeetingRegistrationStatus
  waitlist_position: number | null
}
