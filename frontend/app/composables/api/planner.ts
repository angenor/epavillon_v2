/**
 * LE PLANIFICATEUR DE CRÉNEAUX (A9) — sa part de `useApi()`, déclarée à part.
 *
 * Même motif qu'`api/proposal-review.ts` : la règle du projet est inchangée —
 * aucune page n'importe un mock, aucune page n'appelle `$fetch` ; tout passe par
 * `useApi()`, et c'est bien lui que l'écran appelle (`api.planner.screen(…)`).
 * Seule la place du CODE change, pour tenir `useApi.ts` sous le garde-fou de
 * mille lignes de `CLAUDE.md`. Le découpage suit la règle du projet : par ÉCRAN.
 *
 * ── AUCUNE DE CES ÉCRITURES NE PEUT ÊTRE REFUSÉE POUR CHEVAUCHEMENT ─────────
 *
 * C'est le contrat le plus important de ce fichier, et il se lit dans les types :
 * `schedule()` rend la séance ET les conflits, jamais une erreur « créneau
 * occupé ». Le modèle ne pose aucune contrainte d'exclusion (`075` § 1, ADR-13),
 * l'API n'en ajoutera pas, et un écran qui ferait autrement transformerait
 * l'outil d'arbitrage en mur.
 *
 * Le seul appel qui peut refuser est `publish()` — et il refuse la PUBLICATION,
 * pas le placement.
 *
 * ── LE PÉRIMÈTRE D'ADMINISTRATION ───────────────────────────────────────────
 *
 * La lecture prend le périmètre et refuse une édition qui n'y est pas (règle
 * métier n° 8), comme les autres listes du back-office. Les écritures visent une
 * séance, dont l'édition est connue de la base : c'est l'API qui les vérifiera
 * (prompt B4), et le refus lui appartient.
 */

import type {
  PlannerMutationResult,
  PlannerScreen,
  PublishProgrammeResult,
  ScheduleSessionPayload,
  SessionBroadcastPayload,
  SessionTracksPayload,
} from '~/types/admin-planner'
import type { AdministeredEvents } from '~/types/identity'
import type { Uuid } from '~/types/shared'
import type { ApiTransport } from './proposal-review'

export interface PlannerApiContext extends ApiTransport {
  /** Refuse une édition hors périmètre plutôt que de rendre une grille vide. */
  assertEventInScope: (eventId: Uuid, scope: AdministeredEvents) => void
}

export function createPlannerApi({ call, send, assertEventInScope }: PlannerApiContext) {
  return {
    /**
     * TOUT L'ÉCRAN EN UNE RÉPONSE — jours, salles, journées spéciales, canaux,
     * séances placées, séances à placer, et LES CONFLITS.
     *
     * Les conflits ne sont pas un second appel : une grille affichée avant de
     * savoir ce qui s'y chevauche montre, pendant une seconde, une programmation
     * qui a l'air saine. L'équipe arbitre sur ce qu'elle voit.
     */
    screen: (eventId: Uuid, scope: AdministeredEvents): Promise<PlannerScreen | null> => {
      assertEventInScope(eventId, scope)
      return call('/admin/planner', (m) => m.plannerScreen(eventId), { event_id: eventId })
    },

    /**
     * PLACER, DÉPLACER, REDIMENSIONNER, RETIRER — un seul appel.
     *
     * La base n'en distingue pas : ce sont `room_id`, `starts_at` et `ends_at`
     * de `programme.sessions`. Quatre routes auraient donné quatre occasions de
     * diverger sur la détection des conflits, qui est justement ce que l'écran
     * doit rendre identique dans les quatre gestes.
     *
     * `room_id: null` renvoie la séance au panneau latéral. Ce n'est pas une
     * suppression — la séance existe, son créneau souhaité reste celui du
     * dossier.
     *
     * La réponse porte les conflits de TOUTE l'édition : un déplacement peut
     * résoudre le conflit d'un bloc situé à l'autre bout de la semaine.
     *
     * Elle est TOUJOURS rendue : une séance inconnue vaut 404, donc une erreur
     * levée, jamais un corps vide. Un type nullable aurait offert à l'écran une
     * branche muette — ni séance rangée, ni message.
     */
    schedule: (payload: ScheduleSessionPayload): Promise<PlannerMutationResult> =>
      send(`/sessions/${payload.session_id}/schedule`, payload, (m) => m.scheduleSession(payload), 'PUT'),

    /**
     * RATTACHER À UNE JOURNÉE SPÉCIALE — `programme.session_tracks`.
     *
     * MANUEL ET INDÉPENDANT DE LA DATE : toutes les activités du 12 novembre ne
     * font pas partie de la « Journée finance durable ». La liste envoyée
     * remplace la précédente, et la base retient qui a rattaché quoi — la
     * composition d'un fil est un choix éditorial qu'il arrive d'expliquer à une
     * organisation qui s'étonne de ne pas y figurer.
     */
    setTracks: (personId: Uuid | null, payload: SessionTracksPayload): Promise<PlannerMutationResult> =>
      send(`/sessions/${payload.session_id}/tracks`, payload, (m) => m.setSessionTracks(payload, personId), 'PUT'),

    /**
     * MARQUER UNE SÉANCE COMME DIFFUSÉE, avec son canal.
     *
     * Le canal par défaut de l'édition est posé d'office quand il n'est pas
     * choisi : c'est ce que fait le trigger en base, et sans lui une séance
     * « diffusée » sans canal échapperait à la règle « un seul direct ». Deux
     * directs simultanés restent écrivables — ils remontent au bandeau en
     * gravité bloquante.
     */
    setBroadcast: (payload: SessionBroadcastPayload): Promise<PlannerMutationResult> =>
      send(`/sessions/${payload.session_id}/broadcast`, payload, (m) => m.setSessionBroadcast(payload), 'PUT'),

    /**
     * CE QUI RESTE À RÉGLER AVANT DE PUBLIER — lecture seule.
     *
     * Le récapitulatif s'ouvre avec cette liste avant que quiconque ait cliqué
     * sur « Publier » : on montre ce qui bloque, on ne le découvre pas en
     * essayant.
     */
    readiness: (eventId: Uuid, scope: AdministeredEvents) => {
      assertEventInScope(eventId, scope)
      return call('/admin/planner/readiness', (m) => m.publicationReadiness(eventId), { event_id: eventId })
    },

    /**
     * PUBLIER LA PROGRAMMATION — le seul contrôle bloquant de tout l'écran.
     *
     * Un point de gravité `blocking` retient TOUTE la publication : `blocked`
     * revient à vrai, rien n'est publié, et la liste dit quoi régler. Les
     * avertissements accompagnent la publication sans la retenir — un
     * intervenant attendu à deux endroits est un problème que l'équipe juge, pas
     * une impossibilité matérielle.
     */
    publish: (eventId: Uuid, scope: AdministeredEvents): Promise<PublishProgrammeResult> => {
      assertEventInScope(eventId, scope)
      return send('/admin/planner/publish', { event_id: eventId }, (m) => m.publishProgramme(eventId))
    },
  }
}
