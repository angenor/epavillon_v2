/**
 * La réunion Zoom d'une séance : sa part de `useApi()`.
 *
 * Trois routes en `pending` tant que le crate `live` ne les sert pas : à basculer
 * vers `callOrNull` (les deux lectures, un 404 = pas de réunion), `call` (l'accès)
 * et `send` (la synchronisation) dès qu'elles figurent au contrat.
 */
import type {
  AdminSessionMeeting,
  MeetingAccess,
  MeetingSyncInput,
  PublicSessionMeeting,
  RegistrationJoinResult,
} from '~/types/live'
import type { Uuid } from '~/types/shared'
import type { ApiTransport } from './proposal-review'

export function createMeetingsApi({ send, pending }: Pick<ApiTransport, 'send' | 'pending'>) {
  return {
    /** Public. Nul : la séance n'a pas de réunion. */
    forSession: (sessionId: Uuid): Promise<PublicSessionMeeting | null> =>
      pending(`/sessions/${sessionId}/meeting`, (m) => m.publicSessionMeeting(sessionId), 'read'),

    /**
     * La signature d'entrée, pour l'inscrit ou la personne en liste d'attente. Refus :
     * 401, 403 `LIVE_MEETING_REGISTRATION_REQUIRED`, 409 `LIVE_MEETING_NOT_OPEN` ou
     * `LIVE_MEETING_NOT_READY`, 503 `LIVE_ZOOM_UNAVAILABLE`.
     */
    access: (sessionId: Uuid): Promise<MeetingAccess> =>
      pending(`/sessions/${sessionId}/meeting/access`, (m) => m.meetingAccess(sessionId), 'read'),

    /** La première présence ; un second appel ne l'écrase pas. */
    recordJoin: (registrationId: Uuid): Promise<RegistrationJoinResult> =>
      send(`/registrations/${registrationId}/join`, {}, (m) => m.registrationJoined(registrationId)),

    /** Back-office. Nul : aucune réunion. La portée est vérifiée par l'API, sur l'édition de la séance. */
    adminForSession: (sessionId: Uuid): Promise<AdminSessionMeeting | null> =>
      pending(`/admin/sessions/${sessionId}/meeting`, (m) => m.adminSessionMeeting(sessionId), 'read'),

    /** Crée la réunion chez Zoom, ou la resynchronise ; `external_id` rattache une réunion existante. */
    sync: (sessionId: Uuid, input: MeetingSyncInput): Promise<AdminSessionMeeting> =>
      pending(`/admin/sessions/${sessionId}/meeting`, (m) => m.syncSessionMeeting(sessionId, input), 'write'),
  }
}
