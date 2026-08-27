/**
 * MESSAGES D'INCIDENT (A13) — sa part de `useApi()`.
 *
 * Même motif qu'`api/admin-users.ts` : les pages appellent
 * `api.adminIncidents.list(…)`, n'importent aucun mock et n'appellent jamais
 * `$fetch`. Seule la place du code change, pour tenir `useApi.ts` sous le
 * garde-fou de mille lignes.
 *
 * ── LES HUIT ROUTES EXISTENT DEPUIS LE 27/08 ────────────────────────────────
 *
 * Le crate `live` porte les sept routes de back-office et la lecture publique.
 * Les appels passent donc par `call`, `callOrNull` et `send`, avec leurs
 * paramètres, leurs corps et leurs verbes — `PUT` pour la correction, `DELETE`
 * pour la dépublication. Les jeux d'exemple restent en second argument : sans
 * `NUXT_PUBLIC_API_BASE`, l'écran fonctionne toujours hors ligne.
 *
 * ── LE PÉRIMÈTRE EST VÉRIFIÉ AVANT L'APPEL ──────────────────────────────────
 *
 * `assertEventInScope` d'abord, comme pour le tableau de bord, la liste des
 * propositions et le planificateur : une édition hors périmètre REFUSE l'accès
 * plutôt que de rendre une liste vide, qui se lirait comme « aucun incident » au
 * lieu de « ceci ne vous regarde pas ». Règle métier n° 8.
 *
 * ── `granted` A DISPARU DES QUATRE ÉCRITURES ───────────────────────────────
 *
 * Il servait à rejouer, sur des données d'exemple, ce que
 * `has_permission(acteur, 'live.incident.publish', 'event', édition)` décide en
 * base. L'API lit désormais sa propre session, et **un client qui déclare ses
 * propres droits n'est pas un contrôle d'accès**.
 *
 * Il reste employé AILLEURS, et c'est un autre usage : les pages s'en servent
 * pour décider d'AFFICHER un bouton. Décider ce qu'on montre et décider ce qu'on
 * autorise sont deux choses, et seule la seconde appartient à l'API.
 *
 * ── PUBLIER N'EST PAS ENREGISTRER ───────────────────────────────────────────
 *
 * Deux chemins distincts, parce que ce sont deux actes distincts en base :
 * `live.publish_incident()` et `live.unpublish_incident()` horodatent et
 * attribuent, la seconde gardant le motif. Un brouillon se relit avant de parler
 * à toute une COP, et un bandeau retiré laisse une trace.
 */

import type {
  CreateIncidentPayload,
  IncidentListScreen,
  IncidentWriteResult,
  ManagedIncident,
  UnpublishIncidentPayload,
  UpdateIncidentPayload,
} from '~/types/admin-incidents'
import type { OverrunTemplate } from '~/types/admin-incidents'
import type { AdministeredEvents } from '~/types/identity'
import type { EventId, SessionId, Uuid } from '~/types/shared'
import type { ApiTransport } from './proposal-review'

interface IncidentsApiDeps extends Pick<ApiTransport, 'call' | 'callOrNull' | 'send'> {
  assertEventInScope: (eventId: Uuid, scope: AdministeredEvents) => void
}

export function createAdminIncidentsApi({
  call,
  callOrNull,
  send,
  assertEventInScope,
}: IncidentsApiDeps) {
  return {
    /**
     * LA LISTE ET SES CIBLES — en une réponse.
     *
     * `live.event_incidents()` rend déjà les lignes TRIÉES : actifs d'abord,
     * puis programmés, puis brouillons, puis l'historique, gravité décroissante
     * à état égal. L'écran ne réordonne pas — c'est l'ordre dans lequel on agit.
     */
    list: (eventId: EventId, scope: AdministeredEvents): Promise<IncidentListScreen | null> => {
      assertEventInScope(eventId, scope)
      return call('/admin/incidents', (m) => m.incidentListScreen(eventId), { event_id: eventId })
    },

    /** UN MESSAGE, pour le relire et le corriger. `null` s'il n'existe pas. */
    byId: (incidentId: Uuid, eventId: EventId, scope: AdministeredEvents): Promise<ManagedIncident | null> => {
      assertEventInScope(eventId, scope)
      return callOrNull(`/admin/incidents/${incidentId}`, (m) => m.incidentById(incidentId))
    },

    /**
     * CE QUE LE RACCOURCI « SIGNALER UN DÉBORDEMENT » A BESOIN DE SAVOIR.
     *
     * La séance visée, son titre et son créneau : de quoi pré-remplir la portée,
     * la nature et la fenêtre d'affichage sans que l'équipe saisisse quoi que ce
     * soit pendant que la salle attend.
     */
    overrunTemplate: (sessionId: SessionId): Promise<OverrunTemplate | null> =>
      callOrNull(
        `/admin/incidents/overrun-template`,
        (m) => m.overrunTemplate(sessionId),
        { session_id: sessionId },
      ),

    /** RÉDIGER, et publier dans le même geste si `publish` est vrai. */
    create: (payload: CreateIncidentPayload, actorId: Uuid | null): Promise<IncidentWriteResult> =>
      send('/admin/incidents', payload, (m) => m.createIncident(payload, actorId, []), 'POST'),

    /** CORRIGER. Republier efface la dépublication, comme `live.publish_incident()`. */
    update: (payload: UpdateIncidentPayload, actorId: Uuid | null): Promise<IncidentWriteResult> =>
      send(
        `/admin/incidents/${payload.incident_id}`,
        payload,
        (m) => m.updateIncident(payload, actorId, []),
        'PUT',
      ),

    /** PUBLIER un brouillon, ou rétablir un message retiré. */
    publish: (
      incidentId: Uuid,
      eventId: EventId,
      actorId: Uuid | null,
    ): Promise<IncidentWriteResult> =>
      send(
        `/admin/incidents/${incidentId}/publish`,
        {},
        (m) => m.publishIncident(incidentId, eventId, actorId, []),
        'POST',
      ),

    /**
     * DÉPUBLIER EN UN CLIC, avec motif.
     *
     * Le chemin est celui de la publication et le verbe est `DELETE` — mais ce
     * n'est pas une suppression : la ligne reste, avec `unpublished_at`,
     * `unpublished_by` et `unpublish_reason`. C'est l'historique que le prompt
     * demande, et c'est aussi ce que fait la base — `live.incidents` n'a pas de
     * suppression.
     *
     * **Un `DELETE` porteur d'un corps, et c'est délibéré** : le motif accompagne
     * le geste.
     */
    unpublish: (
      payload: UnpublishIncidentPayload,
      eventId: EventId,
      actorId: Uuid | null,
    ): Promise<IncidentWriteResult> =>
      send(
        `/admin/incidents/${payload.incident_id}/publish`,
        payload,
        (m) => m.unpublishIncident(payload, eventId, actorId, []),
        'DELETE',
      ),
  }
}
