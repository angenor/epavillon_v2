/**
 * Les dix séances PROGRAMMÉES MAIS PAS ENCORE PUBLIÉES (`published_at` nul).
 *
 * Elles n'apparaissent PAS dans la programmation publique — `v_public_schedule`
 * ne retient que les sessions publiées — mais elles existent dans le
 * planificateur (A9), où l'équipe les déplace, les complète et décide du moment
 * de les rendre visibles. C'est la moitié du travail de l'écran : distinguer ce
 * qui est arrêté de ce qui ne l'est pas encore.
 *
 * CINQ D'ENTRE ELLES N'ONT PAS DE SALLE (`room_id` nul), et c'est l'état normal
 * d'une activité retenue mais pas encore installée : le dossier a été accepté,
 * la séance existe avec le créneau souhaité par l'organisation, et l'équipe n'a
 * pas encore décidé où elle se tiendrait. Ce sont ELLES que le planificateur
 * (A9) présente dans son panneau « à placer », et qu'un glisser-déposer installe
 * dans une salle. Sans ce cas dans les données, le panneau latéral de l'écran
 * serait vide et son compteur afficherait zéro.
 *
 * Conséquences vérifiables ailleurs : `detect_conflicts()` ne leur oppose aucun
 * conflit de stand — une séance sans salle n'occupe rien —, et
 * `publication_readiness()` les réclame toutes les cinq, ce qui est exactement
 * ce que le récapitulatif de publication doit montrer.
 *
 * Deux cas limites à traiter :
 *   - `webinairePreparatoire` se tient AVANT l'édition, le 28 octobre : sa date
 *     ne correspond à aucun jour du calendrier, donc `event_day_id` reste nul.
 *     Un écran qui groupe par jour doit prévoir cette colonne.
 *   - `releveJeunesse2` occupe la salle VIRTUELLE en même temps qu'une séance
 *     physique. Aucun conflit de SALLE : une salle virtuelle accepte les
 *     créneaux simultanés. En revanche, `detect_conflicts()` la remonte en
 *     `venue_capacity` — la règle « un seul stand » y vise toute paire de
 *     sessions simultanées de l'édition, en ligne comprise. Écart consigné dans
 *     `docs/progression/ecrans/a9-planificateur.md` : l'interface doit pouvoir distinguer une séance qui
 *     occupe le stand d'une séance qui ne l'occupe pas.
 */

import type { Session } from '~/types/programme/session'
import { ORG, PERSON, PROPOSAL, ROOM, SESSION } from '../ids'
import { session } from './_shared'

