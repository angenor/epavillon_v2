/**
 * Séances des AUTRES éditions : les deux COP passées et le cycle de webinaires.
 *
 * POURQUOI ELLES EXISTENT (prompt A3). La page publique d'une édition porte deux
 * exigences que les trente séances de la COP31 ne pouvaient pas éprouver :
 *
 *   · UN SÉLECTEUR D'ANNÉE — « la programmation des éditions PRÉCÉDENTES reste
 *     consultable depuis la même page ». Sans programme archivé, le sélecteur
 *     n'aurait qu'une entrée et ne prouverait rien ;
 *   · UNE SECTION « AUTRES » — les webinaires et cycles organisés directement
 *     par l'IFDD n'appartiennent à aucune COP et ne doivent pas pour autant
 *     disparaître de la programmation publique.
 *
 * CE QUE CE JEU DE DONNÉES MET À L'ÉPREUVE, et que les autres fichiers ne font
 * pas :
 *   · des séances RÉELLEMENT PASSÉES — `temporal_state = 'past'`. Les trente
 *     séances de la COP31 se tiennent en novembre 2027 : toutes « à venir »,
 *     l'état passé n'était rendu nulle part ;
 *   · un fuseau qui n'est pas celui de Belém (`Asia/Baku`), ce qui vérifie que
 *     l'heure affichée suit l'ÉDITION et non la plateforme ;
 *   · des séances sans jour de calendrier (`event_day_id` nul), le cycle PACO
 *     n'ayant pas de calendrier — ses cinq rendez-vous sont dispersés sur
 *     l'année ;
 *   · une programmation à cheval sur le présent : trois webinaires PACO ont eu
 *     lieu, deux restent à venir. Le même écran doit rendre les deux.
 *
 * Aucune de ces séances ne vient d'un appel à propositions pour le cycle PACO
 * (`proposal_id` nul, l'IFDD programme directement) ; celles des COP passées non
 * plus, leurs dossiers d'origine n'étant pas repris dans ce jeu de données —
 * `programme.sessions.proposal_id` est nullable précisément pour cela.
 */

import type { Session } from '~/types/programme/session'
import { EVENT, ORG, PERSON, PROPOSAL, REGISTRATION_FORM, ROOM, SESSION } from '../ids'
import { session } from './_shared'

// ---------------------------------------------------------------------------
// COP30 — Belém, novembre 2025. Terminée, programme toujours en ligne.
// ---------------------------------------------------------------------------

