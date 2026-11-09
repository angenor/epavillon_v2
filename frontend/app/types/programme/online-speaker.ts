/**
 * Les intervenants en ligne d'une activité diffusée : inscrits par la régie ou
 * par le lien d'inscription transmis au porteur, chacun reçoit son lien Zoom
 * personnel. Le jeton du lien voyage dans l'en-tête `X-Speaker-Link`, jamais
 * dans l'adresse d'une route de l'API.
 */

import type { Civility } from '../identity'
import type { I18nText, IsoDateTime, TimeZoneName, Uuid } from '../shared'

export type SpeakerSource = 'dossier' | 'regie' | 'lien'

export interface OnlineSpeakerInput {
  civility: Civility | null
  first_name: string
  last_name: string
  job_title: string
  organization: string
  email: string
  country_iso2: string | null
  bio: string | null
  /** La régie envoie `true` : c'est l'IFDD qui ajoute la personne. */
  consent: boolean
}

export interface OnlineSpeakerCreated {
  speaker_id: Uuid
}

export interface SpeakerLinkInvite {
  session_id: Uuid
  title: I18nText
  event_title: I18nText
  starts_at: IsoDateTime
  ends_at: IsoDateTime
  timezone: TimeZoneName
  city: string | null
}

export interface RegieSpeaker {
  speaker_id: Uuid
  display_name: string
  job_title: string | null
  organization: string | null
  email: string | null
  source: SpeakerSource
  is_listed: boolean
  created_at: IsoDateTime
  link_sent_at: IsoDateTime | null
}

export interface RegieSpeakers {
  session_id: Uuid
  speakers: RegieSpeaker[]
  signup_url: string
  owner_emails: string[]
  owner_link_sent_at: IsoDateTime | null
}

export interface SpeakerLinksSent {
  recipients: number
}

export interface SpeakerZoomAccess {
  join_url: string
}
