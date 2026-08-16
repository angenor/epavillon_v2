/**
 * Données simulées de `programme.session_speakers` — les intervenants DU JOUR.
 *
 * Ils sont recopiés depuis la proposition à la programmation, puis modifiés :
 * ceux qui étaient annoncés dans le dossier ne sont pas toujours ceux qui
 * montent sur scène. Deux différences volontaires avec
 * `mocks/proposals/speakers.ts` :
 *
 *   - la session sur les mangroves a perdu un intervenant du dossier et gagné
 *     une modératrice de l'IFDD ;
 *   - la session d'ouverture n'a aucun équivalent côté proposition : l'IFDD la
 *     programme directement.
 *
 * `attended` reste nul tant que la présence n'a pas été constatée — l'édition
 * simulée n'a pas encore eu lieu. Un écran qui l'affiche comme « absent » avant
 * la séance dirait le contraire de la donnée.
 */

import type { SessionSpeaker } from '~/types/programme/session'
import type { SpeakerRole } from '~/types/programme/proposal'
import { PERSON, SESSION, SESSION_SPEAKER } from '../ids'

interface SessionSpeakerFields {
  role?: SpeakerRole
  job: string
  org: string
  confirmedAt?: string | null
}

function speaker(
  n: number,
  session_id: string,
  person_id: string,
  sort_order: number,
  fields: SessionSpeakerFields,
): SessionSpeaker {
  return {
    id: SESSION_SPEAKER(n),
    session_id,
    person_id,
    role: fields.role ?? 'speaker',
    job_title_snapshot: fields.job,
    organization_snapshot: fields.org,
    bio: null,
    confirmed_at: fields.confirmedAt ?? null,
    attended: null,
    sort_order,
    created_at: '2026-08-03T14:00:00Z',
  }
}