export const plannedSessions = [
  session({
    // AVANT l'édition : aucun jour de calendrier ne lui correspond.
    id: SESSION.webinairePreparatoire,
    proposal: null,
    organization: ORG.ifdd,
    status: 'planned',
    title: {
      fr: 'Webinaire préparatoire : ce qui se joue à la COP31',
      en: 'Preparatory webinar: what is at stake at COP31',
    },
    slug: 'webinaire-preparatoire-cop31',
    summary: {
      fr: "Séance de mise à niveau pour les organisations qui participent au pavillon, deux semaines avant l'ouverture.",
    },
    format: 'online',
    startsAt: '2027-10-28T14:00:00-03:00',
    endsAt: '2027-10-28T15:30:00-03:00',
    room: ROOM.atelier,
    registration: { required: true, capacity: 300, waitlist: false },
    recorded: true,
    createdBy: PERSON.tremblay,
    createdAt: '2026-08-05T09:00:00Z',
  }),

  // --- 17 novembre ---------------------------------------------------------
  session({
    id: SESSION.villesMekong,
    proposal: PROPOSAL.villesMekong,
    organization: ORG.cvdmf,
    status: 'planned',
    title: {
      fr: 'Plans de refroidissement urbain : trois villes du Mékong francophone',
      en: 'Urban cooling plans: three cities of the French-speaking Mekong',
    },
    slug: 'refroidissement-urbain-mekong',
    format: 'hybrid',
    startsAt: '2027-11-17T09:30:00-03:00',
    endsAt: '2027-11-17T11:00:00-03:00',
    room: ROOM.stand,
    registration: { required: true, capacity: 80, waitlist: true },
    createdBy: PERSON.nkoDiop,
    createdAt: '2026-08-04T11:00:00Z',
  }),
  session({
    // Salle VIRTUELLE, en même temps qu'une séance physique : concomitance
    // parfaitement légitime, aucun conflit à signaler.
    id: SESSION.releveJeunesse2,
    proposal: PROPOSAL.releveJeunesse,
    organization: ORG.ujfc,
    sequence: 2,
    status: 'planned',
    title: {
      fr: 'La relève francophone dans les négociations climatiques — séance en ligne',
      en: 'The next French-speaking generation in climate negotiations — online session',
    },
    slug: 'releve-francophone-negociations-en-ligne',
    summary: { fr: "Deuxième occurrence, destinée aux délégations restées au pays." },
    format: 'online',
    startsAt: '2027-11-17T09:30:00-03:00',
    endsAt: '2027-11-17T11:00:00-03:00',
    room: ROOM.atelier,
    registration: { required: true, capacity: 300, waitlist: false },
    recorded: true,
    createdBy: PERSON.nkoDiop,
    createdAt: '2026-08-04T11:05:00Z',
  }),
  session({
    id: SESSION.rapportsBiennaux1,
    proposal: PROPOSAL.rapportsBiennaux,
    organization: ORG.ifdd,
    sequence: 1,
    status: 'planned',
    title: {
      fr: 'Préparer son rapport biennal de transparence — première séance',
      en: 'Preparing your biennial transparency report — first session',
    },
    slug: 'atelier-rapport-biennal-1',
    summary: { fr: "Atelier sur poste, vingt participants, avec les données réelles des délégations." },
    format: 'in_person',
    startsAt: '2027-11-17T14:00:00-03:00',
    endsAt: '2027-11-17T16:00:00-03:00',
    // À PLACER : dossier retenu, créneau souhaité recopié, salle non encore
    // attribuée. Le planificateur la présente dans son panneau latéral.
    room: null,
    registration: { required: true, capacity: 20, waitlist: true, closesAt: '2027-11-15T18:00:00-03:00' },
    questions: false,
    createdBy: PERSON.tremblay,
    createdAt: '2026-08-04T11:10:00Z',
  }),

  // --- 18 novembre ---------------------------------------------------------
  session({
    id: SESSION.chaleurHopitaux,
    proposal: PROPOSAL.chaleurHopitaux,
    organization: ORG.imre,
    status: 'planned',
    title: {
      fr: 'Chaleur extrême : préparer les systèmes hospitaliers',
      en: 'Extreme heat: preparing hospital systems',
    },
    slug: 'chaleur-extreme-hopitaux',
    format: 'in_person',
    startsAt: '2027-11-18T09:30:00-03:00',
    endsAt: '2027-11-18T11:00:00-03:00',
    // À PLACER : dossier retenu, créneau souhaité recopié, salle non encore
    // attribuée. Le planificateur la présente dans son panneau latéral.
    room: null,
    registration: { required: true, capacity: 80, waitlist: true },
    createdBy: PERSON.nkoDiop,
    createdAt: '2026-08-04T11:15:00Z',
  }),
  session({
    id: SESSION.contentieuxClimatique,
    proposal: PROPOSAL.contentieuxClimatique,
    organization: ORG.cudcm,
    status: 'planned',
    title: {
      fr: "Contentieux climatique : où en est la responsabilité des États ?",
      en: 'Climate litigation: where does state responsibility stand?',
    },
    slug: 'contentieux-climatique-etats',
    summary: {
      fr: "Les décisions récentes déplacent la charge de la preuve ; les administrations francophones n'y sont pas préparées.",
    },
    format: 'hybrid',
    startsAt: '2027-11-18T14:00:00-03:00',
    endsAt: '2027-11-18T15:30:00-03:00',
    // À PLACER : dossier retenu, créneau souhaité recopié, salle non encore
    // attribuée. Le planificateur la présente dans son panneau latéral.
    room: null,
    registration: { required: true, capacity: 80, waitlist: true },
    streamed: true,
    recorded: true,
    createdBy: PERSON.nkoDiop,
    createdAt: '2026-08-04T11:25:00Z',
  }),

  // --- 19 et 20 novembre ---------------------------------------------------
  session({
    id: SESSION.atelierNegociation2,
    proposal: null,
    organization: ORG.ifdd,
    status: 'planned',
    title: {
      fr: 'Atelier de négociation : préparer la dernière ligne droite',
      en: 'Negotiation workshop: preparing the final stretch',
    },
    slug: 'atelier-negociation-19-novembre',
    format: 'in_person',
    startsAt: '2027-11-19T09:30:00-03:00',
    endsAt: '2027-11-19T11:30:00-03:00',
    // À PLACER : dossier retenu, créneau souhaité recopié, salle non encore
    // attribuée. Le planificateur la présente dans son panneau latéral.
    room: null,
    registration: { required: true, capacity: 20, waitlist: true },
    questions: false,
    createdBy: PERSON.tremblay,
    createdAt: '2026-08-04T11:30:00Z',
  }),
  session({
    // Seconde occurrence de l'atelier, placée le 19 pour ne chevaucher aucune
    // autre séance : les seuls chevauchements de ce jeu de données sont ceux
    // qu'on a voulus.
    id: SESSION.rapportsBiennaux2,
    proposal: PROPOSAL.rapportsBiennaux,
    organization: ORG.ifdd,
    sequence: 2,
    status: 'planned',
    title: {
      fr: 'Préparer son rapport biennal de transparence — seconde séance',
      en: 'Preparing your biennial transparency report — second session',
    },
    slug: 'atelier-rapport-biennal-2',
    format: 'in_person',
    startsAt: '2027-11-19T14:00:00-03:00',
    endsAt: '2027-11-19T16:00:00-03:00',
    // À PLACER : dossier retenu, créneau souhaité recopié, salle non encore
    // attribuée. Le planificateur la présente dans son panneau latéral.
    room: null,
    registration: { required: true, capacity: 20, waitlist: true, closesAt: '2027-11-17T18:00:00-03:00' },
    questions: false,
    createdBy: PERSON.tremblay,
    createdAt: '2026-08-04T11:20:00Z',
  }),
  session({
    id: SESSION.pointPresse2,
    proposal: null,
    organization: ORG.ifdd,
    status: 'planned',
    title: { fr: 'Point de presse quotidien', en: 'Daily press briefing' },
    slug: 'point-presse-19-novembre',
    format: 'in_person',
    startsAt: '2027-11-19T17:30:00-03:00',
    endsAt: '2027-11-19T18:00:00-03:00',
    room: ROOM.stand,
    createdBy: PERSON.nkoDiop,
    createdAt: '2026-08-04T11:35:00Z',
  }),
  session({
    id: SESSION.ceremonieCloture,
    proposal: null,
    organization: ORG.ifdd,
    status: 'planned',
    title: {
      fr: 'Clôture du pavillon et bilan de la programmation',
      en: 'Pavilion closing and programme review',
    },
    slug: 'cloture-pavillon',
    summary: { fr: "Restitution des douze jours, remerciements et annonce de l'édition suivante." },
    format: 'hybrid',
    startsAt: '2027-11-20T16:00:00-03:00',
    endsAt: '2027-11-20T17:30:00-03:00',
    room: ROOM.stand,
    streamed: true,
    recorded: true,
    questions: false,
    createdBy: PERSON.bakayoko,
    createdAt: '2026-08-04T11:40:00Z',
  }),
] satisfies Session[]