const cop30 = [
  session({
    id: SESSION.cop30Ouverture,
    event: EVENT.cop30,
    proposal: null,
    organization: ORG.ifdd,
    title: {
      fr: 'Ouverture du pavillon de la Francophonie',
      en: 'Opening of the Francophonie Pavilion',
    },
    slug: 'cop30-ouverture-pavillon',
    summary: {
      fr: "Inauguration du pavillon et présentation des priorités francophones pour la conférence.",
      en: 'Pavilion opening and presentation of Francophonie priorities for the conference.',
    },
    status: 'completed',
    format: 'hybrid',
    startsAt: '2025-11-10T10:00:00-03:00',
    endsAt: '2025-11-10T11:00:00-03:00',
    room: ROOM.cop30Principale,
    streamed: true,
    recorded: true,
    questions: false,
    publishedAt: '2025-10-06T14:00:00Z',
    createdBy: PERSON.nkoDiop,
    createdAt: '2025-10-01T09:00:00Z',
    viewCount: 1_204,
    attendeeCount: 86,
  }),
  session({
    id: SESSION.cop30Adaptation,
    event: EVENT.cop30,
    // Séance issue d'un DOSSIER, comme elle l'a été en réalité : le lien
    // manquait, et l'espace organisation ne pouvait pas remonter de la séance
    // tenue jusqu'à la proposition déposée l'année précédente.
    proposal: PROPOSAL.cop30Littoraux,
    organization: ORG.roac,
    title: {
      fr: "Littoraux d'Afrique de l'Ouest : trois ans de suivi",
      en: 'West African coastlines: three years of monitoring',
    },
    slug: 'cop30-littoraux-afrique-ouest',
    summary: {
      fr: "Restitution du réseau d'observation côtière : ce que trois campagnes de mesure ont changé dans les plans nationaux d'adaptation.",
      en: 'Findings from the coastal observation network and their effect on national adaptation plans.',
    },
    status: 'completed',
    format: 'hybrid',
    startsAt: '2025-11-12T14:00:00-03:00',
    endsAt: '2025-11-12T15:30:00-03:00',
    room: ROOM.cop30Principale,
    recorded: true,
    publishedAt: '2025-10-06T14:00:00Z',
    createdBy: PERSON.perretAdmin,
    createdAt: '2025-10-01T09:10:00Z',
    viewCount: 468,
    attendeeCount: 74,
  }),
  session({
    id: SESSION.cop30Finance,
    event: EVENT.cop30,
    proposal: null,
    organization: ORG.osed,
    title: {
      fr: 'Accréditation directe au Fonds vert : le retour des candidats',
      en: 'Direct accreditation to the Green Climate Fund: applicants speak',
    },
    slug: 'cop30-accreditation-fonds-vert',
    summary: {
      fr: "Quatre organisations racontent leur dossier d'accréditation, du dépôt à la décision, délais et coûts compris.",
      en: 'Four organizations walk through their accreditation files, timelines and costs included.',
    },
    status: 'completed',
    format: 'in_person',
    startsAt: '2025-11-14T11:00:00-03:00',
    endsAt: '2025-11-14T12:30:00-03:00',
    room: ROOM.cop30Principale,
    publishedAt: '2025-10-06T14:00:00Z',
    createdBy: PERSON.perretAdmin,
    createdAt: '2025-10-01T09:20:00Z',
    viewCount: 312,
    attendeeCount: 61,
  }),
  session({
    id: SESSION.cop30Genre,
    event: EVENT.cop30,
    proposal: null,
    organization: ORG.cofemac,
    title: {
      fr: "Budgets climat sensibles au genre : ce qui a marché",
      en: 'Gender-responsive climate budgets: what worked',
    },
    slug: 'cop30-budgets-sensibles-au-genre',
    summary: {
      fr: "Trois ministères des finances francophones présentent leur méthode de marquage budgétaire et ses limites.",
      en: 'Three French-speaking finance ministries present their budget tagging method and its limits.',
    },
    status: 'completed',
    format: 'hybrid',
    startsAt: '2025-11-18T14:00:00-03:00',
    endsAt: '2025-11-18T16:00:00-03:00',
    room: ROOM.cop30Principale,
    streamed: true,
    recorded: true,
    publishedAt: '2025-10-06T14:00:00Z',
    createdBy: PERSON.nkoDiop,
    createdAt: '2025-10-01T09:30:00Z',
    viewCount: 903,
    attendeeCount: 88,
  }),
  session({
    id: SESSION.cop30Cloture,
    event: EVENT.cop30,
    proposal: null,
    organization: ORG.ifdd,
    title: { fr: 'Clôture du pavillon', en: 'Pavilion closing' },
    slug: 'cop30-cloture-pavillon',
    summary: {
      fr: "Bilan des onze jours et annonce du calendrier de préparation de l'édition suivante.",
      en: 'Review of the eleven days and announcement of the next edition’s timeline.',
    },
    status: 'completed',
    format: 'hybrid',
    startsAt: '2025-11-21T16:00:00-03:00',
    endsAt: '2025-11-21T17:00:00-03:00',
    room: ROOM.cop30Principale,
    streamed: true,
    recorded: true,
    publishedAt: '2025-10-06T14:00:00Z',
    createdBy: PERSON.nkoDiop,
    createdAt: '2025-10-01T09:40:00Z',
    viewCount: 541,
    attendeeCount: 52,
  }),
] satisfies Session[]

// ---------------------------------------------------------------------------
// COP29 — Bakou, novembre 2024. Fuseau `Asia/Baku` : +04:00.
// ---------------------------------------------------------------------------

