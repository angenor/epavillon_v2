/**
 * La page publique d'un fil, vue du back-office (30/10) : sa part de `useApi()`.
 *
 * Toutes les écritures sous `/admin/tracks/{id}` rendent `TrackPageSettings`
 * entier : retirer une partie remet ses moments et activités hors partie, et
 * l'écran le voit sans recharger. Le placement d'une activité dans une partie
 * est une route du planificateur et ne rend que le rattachement.
 *
 * Règle n° 8 : chaque appel vérifie l'édition contre le périmètre avant de
 * partir ; l'API refait la garde par l'ascendance du fil (404 hors périmètre).
 */

import type {
  TrackMomentPayload,
  TrackPageSettings,
  TrackPagePayload,
  TrackPartPayload,
} from '~/types/event/track-page'
import type { TrackSessionPlacement, TrackSessionPlacementPayload } from '~/types/programme/track-page'
import type { AdministeredEvents } from '~/types/identity'
import type { TaxonomyTerm } from '~/types/reference'
import type { EventId, TrackId, Uuid } from '~/types/shared'
import type { AdminEventsApiContext } from './admin-events'

export function createAdminTrackPageApi({ call, send, assertEventInScope }: AdminEventsApiContext) {
  return {
    read: (eventId: EventId, trackId: TrackId, scope: AdministeredEvents): Promise<TrackPageSettings> => {
      assertEventInScope(eventId, scope)
      return call(`/admin/tracks/${trackId}/page`, (m) => m.readTrackPage(trackId, eventId))
    },

    /** Écriture totale des champs de la page, partenaires remplacés dans l'ordre envoyé. */
    save: (eventId: EventId, trackId: TrackId, payload: TrackPagePayload, scope: AdministeredEvents): Promise<TrackPageSettings> => {
      assertEventInScope(eventId, scope)
      return send(`/admin/tracks/${trackId}/page`, payload, (m) => m.saveTrackPage(trackId, eventId, payload), 'PUT')
    },

    /** Rôles d'un partenaire de journée : taxonomie `track_partner_role`, libellés en base. */
    partnerRoles: (taxonomy = 'track_partner_role'): Promise<TaxonomyTerm[]> =>
      call(`/reference/taxonomies/${taxonomy}/terms`, (m) => m.trackPartnerRoles()),

    createPart: (eventId: EventId, trackId: TrackId, payload: TrackPartPayload, scope: AdministeredEvents): Promise<TrackPageSettings> => {
      assertEventInScope(eventId, scope)
      return send(`/admin/tracks/${trackId}/parts`, payload, (m) => m.createTrackPart(trackId, eventId, payload))
    },

    updatePart: (eventId: EventId, trackId: TrackId, partId: Uuid, payload: TrackPartPayload, scope: AdministeredEvents): Promise<TrackPageSettings> => {
      assertEventInScope(eventId, scope)
      return send(
        `/admin/tracks/${trackId}/parts/${partId}`,
        payload,
        (m) => m.updateTrackPart(trackId, eventId, partId, payload),
        'PUT',
      )
    },

    removePart: (eventId: EventId, trackId: TrackId, partId: Uuid, scope: AdministeredEvents): Promise<TrackPageSettings> => {
      assertEventInScope(eventId, scope)
      return send(`/admin/tracks/${trackId}/parts/${partId}`, {}, (m) => m.removeTrackPart(trackId, eventId, partId), 'DELETE')
    },

    createMoment: (eventId: EventId, trackId: TrackId, payload: TrackMomentPayload, scope: AdministeredEvents): Promise<TrackPageSettings> => {
      assertEventInScope(eventId, scope)
      return send(`/admin/tracks/${trackId}/moments`, payload, (m) => m.createTrackMoment(trackId, eventId, payload))
    },

    updateMoment: (eventId: EventId, trackId: TrackId, momentId: Uuid, payload: TrackMomentPayload, scope: AdministeredEvents): Promise<TrackPageSettings> => {
      assertEventInScope(eventId, scope)
      return send(
        `/admin/tracks/${trackId}/moments/${momentId}`,
        payload,
        (m) => m.updateTrackMoment(trackId, eventId, momentId, payload),
        'PUT',
      )
    },

    removeMoment: (eventId: EventId, trackId: TrackId, momentId: Uuid, scope: AdministeredEvents): Promise<TrackPageSettings> => {
      assertEventInScope(eventId, scope)
      return send(
        `/admin/tracks/${trackId}/moments/${momentId}`,
        {},
        (m) => m.removeTrackMoment(trackId, eventId, momentId),
        'DELETE',
      )
    },

    /** Range une activité déjà rattachée dans une partie, la marque temps fort. Permission de planifier. */
    placeSession: (
      eventId: EventId,
      trackId: TrackId,
      sessionId: Uuid,
      payload: TrackSessionPlacementPayload,
      scope: AdministeredEvents,
    ): Promise<TrackSessionPlacement> => {
      assertEventInScope(eventId, scope)
      return send(
        `/admin/planner/tracks/${trackId}/sessions/${sessionId}`,
        payload,
        (m) => m.placeSessionInTrack(trackId, sessionId, payload),
        'PUT',
      )
    },
  }
}
