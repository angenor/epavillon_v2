/**
 * L'inscription à une journée spéciale ou à l'atelier : sa part de `useApi()`.
 *
 * `token` est le jeton du lien de gestion reçu par courriel, pour qui s'est
 * inscrit sans compte. Il part dans l'en-tête `X-Manage-Token`, jamais dans
 * l'adresse : une adresse se retrouve dans les journaux et les en-têtes `Referer`.
 */
import type {
  DayRegisterPayload,
  DayRegistration,
  DayRegistrationManage,
  DayRegistrationResult,
  DayRegistrationRow,
  DaySessionsPayload,
} from '~/types/programme/day-registration'
import type { ApplicableRegistrationForm } from '~/types/programme/registration'
import type { Uuid } from '~/types/shared'
import type { ApiHeaders, ApiTransport } from './proposal-review'

const exemples = () => import('~/mocks/day-registrations')

export const manageHeaders = (token: string | null | undefined): ApiHeaders | undefined =>
  token ? { 'X-Manage-Token': token } : undefined

export function createTrackRegistrationsApi({ call, callOrNull, send }: Pick<ApiTransport, 'call' | 'callOrNull' | 'send'>) {
  return {
    /** Nul : journée inconnue ou non publiée. */
    form: (trackId: Uuid): Promise<ApplicableRegistrationForm | null> =>
      callOrNull('/track-registrations/form', async () => (await exemples()).dayForm(trackId), { track_id: trackId }),

    /** Trois issues en 200 ; 401 `REGISTRATION_ACCOUNT_REQUIRED`, 422 champ nommé. */
    register: (payload: DayRegisterPayload): Promise<DayRegistrationResult> =>
      send('/track-registrations', payload, async () => (await exemples()).dayRegister(payload)),

    /** Nulle : pas inscrite. */
    mine: (trackId: Uuid): Promise<DayRegistration | null> =>
      callOrNull('/track-registrations/mine', async () => (await exemples()).dayMine(trackId), { track_id: trackId }),

    /** Nulle : jeton inconnu, ou remplacé par un envoi plus récent. */
    manage: (token: string): Promise<DayRegistrationManage | null> =>
      callOrNull('/track-registrations/manage', async () => (await exemples()).dayManage(token), undefined, manageHeaders(token)),

    /** 422 `REGISTRATION_LOCKED` : l'inscription est annulée. */
    updateSessions: (registrationId: Uuid, payload: DaySessionsPayload, token: string | null = null): Promise<DayRegistration> =>
      send(
        `/track-registrations/${registrationId}/sessions`,
        payload,
        async () => (await exemples()).dayUpdateSessions(registrationId, payload),
        'PUT',
        manageHeaders(token),
      ),

    cancel: (registrationId: Uuid, token: string | null = null): Promise<DayRegistration> =>
      send(
        `/track-registrations/${registrationId}/cancel`,
        {},
        async () => (await exemples()).dayCancel(registrationId),
        'POST',
        manageHeaders(token),
      ),

    /** Liste nominative ; la portée est vérifiée par l'API, sur l'édition du fil. */
    adminList: (trackId: Uuid): Promise<DayRegistrationRow[]> =>
      call('/admin/track-registrations', async () => (await exemples()).dayRegistrationRows(trackId), { track_id: trackId }),
  }
}