const cop29 = [
  session({
    id: SESSION.cop29Ouverture,
    event: EVENT.cop29,
    timezone: 'Asia/Baku',
    proposal: null,
    organization: ORG.ifdd,
    title: {
      fr: 'Ouverture du pavillon de la Francophonie',
      en: 'Opening of the Francophonie Pavilion',
    },
    slug: 'cop29-ouverture-pavillon',
    summary: {
      fr: "Ouverture de l'espace francophone et présentation du programme des douze jours.",
      en: 'Opening of the French-speaking space and twelve-day programme overview.',
    },
    status: 'completed',
    format: 'hybrid',
    startsAt: '2024-11-11T10:00:00+04:00',
    endsAt: '2024-11-11T11:00:00+04:00',
    room: ROOM.cop29Principale,
    streamed: true,
    recorded: true,
    questions: false,
    publishedAt: '2024-10-14T12:00:00Z',
    createdBy: PERSON.nkoDiop,
    createdAt: '2024-10-08T09:00:00Z',
    viewCount: 786,
    attendeeCount: 64,
  }),
  session({
    id: SESSION.cop29Transparence,
    event: EVENT.cop29,
    timezone: 'Asia/Baku',
    proposal: null,
    organization: ORG.anteb,
    title: {
      fr: 'Premier rapport biennal de transparence : la copie rendue',
      en: 'First biennial transparency report: the submitted copy',
    },
    slug: 'cop29-premier-rapport-transparence',
    summary: {
      fr: "Ce que la rédaction du premier rapport a coûté en données, en personnel et en temps, dans trois administrations.",
      en: 'What drafting the first report cost in data, staff and time, across three administrations.',
    },
    status: 'completed',
    format: 'in_person',
    startsAt: '2024-11-13T11:00:00+04:00',
    endsAt: '2024-11-13T12:30:00+04:00',
    room: ROOM.cop29Principale,
    publishedAt: '2024-10-14T12:00:00Z',
    createdBy: PERSON.perretAdmin,
    createdAt: '2024-10-08T09:10:00Z',
    viewCount: 254,
    attendeeCount: 47,
  }),
  session({
    id: SESSION.cop29Jeunesse,
    event: EVENT.cop29,
    timezone: 'Asia/Baku',
    proposal: null,
    organization: ORG.ujfc,
    title: {
      fr: 'Jeunes négociateurs francophones : première session',
      en: 'Young French-speaking negotiators: first session',
    },
    slug: 'cop29-jeunes-negociateurs',
    summary: {
      fr: "Quinze jeunes délégués rendent compte de leur première conférence : ce qu'ils ont pu faire, et ce qui les en a empêchés.",
      en: 'Fifteen young delegates report on their first conference: what they could do, and what stopped them.',
    },
    status: 'completed',
    format: 'hybrid',
    startsAt: '2024-11-16T14:00:00+04:00',
    endsAt: '2024-11-16T15:30:00+04:00',
    room: ROOM.cop29Principale,
    streamed: true,
    recorded: true,
    publishedAt: '2024-10-14T12:00:00Z',
    createdBy: PERSON.nkoDiop,
    createdAt: '2024-10-08T09:20:00Z',
    viewCount: 619,
    attendeeCount: 58,
  }),
  session({
    id: SESSION.cop29Cloture,
    event: EVENT.cop29,
    timezone: 'Asia/Baku',
    proposal: null,
    organization: ORG.ifdd,
    title: { fr: 'Clôture du pavillon', en: 'Pavilion closing' },
    slug: 'cop29-cloture-pavillon',
    summary: { fr: "Bilan de l'édition et remerciements aux organisations partenaires." },
    status: 'completed',
    format: 'hybrid',
    startsAt: '2024-11-22T16:00:00+04:00',
    endsAt: '2024-11-22T17:00:00+04:00',
    room: ROOM.cop29Principale,
    recorded: true,
    publishedAt: '2024-10-14T12:00:00Z',
    createdBy: PERSON.nkoDiop,
    createdAt: '2024-10-08T09:30:00Z',
    viewCount: 288,
    attendeeCount: 39,
  }),
] satisfies Session[]

// ---------------------------------------------------------------------------
// Cycle PACO 2026 — hors COP, entièrement en ligne, fuseau `America/Toronto`.
//
// Trois rendez-vous ont eu lieu, deux restent à venir : c'est ce qui permet de
// voir sur un même écran une programmation à cheval sur le présent. Le décalage
// change en cours d'année (−05:00 l'hiver, −04:00 l'été) : les heures sont
// écrites avec leur décalage réel, jamais recalculées à la main.
// ---------------------------------------------------------------------------

