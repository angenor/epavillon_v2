/**
 * L'inscription à une journée spéciale (fil `special_day`) ou à l'atelier
 * préparatoire (`side_programme`) : un seul direct pour toute la journée,
 * moments compris ; les activités gardées ne servent qu'aux rappels et aux
 * statistiques, jamais à l'accès.
 *
 * Sans compte, la personne gère son inscription par le lien reçu par courriel :
 * son jeton voyage dans l'en-tête `X-Manage-Token`, jamais dans l'adresse d'une
 * route de l'API.
 */

import type { I18nText, IsoDateTime, OrganizationId, SessionId, TrackId, Uuid } from '../shared'
import type { RegistrationGuest } from './registration'

export type DayRegistrationStatus = 'registered' | 'cancelled' | 'attended' | 'no_show'

export interface DayRegistration {
  id: Uuid
  track_id: TrackId
  status: DayRegistrationStatus
  /** Activités gardées. */
  session_ids: SessionId[]
  /** M de « N activités sur M » : activités publiées et non annulées de la journée. */
  activity_count: number
  created_at: IsoDateTime
  joined_at: IsoDateTime | null
}

/** Trois issues en 200 : inscrite, déjà inscrite, journée close. */
export type DayRegistrationResult =
  | { status: 'registered'; registration: DayRegistration }
  /** Nulle pour une personne sans compte : l'API ne rend rien et renvoie le lien par courriel. */
  | { status: 'already_registered'; registration: DayRegistration | null }
  | { status: 'closed'; closed_at: IsoDateTime }

export interface DayRegisterPayload {
  track_id: TrackId
  session_ids: SessionId[]
  answers: Record<string, unknown>
  locale?: string
  guest?: RegistrationGuest
  sensitive_data_consent?: boolean
  organization_id?: OrganizationId | null
}

export interface DaySessionsPayload {
  session_ids: SessionId[]
}

/** `GET /track-registrations/manage` : l'inscription du porteur du lien, et de quoi nommer la journée. */
export interface DayRegistrationManage {
  registration: DayRegistration
  event_slug: string
  track_slug: string
  track_title: I18nText
  first_name: string
}

/** Une ligne de la liste nominative du back-office. */
export interface DayRegistrationRow {
  id: Uuid
  status: DayRegistrationStatus
  person_id: Uuid
  first_name: string
  last_name: string
  email: string
  organization_name: string | null
  has_account: boolean
  kept_count: number
  created_at: IsoDateTime
  joined_at: IsoDateTime | null
}
