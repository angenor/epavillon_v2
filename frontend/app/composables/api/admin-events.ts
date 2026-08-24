/**
 * GESTION DES ÉVÉNEMENTS (A10) — sa part de `useApi()`, déclarée à part.
 *
 * Même motif qu'`api/proposal-review.ts` et `api/planner.ts` : la règle du projet
 * est inchangée — aucune page n'importe un mock, aucune page n'appelle `$fetch`.
 * Les écrans appellent `api.adminEvents.list(…)`, `api.adminEvents.detail(…)`.
 * Seule la place du CODE change, pour tenir `useApi.ts` sous le garde-fou de
 * mille lignes de `CLAUDE.md`.
 *
 * ── POURQUOI `adminEvents` ET NON `events` ──────────────────────────────────
 *
 * `api.events` existe déjà : ce sont les lectures PUBLIQUES d'une édition — la
 * page publique, le sélecteur d'année, le bandeau. Elles ne prennent aucun
 * périmètre d'administration, et pour cause : une édition annoncée est publique.
 * Les appels d'ici sont ceux du back-office : ils prennent le périmètre, refusent
 * une édition qui n'y est pas, et écrivent. Les mélanger sous une même clé aurait
 * fini par faire passer une lecture publique pour une lecture administrée.
 *
 * ── LE PÉRIMÈTRE D'ADMINISTRATION ───────────────────────────────────────────
 *
 * Règle métier n° 8. La liste est filtrée par le périmètre ; le détail et les
 * écritures REFUSENT une édition hors périmètre plutôt que de rendre une page
 * vide — les deux ne se lisent pas pareil. Ce filtrage sera doublé côté API
 * (prompt B3) : ce n'est pas ici un contrôle de sécurité mais le comportement
 * attendu de l'écran.
 *
 * ── CE QUE CES ÉCRITURES PEUVENT REFUSER ────────────────────────────────────
 *
 * Beaucoup, et c'est la différence avec le planificateur. Les contraintes de
 * `060_events.sql` sont des invariants de DONNÉES, pas des arbitrages : un slug
 * en double, une clôture avant l'ouverture, deux appels sur une même édition sont
 * refusés en base et le sont ici. Rien à voir avec un chevauchement de créneaux,
 * qui reste toujours écrivable (règle métier n° 2).
 *
 * ── UNE ÉCRITURE D'ONGLET REND LA COMPOSITION ENTIÈRE ───────────────────────
 *
 * `EditionTabResult.detail` porte les six onglets recalculés. Le coût est assumé :
 * ajouter une salle change le décompte des séances plaçables, retirer un jour
 * détache des séances, désactiver un canal touche la règle du direct unique. Rendre
 * seulement l'objet modifié laisserait cinq onglets afficher des décomptes faux
 * jusqu'au prochain rechargement.
 */

import type {
  CallSaveResult,
  CommitteePayload,
  CommitteeSaveResult,
  DayGenerationPlan,
  EditionCallPayload,
  EditionChannelPayload,
  EditionCriterion,
  EditionDayPayload,
  EditionDetail,
  EditionFormOptions,
  EditionFormPayload,
  EditionImagePayload,
  EditionListScreen,
  EditionRoomPayload,
  EditionSaveResult,
  EditionTabResult,
  EditionTrackPayload,
  EditionVenuePayload,
} from '~/types/admin-events'
import type { AdministeredEvents } from '~/types/identity'
import type { AssetId, Uuid } from '~/types/shared'
import type { ApiTransport } from './proposal-review'

export interface AdminEventsApiContext extends ApiTransport {
  /** Refuse une édition hors périmètre plutôt que de rendre une page vide. */
  assertEventInScope: (eventId: Uuid, scope: AdministeredEvents) => void
}

/**
 * Le lot de remplacement de `PUT /media/attachments`.
 *
 * Chaque rôle NOMMÉ dans la liste est vidé puis regarni ; un rôle absent n'est
 * pas touché. C'est ce qui permet aux trois déclinaisons de partir d'un geste et
 * à un `asset_id` nul d'en retirer une sans toucher aux deux autres.
 */
interface AttachmentBatch {
  owner_schema: string
  owner_table: string
  owner_id: Uuid
  assignments: { role: string; asset_id: AssetId | null }[]
}