export const sessionSpeakers = [
  // Ouverture — programmée directement par l'IFDD
  speaker(1, SESSION.ouverture, PERSON.bakayoko, 10, {
    role: 'moderator',
    job: 'Responsable de la programmation',
    org: 'Institut de la Francophonie pour le développement durable',
    confirmedAt: '2026-08-01T09:00:00Z',
  }),
  speaker(2, SESSION.ouverture, PERSON.perretAdmin, 20, {
    job: 'Coordonnatrice de la COP31',
    org: 'Institut de la Francophonie pour le développement durable',
    confirmedAt: '2026-08-01T09:05:00Z',
  }),

  // Adaptation côtière — reprise fidèle du dossier
  speaker(10, SESSION.adaptationCotiere, PERSON.sowFall, 10, {
    role: 'moderator',
    job: 'Directrice exécutive',
    org: "Réseau ouest-africain pour l'adaptation côtière",
    confirmedAt: '2026-08-03T15:00:00Z',
  }),
  speaker(11, SESSION.adaptationCotiere, PERSON.mbayeNdiaye, 20, {
    job: 'Chargé de projet littoral',
    org: "Réseau ouest-africain pour l'adaptation côtière",
    confirmedAt: '2026-08-03T15:00:00Z',
  }),
  speaker(12, SESSION.adaptationCotiere, PERSON.zinsou, 30, {
    job: 'Directeur de la transition écologique',
    org: "Agence nationale de la transition écologique du Bénin",
    confirmedAt: '2026-08-03T15:05:00Z',
  }),
  speaker(13, SESSION.adaptationCotiere, PERSON.duchesne, 40, {
    role: 'panelist',
    job: 'Expert finance climatique',
    org: 'Institut de la Francophonie pour le développement durable',
    confirmedAt: '2026-08-03T15:05:00Z',
  }),

  // Alerte précoce
  speaker(20, SESSION.alertePrecoce, PERSON.ouedraogo, 10, {
    role: 'keynote',
    job: 'Directeur',
    org: "Observatoire du Sahel pour l'énergie durable",
    confirmedAt: '2026-08-03T15:10:00Z',
  }),
  speaker(21, SESSION.alertePrecoce, PERSON.kabore, 20, {
    job: "Cheffe du département accès à l'énergie",
    org: "Observatoire du Sahel pour l'énergie durable",
    confirmedAt: '2026-08-03T15:10:00Z',
  }),

  // Mini-réseaux solaires
  speaker(30, SESSION.miniReseaux, PERSON.kabore, 10, {
    role: 'moderator',
    job: "Cheffe du département accès à l'énergie",
    org: "Observatoire du Sahel pour l'énergie durable",
    confirmedAt: '2026-08-03T15:15:00Z',
  }),
  speaker(31, SESSION.miniReseaux, PERSON.moreau, 20, {
    job: 'Associé fondateur',
    org: 'Verdéo Solutions',
    confirmedAt: '2026-08-03T15:15:00Z',
  }),

  // Pertes et préjudices
  speaker(40, SESSION.pertesPrejudices, PERSON.josephPierre, 10, {
    role: 'moderator',
    job: 'Responsable des programmes',
    org: 'Fonds haïtien pour la résilience communautaire',
    confirmedAt: '2026-08-03T15:20:00Z',
  }),
  speaker(41, SESSION.pertesPrejudices, PERSON.lemoine, 20, {
    role: 'keynote',
    job: 'Professeure de droit international de l’environnement',
    org: 'Chaire universitaire de droit climatique de Montréal',
    confirmedAt: '2026-08-03T15:20:00Z',
  }),
  speaker(42, SESSION.pertesPrejudices, PERSON.ngoBassong, 30, {
    role: 'panelist',
    job: 'Coordonnatrice générale',
    org: 'Coalition des femmes pour le climat en Afrique centrale',
    confirmedAt: '2026-08-03T15:25:00Z',
  }),

  // Article 6
  speaker(50, SESSION.article6, PERSON.lemoine, 10, {
    role: 'keynote',
    job: 'Professeure de droit international de l’environnement',
    org: 'Chaire universitaire de droit climatique de Montréal',
    confirmedAt: '2026-08-03T15:30:00Z',
  }),
  speaker(51, SESSION.article6, PERSON.gagnon, 20, {
    role: 'moderator',
    job: 'Coordonnatrice de la chaire',
    org: 'Chaire universitaire de droit climatique de Montréal',
    confirmedAt: '2026-08-03T15:30:00Z',
  }),

  // Accès au Fonds vert
  speaker(60, SESSION.accesFondsVert, PERSON.duchesne, 10, {
    role: 'facilitator',
    job: 'Expert finance climatique',
    org: 'Institut de la Francophonie pour le développement durable',
    confirmedAt: '2026-08-03T15:35:00Z',
  }),
  speaker(61, SESSION.accesFondsVert, PERSON.josephPierre, 20, {
    job: 'Responsable des programmes',
    org: 'Fonds haïtien pour la résilience communautaire',
    confirmedAt: '2026-08-03T15:35:00Z',
  }),
  speaker(62, SESSION.accesFondsVert, PERSON.zinsou, 30, {
    job: 'Directeur de la transition écologique',
    org: "Agence nationale de la transition écologique du Bénin",
    confirmedAt: '2026-08-03T15:40:00Z',
  }),

  // Agroécologie
  speaker(70, SESSION.agroecologie, PERSON.zinsou, 10, {
    role: 'moderator',
    job: 'Directeur de la transition écologique',
    org: "Agence nationale de la transition écologique du Bénin",
    confirmedAt: '2026-08-03T15:45:00Z',
  }),

  // Bassin du Niger
  speaker(80, SESSION.bassinNiger, PERSON.benAmor, 10, {
    role: 'keynote',
    job: 'Directeur de recherche',
    org: "Institut méditerranéen de recherche sur l'eau",
    confirmedAt: '2026-08-03T15:50:00Z',
  }),
  speaker(81, SESSION.bassinNiger, PERSON.elFassi, 20, {
    role: 'moderator',
    job: 'Chercheuse principale',
    org: "Institut méditerranéen de recherche sur l'eau",
    confirmedAt: '2026-08-03T15:50:00Z',
  }),

  // Mangroves — LA LISTE A CHANGÉ depuis le dossier : Ousmane Mbaye Ndiaye ne
  // fait plus le déplacement, une modératrice de l'IFDD le remplace.
  speaker(90, SESSION.mangroves, PERSON.ngoBassong, 10, {
    job: 'Coordonnatrice générale',
    org: 'Coalition des femmes pour le climat en Afrique centrale',
    confirmedAt: '2026-08-03T15:55:00Z',
  }),
  speaker(91, SESSION.mangroves, PERSON.nkoDiop, 20, {
    role: 'moderator',
    job: 'Programmatrice',
    org: 'Institut de la Francophonie pour le développement durable',
    confirmedAt: '2026-08-14T09:00:00Z',
  }),

  // Élevage pastoral
  speaker(100, SESSION.pastoralisme, PERSON.sowFall, 10, {
    role: 'moderator',
    job: 'Directrice exécutive',
    org: "Réseau ouest-africain pour l'adaptation côtière",
    confirmedAt: '2026-08-03T16:00:00Z',
  }),

  // Transition juste
  speaker(110, SESSION.transitionJuste, PERSON.ngoBassong, 10, {
    role: 'moderator',
    job: 'Coordonnatrice générale',
    org: 'Coalition des femmes pour le climat en Afrique centrale',
    confirmedAt: '2026-08-03T16:05:00Z',
  }),
  speaker(111, SESSION.transitionJuste, PERSON.koffi, 20, {
    job: 'Président',
    org: 'Union des jeunes francophones pour le climat',
    confirmedAt: '2026-08-03T16:05:00Z',
  }),

  // Relève jeunesse — les deux occurrences ont le même panel
  speaker(120, SESSION.releveJeunesse, PERSON.koffi, 10, {
    role: 'moderator',
    job: 'Président',
    org: 'Union des jeunes francophones pour le climat',
    confirmedAt: '2026-08-03T16:10:00Z',
  }),
  speaker(121, SESSION.releveJeunesse, PERSON.nkoDiop, 20, {
    role: 'panelist',
    job: 'Programmatrice',
    org: 'Institut de la Francophonie pour le développement durable',
    confirmedAt: '2026-08-03T16:10:00Z',
  }),
  speaker(122, SESSION.releveJeunesse2, PERSON.koffi, 10, {
    role: 'moderator',
    job: 'Président',
    org: 'Union des jeunes francophones pour le climat',
    confirmedAt: '2026-08-04T11:10:00Z',
  }),

  // Ateliers et séances programmées par l'IFDD
  speaker(130, SESSION.atelierNegociation1, PERSON.tremblay, 10, {
    role: 'facilitator',
    job: 'Chargé de programme énergie et climat',
    org: 'Institut de la Francophonie pour le développement durable',
    confirmedAt: '2026-08-03T16:15:00Z',
  }),
  speaker(131, SESSION.rapportsBiennaux1, PERSON.tremblay, 10, {
    role: 'facilitator',
    job: 'Chargé de programme énergie et climat',
    org: 'Institut de la Francophonie pour le développement durable',
    confirmedAt: '2026-08-04T11:15:00Z',
  }),
  speaker(132, SESSION.rapportsBiennaux1, PERSON.nkoDiop, 20, {
    role: 'facilitator',
    job: 'Programmatrice',
    org: 'Institut de la Francophonie pour le développement durable',
    confirmedAt: '2026-08-04T11:15:00Z',
  }),
  speaker(133, SESSION.rapportsBiennaux2, PERSON.tremblay, 10, {
    role: 'facilitator',
    job: 'Chargé de programme énergie et climat',
    org: 'Institut de la Francophonie pour le développement durable',
    confirmedAt: '2026-08-04T11:22:00Z',
  }),
  speaker(134, SESSION.ceremonieCloture, PERSON.bakayoko, 10, {
    role: 'moderator',
    job: 'Responsable de la programmation',
    org: 'Institut de la Francophonie pour le développement durable',
    confirmedAt: '2026-08-04T11:45:00Z',
  }),

  // Séances programmées, panels non encore confirmés
  speaker(140, SESSION.villesMekong, PERSON.tranVanMinh, 10, {
    role: 'moderator',
    job: 'Secrétaire général',
    org: 'Consortium des villes durables du Mékong francophone',
    confirmedAt: null,
  }),
  speaker(141, SESSION.chaleurHopitaux, PERSON.benAmor, 10, {
    role: 'keynote',
    job: 'Directeur de recherche',
    org: "Institut méditerranéen de recherche sur l'eau",
    confirmedAt: null,
  }),
  speaker(142, SESSION.contentieuxClimatique, PERSON.lemoine, 10, {
    role: 'keynote',
    job: 'Professeure de droit international de l’environnement',
    org: 'Chaire universitaire de droit climatique de Montréal',
    confirmedAt: null,
  }),
] satisfies SessionSpeaker[]
