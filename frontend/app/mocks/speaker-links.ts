/**
 * Le lien d'inscription des intervenants en ligne, hors ligne : le jeton
 * « exemple » ouvre une activité de la CdP31 demain à 11:30, heure d'Antalya ;
 * tout autre jeton est un lien expiré.
 */
import type { OnlineSpeakerCreated, OnlineSpeakerInput, SpeakerLinkInvite, SpeakerZoomAccess } from '~/types/programme/online-speaker'
import type { Uuid } from '~/types/shared'
import { ApiRequestError } from '~/utils/api-error'
import { instantFromWallClock, dayKeyInZone } from '~/utils/datetime'
import { events } from './event'
import { TRACK_PAGE } from './ids'

export const SPEAKER_LINK_SAMPLE_TOKEN = 'exemple'
const SAMPLE_ZOOM_URL = 'https://us06web.zoom.us/w/0000000000?tk=exemple'

let sequence = 0

function expired(): never {
  throw new ApiRequestError({ code: 'NOT_FOUND', message: "Ce lien n'est plus valable." }, 404)
}

export function speakerLinkInvite(token: string): SpeakerLinkInvite | null {
  if (token !== SPEAKER_LINK_SAMPLE_TOKEN) return null
  const event = events.find((entry) => entry.id === TRACK_PAGE.edition)
  const timezone = event?.timezone ?? 'Europe/Istanbul'
  const tomorrow = dayKeyInZone(Date.now() + 24 * 3_600_000, timezone)
  const starts = instantFromWallClock(`${tomorrow}T11:30`, timezone)
  const ends = instantFromWallClock(`${tomorrow}T12:30`, timezone)
  return {
    session_id: '01a05300-0000-7000-8000-000000000001',
    title: { fr: "Financer l'adaptation des villes côtières", en: 'Financing adaptation in coastal cities' },
    event_title: event?.title ?? { fr: 'CdP31, Antalya', en: 'COP31, Antalya' },
    starts_at: starts ?? new Date().toISOString(),
    ends_at: ends ?? new Date().toISOString(),
    timezone,
    city: event?.city ?? 'Antalya',
  }
}

export function speakerLinkRegister(token: string, _input: OnlineSpeakerInput): OnlineSpeakerCreated {
  if (token !== SPEAKER_LINK_SAMPLE_TOKEN) expired()
  return { speaker_id: `01a05400-0000-7000-8000-${String(++sequence).padStart(12, '0')}` }
}

export function speakerLinkZoom(token: string, _speakerId: Uuid): SpeakerZoomAccess {
  if (token !== SPEAKER_LINK_SAMPLE_TOKEN) expired()
  return { join_url: SAMPLE_ZOOM_URL }
}

export function speakerZoomAccess(_speakerId: Uuid): SpeakerZoomAccess {
  return { join_url: SAMPLE_ZOOM_URL }
}