export function createAdminEventsApi({ call, send, assertEventInScope }: AdminEventsApiContext) {
  return {
    /**
     * LA LISTE DES ÉDITIONS, ses séries et ses années — en une réponse.
     *
     * Aucun `assertEventInScope` ici : la liste n'a pas d'édition à vérifier, elle
     * est FILTRÉE par le périmètre. Une personne sans aucun droit reçoit une liste
     * vide, et c'est l'écran qui rend « accès refusé » sur `canAdminister`.
     */
    list: (scope: AdministeredEvents): Promise<EditionListScreen> =>
      call('/admin/events', (m) => m.editionListScreen(scope)),

    /**
     * Ce qu'il faut pour ouvrir le formulaire : séries, pays, fuseaux, statuts.
     *
     * Séparé de la liste parce qu'il ne change pas d'une édition à l'autre — le
     * référentiel des pays n'a pas à repartir à chaque affichage du tableau.
     */
    formOptions: (): Promise<EditionFormOptions> =>
      call('/admin/events/form-options', (m) => m.editionFormOptions()),

    /**
     * TOUT L'ÉCRAN DE DÉTAIL — l'édition et ses six onglets.
     *
     * Rend `null` pour une édition inexistante ; LÈVE pour une édition hors
     * périmètre. La distinction compte : « cette édition n'existe pas » et « vous
     * n'y avez pas accès » n'appellent pas le même écran, et confondre les deux
     * renseignerait sur l'existence de ce qu'on n'a pas le droit de voir.
     */
    detail: (eventId: Uuid, scope: AdministeredEvents): Promise<EditionDetail | null> => {
      assertEventInScope(eventId, scope)
      return call(`/admin/events/${eventId}`, (m) => m.editionDetail(eventId))
    },

    /**
     * CRÉER OU MODIFIER UNE ÉDITION.
     *
     * La création n'exige aucun périmètre d'édition — il n'y en a pas encore. Elle
     * demande `event.event.manage` sur la portée GLOBALE, ce que l'écran vérifie
     * par permission et que l'API vérifiera à son tour.
     *
     * La réponse dit ce qui est arrivé au CALENDRIER : une période élargie ajoute
     * des jours, une période resserrée en laisse hors bornes. L'écran l'annonce
     * plutôt que de le laisser découvrir au planificateur.
     */
    save: (payload: EditionFormPayload, actorId: Uuid | null, scope: AdministeredEvents): Promise<EditionSaveResult> => {
      if (payload.id) assertEventInScope(payload.id, scope)
      return send(
        payload.id ? `/admin/events/${payload.id}` : '/admin/events',
        payload,
        (m) => m.saveEdition(payload, actorId),
        payload.id ? 'PUT' : 'POST',
      )
    },

    /**
     * LES TROIS DÉCLINAISONS DE L'ÉDITION — écriture du module MÉDIA.
     *
     * L'enregistrement d'une édition NE POSE PAS ses images : `event.events` ne
     * les porte pas, le rattachement est polymorphe, et un crate de module
     * n'écrit pas dans le schéma d'un autre. C'est donc `PUT /media/attachments`
     * qui les pose, sur `('event', 'events', <édition>)`.
     *
     * L'ORDRE COMPTE. Sur une édition existante, on rattache AVANT d'enregistrer
     * la fiche : un fichier qui n'a pas la forme de son rôle est refusé, et rien
     * n'a alors été écrit. À la création, l'inverse est forcé — l'objet à qui
     * rattacher n'existe pas encore.
     */
    saveImages: (eventId: Uuid, images: EditionImagePayload, scope: AdministeredEvents): Promise<void> => {
      assertEventInScope(eventId, scope)
      const batch: AttachmentBatch = {
        owner_schema: 'event',
        owner_table: 'events',
        owner_id: eventId,
        assignments: Object.entries(images).map(([role, asset_id]) => ({ role, asset_id })),
      }
      // Hors ligne, rien n'est posé : `attachEditionImages` existe dans
      // `mocks/admin-events` mais attend d'être ré-exporté par `mocks/index.ts`.
      return send('/media/attachments', batch, () => undefined, 'PUT')
    },

    // -----------------------------------------------------------------------
    // Onglet « Journées du calendrier »
    // -----------------------------------------------------------------------

    /**
     * CE QUE LA GÉNÉRATION VA FAIRE, AVANT DE LE FAIRE.
     *
     * Lecture seule. Les jours ne sont pas dérivés par un trigger — `event_days`
     * n'en porte aucun —, la génération est donc un geste explicite, et un geste
     * explicite s'annonce : combien de jours créés, lesquels sortent de la période
     * et combien de séances ils portent.
     */
    dayPlan: (eventId: Uuid, scope: AdministeredEvents): Promise<DayGenerationPlan | null> => {
      assertEventInScope(eventId, scope)
      return call(`/admin/events/${eventId}/days/plan`, (m) => m.planDayGeneration(eventId))
    },

    /**
     * GÉNÉRER LE CALENDRIER. `removeOutsidePeriod` retire les jours hors bornes —
     * et détache les séances qu'ils portaient, ce que la réponse chiffre. Une
     * édition garde parfois un jour hors période à dessein : le choix est à
     * l'équipe, pas au code.
     */
    generateDays: (eventId: Uuid, removeOutsidePeriod: boolean, scope: AdministeredEvents): Promise<EditionTabResult> => {
      assertEventInScope(eventId, scope)
      return send(
        `/admin/events/${eventId}/days`,
        { remove_outside_period: removeOutsidePeriod },
        (m) => m.generateEventDays(eventId, removeOutsidePeriod),
      )
    },

    /** Le contenu ÉDITORIAL d'un jour : titre, slug de sa page, couleur, mise en avant. */
    saveDay: (eventId: Uuid, payload: EditionDayPayload, scope: AdministeredEvents): Promise<EditionTabResult> => {
      assertEventInScope(eventId, scope)
      return send(`/admin/events/${eventId}/days/${payload.id}`, payload, (m) => m.saveEventDay(eventId, payload), 'PUT')
    },

    // -----------------------------------------------------------------------
    // Onglet « Journées spéciales »
    // -----------------------------------------------------------------------

    /**
     * CRÉER OU MODIFIER UN FIL. Rien ici ne compose le fil : le rattachement des
     * séances vit dans `programme.session_tracks` et se décide au planificateur
     * (A9). Cet appel écrit le fil, son habillage et sa page publique.
     */
    saveTrack: (payload: EditionTrackPayload, scope: AdministeredEvents): Promise<EditionTabResult> => {
      assertEventInScope(payload.event_id, scope)
      return send(
        payload.id ? `/admin/tracks/${payload.id}` : '/admin/tracks',
        payload,
        (m) => m.saveTrack(payload),
        payload.id ? 'PUT' : 'POST',
      )
    },

    /**
     * SUPPRIMER UN FIL — la seule suppression de cet écran qui CASCADE.
     * `sessions_detached` compte les rattachements perdus : c'est du travail
     * éditorial, et l'écran le chiffre avant de confirmer.
     */
    removeTrack: (eventId: Uuid, trackId: Uuid, scope: AdministeredEvents): Promise<EditionTabResult> => {
      assertEventInScope(eventId, scope)
      return send(`/admin/tracks/${trackId}`, { event_id: eventId }, (m) => m.removeTrack(eventId, trackId), 'DELETE')
    },

    // -----------------------------------------------------------------------
    // Onglet « Lieux et salles »
    // -----------------------------------------------------------------------

    saveVenue: (payload: EditionVenuePayload, scope: AdministeredEvents): Promise<EditionTabResult> => {
      assertEventInScope(payload.event_id, scope)
      return send(
        payload.id ? `/admin/venues/${payload.id}` : '/admin/venues',
        payload,
        (m) => m.saveVenue(payload),
        payload.id ? 'PUT' : 'POST',
      )
    },

    /** Retirer un lieu retire ses salles (`CASCADE`) et déplace leurs séances au panneau. */
    removeVenue: (eventId: Uuid, venueId: Uuid, scope: AdministeredEvents): Promise<EditionTabResult> => {
      assertEventInScope(eventId, scope)
      return send(`/admin/venues/${venueId}`, { event_id: eventId }, (m) => m.removeVenue(eventId, venueId), 'DELETE')
    },

    /**
     * CRÉER OU MODIFIER UNE SALLE.
     *
     * `is_virtual` n'est pas un détail d'inventaire : une salle virtuelle accepte
     * les créneaux simultanés et `detect_conflicts()` n'y signale plus de double
     * réservation. Basculer le stand physique en virtuel ferait taire le conflit
     * de gravité haute que l'équipe doit absolument voir (règle métier n° 3).
     */
    saveRoom: (eventId: Uuid, payload: EditionRoomPayload, scope: AdministeredEvents): Promise<EditionTabResult> => {
      assertEventInScope(eventId, scope)
      return send(
        payload.id ? `/admin/rooms/${payload.id}` : '/admin/rooms',
        payload,
        (m) => m.saveRoom(eventId, payload),
        payload.id ? 'PUT' : 'POST',
      )
    },

    removeRoom: (eventId: Uuid, roomId: Uuid, scope: AdministeredEvents): Promise<EditionTabResult> => {
      assertEventInScope(eventId, scope)
      return send(`/admin/rooms/${roomId}`, { event_id: eventId }, (m) => m.removeRoom(eventId, roomId), 'DELETE')
    },

    // -----------------------------------------------------------------------
    // Onglet « Canal de diffusion »
    // -----------------------------------------------------------------------

    /**
     * CRÉER OU MODIFIER UN CANAL. Poser le canal par défaut le retire du
     * précédent : `ux_broadcast_channels_default` n'en autorise qu'un par édition,
     * et c'est lui qui fait tenir la règle « un seul direct à la fois » sans que
     * personne ait à le saisir séance par séance.
     */
    saveChannel: (payload: EditionChannelPayload, scope: AdministeredEvents): Promise<EditionTabResult> => {
      assertEventInScope(payload.event_id, scope)
      return send(
        payload.id ? `/admin/channels/${payload.id}` : '/admin/channels',
        payload,
        (m) => m.saveChannel(payload),
        payload.id ? 'PUT' : 'POST',
      )
    },

    /**
     * RETIRER UN CANAL — désactivé s'il a servi, supprimé sinon.
     *
     * Une séance passée garde la trace du canal sur lequel elle a été diffusée ;
     * `error_code: 'deactivated'` dit à l'écran que la ligne est restée, inactive.
     * C'est un succès, pas un refus.
     */
    removeChannel: (eventId: Uuid, channelId: Uuid, scope: AdministeredEvents): Promise<EditionTabResult> => {
      assertEventInScope(eventId, scope)
      return send(
        `/admin/channels/${channelId}`,
        { event_id: eventId },
        (m) => m.removeChannel(eventId, channelId),
        'DELETE',
      )
    },

    // -----------------------------------------------------------------------
    // Onglet « Appel à propositions »
    // -----------------------------------------------------------------------

    /**
     * ENREGISTRER L'APPEL ET SA GRILLE, ENSEMBLE.
     *
     * UN SEUL APPEL PAR ÉDITION : `already_exists` traduit
     * `ux_calls_one_per_event`, et l'écran n'offre pas d'en créer un second.
     *
     * `scores_affected` prévient qu'un barème modifié va déplacer des moyennes
     * déjà calculées. Les notes ne sont pas perdues — `review_scores` référence le
     * critère — mais `refresh_proposal_score()` les repondère, et un classement
     * qui bouge sans explication est une conversation difficile avec le comité.
     */
    saveCall: (payload: EditionCallPayload, actorId: Uuid | null, scope: AdministeredEvents): Promise<CallSaveResult> => {
      assertEventInScope(payload.event_id, scope)
      return send(
        payload.id ? `/admin/calls/${payload.id}` : '/admin/calls',
        payload,
        (m) => m.saveCall(payload, actorId),
        payload.id ? 'PUT' : 'POST',
      )
    },

    /**
     * LA GRILLE PAR DÉFAUT — `event.seed_default_criteria()`.
     *
     * Lue et non recopiée : les six critères et leurs poids vivent dans la base,
     * et les réécrire dans un composant Vue serait le défaut n° 1 de la v1
     * appliqué à une grille d'évaluation.
     */
    defaultCriteria: (): Promise<EditionCriterion[]> =>
      call('/admin/calls/default-criteria', (m) => m.defaultCriteriaGrid()),

    // -----------------------------------------------------------------------
    // Onglet « Comité de sélection »
    // -----------------------------------------------------------------------

    /**
     * ENREGISTRER LA COMPOSITION DU COMITÉ, D'UN SEUL GESTE — ajouts, retraits et
     * plafonds de charge ensemble.
     *
     * `removed_with_assignments` nomme les membres retirés qui portaient encore des
     * dossiers : leurs revues rendues restent au dossier, mais quelqu'un doit
     * reprendre le reste. Un retrait silencieux laisse des dossiers sans lecteur à
     * trois jours de la décision.
     */
    saveCommittee: (eventId: Uuid, payload: CommitteePayload, scope: AdministeredEvents): Promise<CommitteeSaveResult> => {
      assertEventInScope(eventId, scope)
      return send(`/admin/calls/${payload.call_id}/reviewers`, payload, (m) => m.saveCommittee(payload, eventId), 'PUT')
    },
  }
}
