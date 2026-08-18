/**
 * Les vingt séances DÉJÀ PUBLIÉES : elles alimentent la programmation
 * publique (`v_public_schedule` ne retient que les sessions dont `published_at`
 * est renseigné) et la page de l'événement.
 *
 * DEUX CONFLITS SONT VOLONTAIRES, et ils ne se ressemblent pas :
 *
 *   1. CONFLIT DE SALLE — « Restaurer les mangroves » et « Élevage pastoral »
 *      occupent la salle Baobab le 14 novembre de 14 h à 15 h 30. Un seul stand,
 *      une salle à la fois : `detect_conflicts()` remonte une gravité `blocking`.
 *   2. CONFLIT DE DIFFUSION — « Marchés carbone de l'article 6 » et « Accéder au
 *      Fonds vert » sont diffusées en même temps le 12 novembre, sur l'unique
 *      canal de l'édition. Une seule équipe technique, un seul flux.
 *
 * AUCUN DES DEUX N'EST BLOQUÉ À L'ÉCRITURE, et c'est le point : l'équipe
 * arbitre par glisser-déposer, l'interface signale en permanence, et le seul
 * garde-fou dur se situe à la publication du programme
 * (`publication_readiness()`). Une contrainte d'exclusion transformerait
 * l'outil d'arbitrage en mur — le planificateur passe par des états incohérents,
 * c'est sa nature.
 *
 * Une séance est ANNULÉE et une autre REPORTÉE : les deux restent visibles, avec
 * leur motif. Une programmation publique qui fait disparaître une activité
 * annulée laisse les inscrits devant une page introuvable.
 */

import type { Session } from '~/types/programme/session'
import { ORG, PERSON, PROPOSAL, ROOM, SESSION } from '../ids'
import { session } from './_shared'

