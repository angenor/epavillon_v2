/** La réunion Zoom d'une séance : sa part de `useApi()`. Un 404 en lecture veut dire « pas de réunion ». */
import type {
  AdminSessionMeeting,
  MeetingAccess,
  MeetingSyncInput,
  PublicSessionMeeting,
  RegistrationJoinResult,
} from '~/types/live'
import type { Uuid } from '~/types/shared'
import type { ApiTransport } from './proposal-review'

export function createMeetingsApi({ call, callOrNull, send }: Pick<ApiTransport, 'call' | 'callOrNull' | 'send'>) {
  return {
    /** Public. Nul : la séance n'a pas de réunion. */
    forSession: (sessionId: Uuid): Promise<PublicSessionMeeting | null> =>
      callOrNull(`/sessions/${sessionId}/meeting`, (m) => m.publicSessionMeeting(sessionId)),

    /**
     * La signature d'entrée, pour l'inscrit ou la personne en liste d'attente. Refus :
     * 401, 403 `LIVE_MEETING_REGISTRATION_REQUIRED`, 409 `LIVE_MEETING_NOT_OPEN` ou
     * `LIVE_MEETING_NOT_READY`, 503 `LIVE_ZOOM_UNAVAILABLE`.
     */
    access: (sessionId: Uuid): Promise<MeetingAccess> =>
      call(`/sessions/${sessionId}/meeting/access`, (m) => m.meetingAccess(sessionId)),

    /** La première présence ; un second appel ne l'écrase pas. */
    recordJoin: (registrationId: Uuid): Promise<RegistrationJoinResult> =>
      send(`/registrations/${registrationId}/join`, {}, (m) => m.registrationJoined(registrationId)),

    /** Back-office. Nul : aucune réunion. La portée est vérifiée par l'API, sur l'édition de la séance. */
    adminForSession: (sessionId: Uuid): Promise<AdminSessionMeeting | null> =>
      callOrNull(`/admin/sessions/${sessionId}/meeting`, (m) => m.adminSessionMeeting(sessionId)),

    /** Crée la réunion chez Zoom, ou la resynchronise ; `external_id` rattache une réunion existante. */
    sync: (sessionId: Uuid, input: MeetingSyncInput): Promise<AdminSessionMeeting> =>
      send(`/admin/sessions/${sessionId}/meeting`, input, (m) => m.syncSessionMeeting(sessionId, input)),
  }
}
