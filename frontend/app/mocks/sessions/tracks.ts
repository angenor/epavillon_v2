/**
 * Données simulées de `programme.session_tracks` — la composition des journées
 * spéciales.
 *
 * LE RATTACHEMENT EST UN CHOIX ÉDITORIAL, JAMAIS UNE DÉDUCTION SUR LES DATES.
 * Deux vérifications que ce jeu de données permet :
 *
 *   - le 12 novembre porte quatre séances, dont TROIS SEULEMENT appartiennent à
 *     la journée finance durable. Un écran qui déduirait le fil des dates y
 *     rangerait aussi le bilan de fin de journée — qui n'en fait pas partie ;
 *   - la journée jeunesse et climat DÉBORDE sur le 17 novembre au matin, avec
 *     la séance en ligne. Un fil n'a pas les bornes d'un jour de calendrier.
 *
 * On trace QUI a rattaché QUOI : il arrive d'avoir à l'expliquer à une
 * organisation qui s'étonne de ne pas figurer dans une journée spéciale.
 */

import type { SessionTrack } from '~/types/programme/session'
import { PERSON, SESSION, TRACK } from '../ids'

export const sessionTracks = [
  // --- Journée finance durable (12 novembre) -------------------------------
  {
    session_id: SESSION.pertesPrejudices,
    track_id: TRACK.financeDurable,
    sort_order: 10,
    is_highlight: true,
    added_by: PERSON.duchesne,
    added_at: '2026-07-30T10:00:00Z',
  },
  {
    session_id: SESSION.accesFondsVert,
    track_id: TRACK.financeDurable,
    sort_order: 20,
    is_highlight: false,
    added_by: PERSON.duchesne,
    added_at: '2026-07-30T10:05:00Z',
  },
  {
    session_id: SESSION.article6,
    track_id: TRACK.financeDurable,
    sort_order: 30,
    is_highlight: false,
    added_by: PERSON.duchesne,
    added_at: '2026-07-30T10:10:00Z',
  },
  // Le bilan de la journée se tient le 12 novembre et n'appartient PAS au fil :
  // c'est une séance de l'IFDD, hors sélection éditoriale.

  // --- Journée jeunesse et climat (16 novembre après-midi → 17 au matin) ----
  {
    session_id: SESSION.forumJeunesse,
    track_id: TRACK.jeunesseClimat,
    sort_order: 10,
    is_highlight: false,
    added_by: PERSON.nkoDiop,
    added_at: '2026-07-30T11:00:00Z',
  },
  {
    session_id: SESSION.transitionJuste,
    track_id: TRACK.jeunesseClimat,
    sort_order: 20,
    is_highlight: false,
    added_by: PERSON.nkoDiop,
    added_at: '2026-07-30T11:05:00Z',
  },
  {
    session_id: SESSION.releveJeunesse,
    track_id: TRACK.jeunesseClimat,
    sort_order: 30,
    is_highlight: true,
    added_by: PERSON.nkoDiop,
    added_at: '2026-07-30T11:10:00Z',
  },
  {
    // DÉBORDEMENT SUR LE LENDEMAIN : la séance en ligne du 17 novembre au matin
    // appartient à la journée du 16. C'est exactement ce qu'un rattachement
    // déduit des dates ne saurait pas faire.
    session_id: SESSION.releveJeunesse2,
    track_id: TRACK.jeunesseClimat,
    sort_order: 40,
    is_highlight: false,
    added_by: PERSON.nkoDiop,
    added_at: '2026-08-04T11:08:00Z',
  },
] satisfies SessionTrack[]