export const publishedSessions = [
  // --- 9 novembre — ouverture ----------------------------------------------
  session({
    id: SESSION.ouverture,
    proposal: null,
    organization: ORG.ifdd,
    title: {
      fr: 'Ouverture du pavillon de la Francophonie',
      en: 'Opening of the Francophonie Pavilion',
    },
    slug: 'ouverture-pavillon',
    summary: {
      fr: "Inauguration du pavillon, présentation du programme des douze jours et mot des autorités.",
      en: 'Pavilion opening, twelve-day programme overview and official remarks.',
    },
    format: 'hybrid',
    startsAt: '2027-11-09T10:00:00-03:00',
    endsAt: '2027-11-09T11:00:00-03:00',
    room: ROOM.stand,
    streamed: true,
    recorded: true,
    questions: false,
    publishedAt: '2026-08-03T14:00:00Z',
    createdBy: PERSON.nkoDiop,
    createdAt: '2026-07-31T09:00:00Z',
    viewCount: 412,
  }),
  session({
    id: SESSION.pointPresse1,
    proposal: null,
    organization: ORG.ifdd,
    title: { fr: 'Point de presse quotidien', en: 'Daily press briefing' },
    slug: 'point-presse-9-novembre',
    summary: { fr: "Le point sur les négociations de la journée, en français." },
    format: 'in_person',
    startsAt: '2027-11-09T17:30:00-03:00',
    endsAt: '2027-11-09T18:00:00-03:00',
    room: ROOM.stand,
    questions: true,
    publishedAt: '2026-08-03T14:00:00Z',
    createdBy: PERSON.nkoDiop,
    createdAt: '2026-07-31T09:05:00Z',
    viewCount: 86,
  }),

  // --- 10 novembre ---------------------------------------------------------
  session({
    id: SESSION.rencontreNegociateurs,
    proposal: null,
    organization: ORG.ifdd,
    title: {
      fr: 'Rencontre des négociateurs francophones',
      en: 'Meeting of French-speaking negotiators',
    },
    slug: 'rencontre-negociateurs-francophones',
    summary: {
      fr: "Point de coordination quotidien entre délégations francophones, à huis clos.",
    },
    format: 'in_person',
    startsAt: '2027-11-10T09:30:00-03:00',
    endsAt: '2027-11-10T11:00:00-03:00',
    room: ROOM.stand,
    registration: { required: true, capacity: 35, waitlist: false, closesAt: '2027-11-08T23:59:59-03:00' },
    questions: false,
    publishedAt: '2026-08-03T14:00:00Z',
    createdBy: PERSON.perretAdmin,
    createdAt: '2026-07-31T09:10:00Z',
    viewCount: 143,
  }),
  session({
    id: SESSION.adaptationCotiere,
    proposal: PROPOSAL.adaptationCotiere,
    organization: ORG.roac,
    title: {
      fr: "Financer l'adaptation côtière en Afrique de l'Ouest",
      en: 'Financing coastal adaptation in West Africa',
    },
    slug: 'financer-adaptation-cotiere',
    summary: {
      fr: "Deux mille kilomètres de littoral reculent, et les guichets de financement raisonnent encore par projet isolé.",
      en: 'Two thousand kilometres of coastline are receding, while funding windows still think project by project.',
    },
    description: {
      fr: "Un porteur de projet, une agence nationale et un bailleur confrontent un montage régional aux obstacles institutionnels qui l'ont jusqu'ici empêché.",
    },
    format: 'hybrid',
    startsAt: '2027-11-10T14:00:00-03:00',
    endsAt: '2027-11-10T15:30:00-03:00',
    room: ROOM.stand,
    registration: { required: true, capacity: 80, waitlist: true, opensAt: '2026-09-01T00:00:00-03:00' },
    streamed: true,
    recorded: true,
    publishedAt: '2026-08-03T14:00:00Z',
    createdBy: PERSON.nkoDiop,
    createdAt: '2026-07-29T10:00:00Z',
    viewCount: 268,
  }),

  // --- 11 novembre ---------------------------------------------------------
  session({
    id: SESSION.alertePrecoce,
    proposal: PROPOSAL.alertePrecoce,
    organization: ORG.osed,
    title: {
      fr: "Systèmes d'alerte précoce multirisques au Sahel : ce qui fonctionne",
      en: 'Multi-hazard early warning systems in the Sahel: what works',
    },
    slug: 'alerte-precoce-sahel',
    summary: {
      fr: "Une alerte qui n'atteint pas le dernier kilomètre ne sert à rien, quelle que soit la qualité de la prévision.",
    },
    format: 'in_person',
    startsAt: '2027-11-11T09:30:00-03:00',
    endsAt: '2027-11-11T11:00:00-03:00',
    room: ROOM.stand,
    registration: { required: true, capacity: 80, waitlist: true },
    publishedAt: '2026-08-03T14:00:00Z',
    createdBy: PERSON.nkoDiop,
    createdAt: '2026-07-29T10:10:00Z',
    viewCount: 174,
  }),
  session({
    id: SESSION.miniReseaux,
    proposal: PROPOSAL.miniReseaux,
    organization: ORG.osed,
    title: {
      fr: 'Mini-réseaux solaires : des modèles économiques enfin viables ?',
      en: 'Solar mini-grids: finally viable business models?',
    },
    slug: 'mini-reseaux-solaires',
    summary: {
      fr: "Quarante mini-réseaux, huit ans de recul, et un point mort atteint par moins de la moitié d'entre eux.",
    },
    format: 'hybrid',
    startsAt: '2027-11-11T14:00:00-03:00',
    endsAt: '2027-11-11T15:30:00-03:00',
    room: ROOM.stand,
    registration: { required: true, capacity: 80, waitlist: true },
    recorded: true,
    publishedAt: '2026-08-03T14:00:00Z',
    createdBy: PERSON.nkoDiop,
    createdAt: '2026-07-29T10:15:00Z',
    viewCount: 191,
  }),

  // --- 12 novembre — journée finance durable -------------------------------
  session({
    id: SESSION.pertesPrejudices,
    proposal: PROPOSAL.pertesPrejudices,
    organization: ORG.fhrc,
    title: {
      fr: 'Pertes et préjudices : opérationnaliser le fonds pour les pays francophones',
      en: 'Loss and damage: making the fund work for French-speaking countries',
    },
    slug: 'pertes-prejudices-fonds',
    summary: {
      fr: "Le fonds existe. Les dossiers, eux, se rédigent toujours dans des formats qu'aucune administration locale ne maîtrise.",
    },
    format: 'hybrid',
    startsAt: '2027-11-12T09:30:00-03:00',
    endsAt: '2027-11-12T11:00:00-03:00',
    room: ROOM.stand,
    registration: { required: true, capacity: 80, waitlist: true },
    streamed: true,
    recorded: true,
    publishedAt: '2026-08-03T14:00:00Z',
    createdBy: PERSON.nkoDiop,
    createdAt: '2026-07-29T10:20:00Z',
    viewCount: 324,
  }),
  session({
    // CONFLIT DE DIFFUSION (1/2) : même canal, même créneau que la session
    // suivante. Salle différente : le conflit porte sur le direct, pas sur le lieu.
    id: SESSION.article6,
    proposal: PROPOSAL.article6,
    organization: ORG.cudcm,
    title: {
      fr: "Marchés carbone de l'article 6 : quelles garanties d'intégrité ?",
      en: 'Article 6 carbon markets: what integrity safeguards?',
    },
    slug: 'marches-carbone-article-6',
    summary: {
      fr: "Les premières autorisations sont délivrées ; les garde-fous, eux, restent à écrire dans le droit national.",
    },
    format: 'hybrid',
    startsAt: '2027-11-12T14:00:00-03:00',
    endsAt: '2027-11-12T15:30:00-03:00',
    room: ROOM.stand,
    registration: { required: true, capacity: 35, waitlist: true },
    streamed: true,
    recorded: true,
    publishedAt: '2026-08-03T14:00:00Z',
    createdBy: PERSON.nkoDiop,
    createdAt: '2026-07-29T10:25:00Z',
    viewCount: 297,
  }),
  session({
    // CONFLIT DE DIFFUSION (2/2).
    id: SESSION.accesFondsVert,
    proposal: PROPOSAL.accesFondsVert,
    organization: ORG.ifdd,
    title: {
      fr: 'Accéder au Fonds vert sans intermédiaire : accréditation, mode d’emploi',
      en: 'Direct access to the Green Climate Fund: an accreditation walkthrough',
    },
    slug: 'acces-direct-fonds-vert',
    summary: {
      fr: "Trois entités nationales francophones accréditées en cinq ans : pourquoi si peu, et comment faire.",
    },
    format: 'hybrid',
    startsAt: '2027-11-12T14:00:00-03:00',
    endsAt: '2027-11-12T15:30:00-03:00',
    room: ROOM.stand,
    registration: { required: true, capacity: 80, waitlist: true },
    streamed: true,
    recorded: true,
    publishedAt: '2026-08-03T14:00:00Z',
    createdBy: PERSON.nkoDiop,
    createdAt: '2026-07-29T10:30:00Z',
    viewCount: 341,
  }),
  session({
    id: SESSION.bilanFinance,
    proposal: null,
    organization: ORG.ifdd,
    title: {
      fr: 'Bilan de la journée finance durable',
      en: 'Sustainable finance day wrap-up',
    },
    slug: 'bilan-journee-finance',
    summary: { fr: "Restitution des trois sessions de la journée et demandes portées au conseil du fonds." },
    format: 'in_person',
    startsAt: '2027-11-12T16:30:00-03:00',
    endsAt: '2027-11-12T17:30:00-03:00',
    room: ROOM.stand,
    publishedAt: '2026-08-03T14:00:00Z',
    createdBy: PERSON.duchesne,
    createdAt: '2026-07-31T11:00:00Z',
    viewCount: 77,
  }),

  // --- 13 novembre ---------------------------------------------------------
  session({
    id: SESSION.atelierNegociation1,
    proposal: null,
    organization: ORG.ifdd,
    title: {
      fr: 'Atelier de négociation : lire et amender un texte',
      en: 'Negotiation workshop: reading and amending a text',
    },
    slug: 'atelier-negociation-13-novembre',
    summary: { fr: "Atelier pratique sur un texte réellement en discussion. Huit postes de travail." },
    format: 'in_person',
    startsAt: '2027-11-13T09:30:00-03:00',
    endsAt: '2027-11-13T11:30:00-03:00',
    room: ROOM.stand,
    // Huit places seulement : c'est la session qui remplit sa liste d'attente,
    // et la seule où les positions d'attente se vérifient.
    registration: { required: true, capacity: 8, waitlist: true, closesAt: '2027-11-11T23:59:59-03:00' },
    questions: false,
    publishedAt: '2026-08-03T14:00:00Z',
    createdBy: PERSON.tremblay,
    createdAt: '2026-07-31T11:10:00Z',
    viewCount: 118,
  }),
  session({
    id: SESSION.agroecologie,
    proposal: PROPOSAL.agroecologie,
    organization: ORG.anteb,
    title: {
      fr: 'Agroécologie et sécurité alimentaire face aux sécheresses répétées',
      en: 'Agroecology and food security in the face of repeated droughts',
    },
    slug: 'agroecologie-securite-alimentaire',
    summary: { fr: "Six campagnes agricoles comparées, dont deux années sèches : c'est là que l'écart se creuse." },
    format: 'in_person',
    startsAt: '2027-11-13T14:00:00-03:00',
    endsAt: '2027-11-13T15:30:00-03:00',
    room: ROOM.stand,
    registration: { required: true, capacity: 80, waitlist: true },
    publishedAt: '2026-08-03T14:00:00Z',
    createdBy: PERSON.nkoDiop,
    createdAt: '2026-07-29T10:35:00Z',
    viewCount: 152,
  }),

  // --- 14 novembre ---------------------------------------------------------
  session({
    id: SESSION.bassinNiger,
    proposal: PROPOSAL.bassinNiger,
    organization: ORG.imre,
    title: {
      fr: 'Eau et pêche : gérer la rareté dans le bassin du Niger',
      en: 'Water and fisheries: managing scarcity in the Niger Basin',
    },
    slug: 'eau-peche-bassin-niger',
    format: 'in_person',
    startsAt: '2027-11-14T09:30:00-03:00',
    endsAt: '2027-11-14T11:00:00-03:00',
    room: ROOM.stand,
    registration: { required: true, capacity: 80, waitlist: true },
    publishedAt: '2026-08-03T14:00:00Z',
    createdBy: PERSON.nkoDiop,
    createdAt: '2026-07-29T10:40:00Z',
    viewCount: 134,
  }),
  session({
    // CONFLIT DE SALLE (1/2) : Baobab, 14 h — 15 h 30.
    id: SESSION.mangroves,
    proposal: PROPOSAL.mangroves,
    organization: ORG.cofemac,
    title: {
      fr: "Restaurer les mangroves d'Afrique centrale : carbone bleu et moyens d'existence",
      en: 'Restoring Central African mangroves: blue carbon and livelihoods',
    },
    slug: 'restaurer-mangroves-afrique-centrale',
    summary: { fr: "Huit ans après les premières plantations, où sont passés les revenus du carbone bleu ?" },
    format: 'in_person',
    startsAt: '2027-11-14T14:00:00-03:00',
    endsAt: '2027-11-14T15:30:00-03:00',
    room: ROOM.stand,
    registration: { required: true, capacity: 80, waitlist: true },
    publishedAt: '2026-08-03T14:00:00Z',
    createdBy: PERSON.nkoDiop,
    createdAt: '2026-07-29T10:45:00Z',
    viewCount: 163,
  }),
  session({
    // CONFLIT DE SALLE (2/2) : même salle, même créneau. Écrit sans obstacle,
    // signalé en gravité haute, à arbitrer avant publication définitive.
    id: SESSION.pastoralisme,
    proposal: PROPOSAL.pastoralisme,
    organization: ORG.roac,
    title: {
      fr: 'Élevage pastoral et adaptation : concilier mobilité et ressources',
      en: 'Pastoral livestock and adaptation: reconciling mobility and resources',
    },
    slug: 'elevage-pastoral-adaptation',
    format: 'in_person',
    startsAt: '2027-11-14T14:00:00-03:00',
    endsAt: '2027-11-14T15:30:00-03:00',
    room: ROOM.stand,
    registration: { required: true, capacity: 80, waitlist: true },
    publishedAt: '2026-08-03T14:00:00Z',
    createdBy: PERSON.perretAdmin,
    createdAt: '2026-07-30T15:20:00Z',
    updatedAt: '2026-08-14T10:00:00Z',
    viewCount: 128,
  }),

  // --- 15 novembre — journée de repos --------------------------------------
  session({
    // REPORTÉE : la séance reste visible, avec sa raison, jusqu'à sa
    // reprogrammation. La faire disparaître laisserait les inscrits devant une
    // page introuvable.
    id: SESSION.visiteTerrain,
    proposal: null,
    organization: ORG.ifdd,
    status: 'postponed',
    title: {
      fr: "Visite de terrain : restauration d'une zone humide périurbaine",
      en: 'Field visit: restoring a peri-urban wetland',
    },
    slug: 'visite-terrain-zone-humide',
    summary: { fr: "Sortie encadrée, transport assuré depuis le pavillon. Nouvelle date à confirmer." },
    format: 'in_person',
    startsAt: '2027-11-15T09:00:00-03:00',
    endsAt: '2027-11-15T13:00:00-03:00',
    room: null,
    locationNote: {
      fr: "Départ en autocar devant le pavillon, zone bleue, hall B.",
      en: 'Coach departure in front of the pavilion, blue zone, hall B.',
    },
    registration: { required: true, capacity: 25, waitlist: true, closesAt: '2027-11-13T18:00:00-03:00' },
    questions: false,
    publishedAt: '2026-08-03T14:00:00Z',
    createdBy: PERSON.nkoDiop,
    createdAt: '2026-07-31T11:20:00Z',
    updatedAt: '2026-08-15T08:30:00Z',
    viewCount: 95,
  }),
  session({
    // ANNULÉE : le motif est obligatoire en base.
    id: SESSION.cafeFrancophone,
    proposal: null,
    organization: ORG.ifdd,
    status: 'cancelled',
    title: { fr: 'Café francophone', en: 'Francophone coffee break' },
    slug: 'cafe-francophone',
    summary: { fr: "Rencontre informelle entre délégations, organisations et journalistes." },
    format: 'in_person',
    startsAt: '2027-11-15T16:00:00-03:00',
    endsAt: '2027-11-15T17:00:00-03:00',
    room: ROOM.stand,
    cancelledReason: {
      fr: "Le pavillon est fermé au public le jour de repos : la rencontre est reportée au 16 novembre en fin de journée.",
      en: 'The pavilion is closed to the public on the rest day; the gathering moves to 16 November.',
    },
    questions: false,
    publishedAt: '2026-08-03T14:00:00Z',
    createdBy: PERSON.nkoDiop,
    createdAt: '2026-07-31T11:25:00Z',
    updatedAt: '2026-08-14T16:40:00Z',
    viewCount: 61,
  }),

  // --- 16 novembre — journée jeunesse et climat ----------------------------
  session({
    id: SESSION.forumJeunesse,
    proposal: null,
    organization: ORG.ujfc,
    title: {
      fr: 'Forum ouvert de la jeunesse francophone',
      en: 'Open forum of French-speaking youth',
    },
    slug: 'forum-ouvert-jeunesse',
    summary: { fr: "Format ouvert : l'ordre du jour est construit par les participants en début de séance." },
    format: 'in_person',
    startsAt: '2027-11-16T09:30:00-03:00',
    endsAt: '2027-11-16T11:00:00-03:00',
    room: ROOM.stand,
    registration: { required: true, capacity: 35, waitlist: true },
    publishedAt: '2026-08-03T14:00:00Z',
    createdBy: PERSON.nkoDiop,
    createdAt: '2026-07-31T11:30:00Z',
    viewCount: 149,
  }),
  session({
    id: SESSION.transitionJuste,
    proposal: PROPOSAL.transitionJuste,
    organization: ORG.cofemac,
    title: {
      fr: "Genre et transition juste : l'expérience des coopératives féminines",
      en: 'Gender and just transition: the experience of women’s cooperatives',
    },
    slug: 'genre-transition-juste',
    summary: {
      fr: "Quand une filière se ferme, ce sont les emplois informels, très majoritairement féminins, qui disparaissent en premier.",
    },
    format: 'in_person',
    startsAt: '2027-11-16T14:00:00-03:00',
    endsAt: '2027-11-16T15:30:00-03:00',
    room: ROOM.stand,
    registration: { required: true, capacity: 80, waitlist: true },
    recorded: true,
    publishedAt: '2026-08-03T14:00:00Z',
    createdBy: PERSON.nkoDiop,
    createdAt: '2026-07-29T10:50:00Z',
    viewCount: 187,
  }),
  session({
    id: SESSION.releveJeunesse,
    proposal: PROPOSAL.releveJeunesse,
    organization: ORG.ujfc,
    sequence: 1,
    title: {
      fr: 'La relève francophone dans les négociations climatiques',
      en: 'The next French-speaking generation in climate negotiations',
    },
    slug: 'releve-francophone-negociations',
    summary: { fr: "Former des jeunes négociateurs suppose de leur donner un mandat, pas seulement un badge." },
    format: 'hybrid',
    startsAt: '2027-11-16T16:00:00-03:00',
    endsAt: '2027-11-16T17:30:00-03:00',
    room: ROOM.stand,
    registration: { required: true, capacity: 80, waitlist: true },
    streamed: true,
    recorded: true,
    publishedAt: '2026-08-03T14:00:00Z',
    createdBy: PERSON.nkoDiop,
    createdAt: '2026-07-29T10:55:00Z',
    viewCount: 216,
  }),
] satisfies Session[]
