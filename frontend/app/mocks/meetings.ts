/**
 * La réunion Zoom d'une séance, sans API ni clés Zoom.
 *
 * Toute séance diffusée du Pavillon (`pavillon.ts`, heures posées autour de
 * maintenant) et du jeu du site a sa réunion : la salle s'ouvre 30 minutes avant
 * le début et ferme 30 minutes après la fin. L'accès, lui, échoue toujours comme
 * l'API sans configuration Zoom (503 `LIVE_ZOOM_UNAVAILABLE`) : on ne fabrique pas
 * de signature.
 */
import type { AdminSessionMeeting, MeetingAccess, MeetingStatus, MeetingSyncInput, PublicSessionMeeting, RegistrationJoinResult } from '~/types/live'
import type { ApiErrorCode } from '~/types/api-error'
import type { PublicScheduleRow } from '~/types/views'
import { ApiRequestError } from '~/utils/api-error'
import { programme } from './pavillon'
import { allSessions } from './sessions'

const MARGIN_MS = 30 * 60_000

type Horaire = Pick<PublicScheduleRow, 'id' | 'starts_at' | 'ends_at' | 'is_streamed' | 'status'>

function seance(sessionId: string): Horaire | null {
  return programme('').valeur.find((s) => s.id === sessionId) ?? allSessions.find((s) => s.id === sessionId) ?? null
}

function statut(s: Horaire, at: number): MeetingStatus {
  if (s.status === 'cancelled') return 'cancelled'
  if (at >= Date.parse(s.ends_at)) return 'ended'
  return at >= Date.parse(s.starts_at) ? 'started' : 'scheduled'
}

const numero = (sessionId: string) => `8${sessionId.replace(/\D/g, '').slice(-9).padStart(9, '0')}`

export function publicSessionMeeting(sessionId: string, at: number = Date.now()): PublicSessionMeeting | null {
  const s = seance(sessionId)
  if (!s?.is_streamed) return null
  return {
    provider: 'zoom',
    status: statut(s, at),
    opens_at: new Date(Date.parse(s.starts_at) - MARGIN_MS).toISOString(),
    starts_at: s.starts_at,
    ends_at: s.ends_at,
    closes_at: new Date(Date.parse(s.ends_at) + MARGIN_MS).toISOString(),
  }
}

export function meetingAccess(_sessionId: string): MeetingAccess {
  const code: ApiErrorCode = 'LIVE_ZOOM_UNAVAILABLE'
  throw new ApiRequestError(
    { code, message: "La réunion Zoom est momentanément indisponible. Réessayez dans quelques instants ou ouvrez-la dans l'application Zoom." },
    503,
  )
}

export function registrationJoined(_registrationId: string): RegistrationJoinResult {
  return { joined_at: new Date().toISOString() }
}

const reunions = new Map<string, AdminSessionMeeting>()

export function adminSessionMeeting(sessionId: string): AdminSessionMeeting | null {
  const deja = reunions.get(sessionId)
  if (deja) return deja
  const s = seance(sessionId)
  if (!s?.is_streamed) return null
  return {
    id: `01990000-0000-7000-8000-0000000f${sessionId.slice(-4)}`,
    session_id: sessionId,
    provider: 'zoom',
    external_id: numero(sessionId),
    status: statut(s, Date.now()),
    sync_status: 'synced',
    last_sync_error: null,
    join_url: `https://zoom.us/j/${numero(sessionId)}`,
    participants_total: 42,
    participants_synced: 41,
    participants_failed: 1,
  }
}

export function syncSessionMeeting(sessionId: string, input: MeetingSyncInput): AdminSessionMeeting {
  const base = adminSessionMeeting(sessionId)
  const externalId = input.external_id?.replace(/\s/g, '') || base?.external_id || numero(sessionId)
  const reunion: AdminSessionMeeting = {
    id: base?.id ?? `01990000-0000-7000-8000-0000000f${sessionId.slice(-4)}`,
    session_id: sessionId,
    provider: 'zoom',
    external_id: externalId,
    status: base?.status ?? 'scheduled',
    sync_status: 'pending',
    last_sync_error: null,
    join_url: `https://zoom.us/j/${externalId}`,
    participants_total: base?.participants_total ?? 0,
    participants_synced: 0,
    participants_failed: 0,
  }
  reunions.set(sessionId, reunion)
  return reunion
}