const paco = [
  session({
    id: SESSION.pacoNegociation,
    event: EVENT.paco2026,
    timezone: 'America/Toronto',
    proposal: null,
    organization: ORG.ifdd,
    title: {
      fr: 'Comprendre le déroulé d’une session de négociation',
      en: 'Understanding how a negotiation session unfolds',
    },
    slug: 'paco-2026-deroule-negociation',
    summary: {
      fr: "Organes, groupes de négociation, documents de travail : de quoi suivre une session sans en connaître les codes.",
      en: 'Bodies, negotiating groups, working documents: how to follow a session without knowing its codes.',
    },
    status: 'completed',
    format: 'online',
    startsAt: '2026-02-12T13:00:00-05:00',
    endsAt: '2026-02-12T14:30:00-05:00',
    room: ROOM.pacoVisio,
    registration: { required: true, capacity: 500, form: REGISTRATION_FORM.default },
    streamed: true,
    recorded: true,
    publishedAt: '2026-01-20T15:00:00Z',
    createdBy: PERSON.tremblay,
    createdAt: '2026-01-15T10:00:00Z',
    viewCount: 1_732,
    attendeeCount: 214,
  }),
  session({
    id: SESSION.pacoFinance,
    event: EVENT.paco2026,
    timezone: 'America/Toronto',
    proposal: null,
    organization: ORG.ifdd,
    title: {
      fr: 'Les guichets de financement climatique, un par un',
      en: 'Climate finance windows, one by one',
    },
    slug: 'paco-2026-guichets-financement',
    summary: {
      fr: "Fonds vert, Fonds d'adaptation, fonds pertes et préjudices : qui peut déposer, sur quels critères, avec quels délais.",
      en: 'Green Climate Fund, Adaptation Fund, loss and damage fund: who may apply, on what criteria, within what timelines.',
    },
    status: 'completed',
    format: 'online',
    startsAt: '2026-04-16T13:00:00-04:00',
    endsAt: '2026-04-16T14:30:00-04:00',
    room: ROOM.pacoVisio,
    registration: { required: true, capacity: 500, form: REGISTRATION_FORM.default },
    streamed: true,
    recorded: true,
    publishedAt: '2026-01-20T15:00:00Z',
    createdBy: PERSON.tremblay,
    createdAt: '2026-01-15T10:05:00Z',
    viewCount: 2_104,
    attendeeCount: 287,
  }),
  session({
    id: SESSION.pacoCdn,
    event: EVENT.paco2026,
    timezone: 'America/Toronto',
    proposal: null,
    organization: ORG.ifdd,
    title: {
      fr: 'Réviser sa contribution déterminée au niveau national',
      en: 'Revising a nationally determined contribution',
    },
    slug: 'paco-2026-reviser-sa-cdn',
    summary: {
      fr: "Méthode, données mobilisées et pièges rencontrés par trois pays qui ont rendu une CDN révisée.",
      en: 'Method, data and pitfalls encountered by three countries that submitted a revised NDC.',
    },
    status: 'completed',
    format: 'online',
    startsAt: '2026-06-11T13:00:00-04:00',
    endsAt: '2026-06-11T14:30:00-04:00',
    room: ROOM.pacoVisio,
    registration: { required: true, capacity: 500, form: REGISTRATION_FORM.default },
    recorded: true,
    publishedAt: '2026-01-20T15:00:00Z',
    createdBy: PERSON.tremblay,
    createdAt: '2026-01-15T10:10:00Z',
    viewCount: 1_488,
    attendeeCount: 176,
  }),
  session({
    id: SESSION.pacoAdaptation,
    event: EVENT.paco2026,
    timezone: 'America/Toronto',
    proposal: null,
    organization: ORG.ifdd,
    title: {
      fr: "Objectif mondial d'adaptation : où en sont les indicateurs",
      en: 'Global goal on adaptation: where the indicators stand',
    },
    slug: 'paco-2026-objectif-mondial-adaptation',
    summary: {
      fr: "État du cadre d'indicateurs et ce qu'il demandera aux administrations francophones dès l'an prochain.",
      en: 'State of the indicator framework and what it will require of administrations from next year.',
    },
    format: 'online',
    startsAt: '2026-09-24T13:00:00-04:00',
    endsAt: '2026-09-24T14:30:00-04:00',
    room: ROOM.pacoVisio,
    registration: {
      required: true,
      capacity: 500,
      opensAt: '2026-08-01T00:00:00-04:00',
      closesAt: '2026-09-23T23:59:59-04:00',
      form: REGISTRATION_FORM.default,
    },
    streamed: true,
    recorded: true,
    publishedAt: '2026-01-20T15:00:00Z',
    createdBy: PERSON.tremblay,
    createdAt: '2026-01-15T10:15:00Z',
    viewCount: 96,
  }),
  session({
    id: SESSION.pacoBilan,
    event: EVENT.paco2026,
    timezone: 'America/Toronto',
    proposal: null,
    organization: ORG.ifdd,
    title: {
      fr: 'Bilan de l’année climatique et perspectives',
      en: 'Climate year in review and outlook',
    },
    slug: 'paco-2026-bilan-annee',
    summary: {
      fr: "Ce qui a été décidé cette année, ce qui reste ouvert, et ce que la prochaine conférence mettra sur la table.",
      en: 'What was decided this year, what remains open, and what the next conference will take up.',
    },
    format: 'online',
    startsAt: '2026-12-10T13:00:00-05:00',
    endsAt: '2026-12-10T14:30:00-05:00',
    room: ROOM.pacoVisio,
    registration: { required: true, capacity: 500, form: REGISTRATION_FORM.default },
    streamed: true,
    recorded: true,
    publishedAt: '2026-01-20T15:00:00Z',
    createdBy: PERSON.tremblay,
    createdAt: '2026-01-15T10:20:00Z',
    viewCount: 41,
  }),
] satisfies Session[]

/** Les quatorze séances des autres éditions, dans l'ordre chronologique. */
export const otherEditionSessions: Session[] = [...cop30, ...cop29, ...paco].sort((a, b) =>
  a.starts_at.localeCompare(b.starts_at),
)
