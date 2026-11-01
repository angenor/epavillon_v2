/**
 * Régie du jour : sa part de `useApi()`. Chaque geste rend un `RegieGesture`
 * et passe par `send`, jamais rejoué : une annulation rejouée après une coupure
 * annoncerait deux fois l'activité annulée.
 */
import type {
  RegieCancelPayload,
  RegieGesture,
  RegieScreen,
  RegieUndoPayload,
} from '~/types/admin-regie'
import type { AdministeredEvents } from '~/types/identity'
import type { EventId, IsoDate, SessionId, Uuid } from '~/types/shared'
import type { ApiTransport } from './proposal-review'

interface RegieApiDeps extends Pick<ApiTransport, 'call' | 'send'> {
  assertEventInScope: (eventId: Uuid, scope: AdministeredEvents) => void
}

export function createAdminRegieApi({ call, send, assertEventInScope }: RegieApiDeps) {
  return {
    /** Sans `day`, l'API choisit : aujourd'hui dans le fuseau de l'édition, sinon le premier jour à venir. */
    screen: (eventId: EventId, day: IsoDate | null, scope: AdministeredEvents): Promise<RegieScreen | null> => {
      assertEventInScope(eventId, scope)
      return call('/admin/regie', (m) => m.regieScreen(eventId, day), day ? { event_id: eventId, day } : { event_id: eventId })
    },

    confirmPresence: (sessionId: SessionId): Promise<RegieGesture> =>
      send(`/admin/regie/sessions/${sessionId}/presence`, {}, (m) => m.regieConfirmPresence(sessionId)),

    cancel: (sessionId: SessionId, payload: RegieCancelPayload): Promise<RegieGesture> =>
      send(`/admin/regie/sessions/${sessionId}/cancel`, payload, (m) => m.regieCancel(sessionId, payload)),

    /** 409 `REGIE_LIVE_ALREADY_RUNNING` si un autre direct tourne, toutes éditions confondues. */
    startLive: (sessionId: SessionId): Promise<RegieGesture> =>
      send(`/admin/regie/sessions/${sessionId}/live/start`, {}, (m) => m.regieStartLive(sessionId)),

    endLive: (sessionId: SessionId): Promise<RegieGesture> =>
      send(`/admin/regie/sessions/${sessionId}/live/end`, {}, (m) => m.regieEndLive(sessionId)),

    /** Défait une ligne du journal ; 409 `REGIE_RESUME_EXPIRED` pour un direct terminé depuis plus de 15 min. */
    undo: (actionId: Uuid, payload: RegieUndoPayload): Promise<RegieGesture> =>
      send(`/admin/regie/actions/${actionId}/undo`, payload, (m) => m.regieUndo(actionId, payload)),
  }
}
