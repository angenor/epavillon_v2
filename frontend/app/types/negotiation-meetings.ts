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

// ---------------------------------------------------------------------------
// Back-office — `/admin/negotiation/meetings…` (`negotiation.meeting.manage`, portée globale)
// ---------------------------------------------------------------------------

/** `ongoing` et `completed` ne sont posés par aucun geste d'ici. */
export type AdminMeetingStatus = 'draft' | 'scheduled' | 'ongoing' | 'completed' | 'cancelled'

export interface AdminFrancophoneMeeting {
  id: Uuid
  slug: string
  edition: string
  /** Fuseau de l'édition, recopié à la saisie. */
  timezone: string
  /** Nul pour un brouillon sans nature. */
  type: { code: string; label: I18nText } | null
  title: I18nText
  description: I18nText | null
  start_at: IsoDateTime
  end_at: IsoDateTime
  format: FrancophoneMeetingFormat
  venue: string | null
  external_url: string | null
  capacity: number | null
  waitlist_enabled: boolean
  requires_registration: boolean
  registration_opens_at: IsoDateTime | null
  registration_closes_at: IsoDateTime | null
  open_access: boolean
  access_audience: I18nText | null
  is_ifdd_organized: boolean
  organizer_org_id: Uuid | null
  /** « IFDD » ou le nom de l'organisation. */
  organizer: string
  status: AdminMeetingStatus
  cancellation_reason: string | null
  pavilion_session_id: Uuid | null
  registered_count: number
  waitlisted_count: number
  updated_at: IsoDateTime
}

/** `GET /admin/negotiation/meetings?edition=` — brouillons compris, par début. */
export interface AdminFrancophoneMeetings {
  edition: { slug: string; timezone: string; city: string | null }
  meetings: AdminFrancophoneMeeting[]
}

/** Corps de `POST` et `PUT` ; `edition` n'est lue qu'à la création. */
export interface FrancophoneMeetingInput {
  edition?: string
  type: string | null
  title: I18nText
  description: I18nText | null
  start_at: IsoDateTime
  end_at: IsoDateTime
  format: FrancophoneMeetingFormat
  venue: string | null
  external_url: string | null
  capacity: number | null
  waitlist_enabled: boolean
  requires_registration: boolean
  registration_opens_at: IsoDateTime | null
  registration_closes_at: IsoDateTime | null
  open_access: boolean
  access_audience: I18nText | null
  is_ifdd_organized: boolean
  organizer_org_id: Uuid | null
}

export interface CancelMeetingPayload {
  reason: string
}

export interface MeetingPavilionPayload {
  pavilion_session_id: Uuid | null
}

export interface AdminMeetingRegistrant {
  person_id: Uuid
  name: string
  country: I18nText | null
  registered_at: IsoDateTime
  /** Posée pour la liste d'attente seulement. */
  waitlist_position: number | null
}

/** `GET /admin/negotiation/meetings/{id}/registrations`. */
export interface AdminMeetingRegistrations {
  registered: AdminMeetingRegistrant[]
  waitlisted: AdminMeetingRegistrant[]
}

/** `GET /admin/negotiation/pavilion-activities?edition=` — lu dans `programme.sessions`. */
export interface PavilionActivityOption {
  id: Uuid
  title: I18nText
  starts_at: IsoDateTime
}
