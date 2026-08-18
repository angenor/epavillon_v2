/**
 * Données simulées du schéma `event`, partie 1 — les séries, les éditions, leur
 * calendrier.
 *
 * L'ÉDITION COURANTE : COP31 Climat, Belém, du 9 au 20 novembre 2027, hybride,
 * fuseau `America/Belem`, pavillon francophone tenu. Toute heure affichée porte
 * ce fuseau : « 14:30 — 16:00, heure de Belém ». Les horaires sont écrits avec
 * leur décalage (`-03:00`) pour que la conversion soit vérifiable à l'œil.
 *
 * RÉSERVE, déjà consignée dans `docs/PROGRESSION.md` : la ville et l'année de
 * cette édition attendent confirmation du commanditaire — Belém a accueilli la
 * COP30. Ces valeurs sont celles du prompt A0.3 ; elles se changent ici, à un
 * seul endroit, le jour où l'arbitrage est rendu. Le prompt A3 ajoutant les
 * éditions passées, la COP30 y figure à sa date et à son lieu RÉELS (Belém,
 * novembre 2025) : la contradiction devient visible à l'écran, dans le sélecteur
 * d'année, plutôt que d'attendre son tour dans un tableau de suivi.
 *
 * L'APPEL EST ENCORE OUVERT alors qu'une partie de la programmation est déjà
 * publiée. Ce n'est pas une incohérence : l'IFDD publie une première version du
 * programme dès les premières décisions et continue de recevoir des dossiers
 * pour les créneaux restants. Les écrans doivent tenir les deux à la fois.
 *
 * TROIS AUTRES ÉDITIONS, ajoutées au prompt A3 :
 *   · COP30 (Belém, novembre 2025) et COP29 (Bakou, novembre 2024), terminées et
 *     programmes publiés — le sélecteur d'année de la page publique les rend
 *     consultables sans quitter l'écran ;
 *   · le cycle de webinaires PACO 2026, qui N'APPARTIENT À AUCUNE COP. Sa série
 *     est de genre `webinar_series`, l'édition ne tient pas de pavillon
 *     (`has_pavilion = false`) et n'ouvre donc aucun appel à propositions. C'est
 *     ce que la page publique présente sous « Autres activités » : elles ne
 *     doivent pas disparaître de la programmation parce qu'elles ne relèvent pas
 *     d'une conférence.
 */

import type { EventDay, EventEdition } from '~/types/event/edition'
import type { EventSeries } from '~/types/event/series'
import type { IsoDate } from '~/types/shared'
import { COUNTRY, EVENT, EVENT_DAY, EVENT_DAY_COP29, EVENT_DAY_COP30, PERSON, SERIES } from './ids'

// ---------------------------------------------------------------------------
// Série — reprise de `900_seed.sql`
// ---------------------------------------------------------------------------

export const eventSeries = [
  {
    id: SERIES.copClimate,
    code: 'cop_climate',
    kind: 'cop_climate',
    name: { fr: 'COP Climat (CCNUCC)', en: 'Climate COP (UNFCCC)' },
    description: {
      fr: "Conférence des Parties à la Convention-cadre des Nations unies sur les changements climatiques. L'OIF y tient un pavillon francophone.",
      en: 'Conference of the Parties to the UNFCCC.',
    },
    slug: 'cop-climat',
    track_code: 'climate',
    organizer_organization_id: null,
    is_active: true,
    created_at: '2026-01-12T09:00:00Z',
    updated_at: '2026-01-12T09:00:00Z',
  },
  {
    // Cycle périodique organisé par l'IFDD lui-même, sans conférence hôte. Le
    // modèle prévoit ce genre depuis l'origine (`event.series_kind`) ; ce qui
    // manquait, c'était une donnée pour l'éprouver.
    id: SERIES.paco,
    code: 'paco',
    kind: 'webinar_series',
    name: {
      fr: 'PACO — Préparation à l’action climatique et aux ODD',
      en: 'PACO — Climate action and SDG preparation series',
    },
    description: {
      fr: "Cycle de webinaires de l'IFDD destiné aux négociateurs, aux points focaux nationaux et aux organisations de la société civile francophones. Il se tient toute l'année, indépendamment des conférences.",
      en: 'IFDD webinar series for French-speaking negotiators, national focal points and civil society organizations, running year-round and independently of conferences.',
    },
    slug: 'paco',
    track_code: 'climate',
    organizer_organization_id: null,
    is_active: true,
    created_at: '2026-01-12T09:05:00Z',
    updated_at: '2026-01-12T09:05:00Z',
  },
] satisfies EventSeries[]

// ---------------------------------------------------------------------------
// L'édition
// ---------------------------------------------------------------------------

export const events = [
  {
    id: EVENT.cop31,
    series_id: SERIES.copClimate,
    edition_label: 'COP31',
    edition_year: 2027,
    title: {
      fr: 'COP31 — Conférence des Nations unies sur les changements climatiques',
      en: 'COP31 — United Nations Climate Change Conference',
    },
    // Le sigle préfixe le numéro de dossier communiqué aux organisations
    // (« COP31-00147 ») : sans lui, la base retombe sur les huit premiers
    // caractères du slug. Écart n°9 de `docs/PROGRESSION.md`.
    acronym: 'COP31',
    slug: 'cop31-belem-2027',
    description: {
      fr: "Trente et unième Conférence des Parties à la CCNUCC. L'Institut de la Francophonie pour le développement durable y tient le pavillon de la Francophonie : douze jours d'activités en français, ouvertes aux délégations, aux organisations de la société civile et au public en ligne.",
      en: 'Thirty-first Conference of the Parties to the UNFCCC. IFDD runs the Francophonie pavilion: twelve days of activities in French, open to delegations, civil society organizations and online audiences.',
    },
    status: 'announced',
    participation_mode: 'hybrid',
    timezone: 'America/Belem',
    starts_at: '2027-11-09T09:00:00-03:00',
    ends_at: '2027-11-20T18:00:00-03:00',
    country_id: COUNTRY.br,
    city: 'Belém',
    address: 'Parc des expositions du Hangar, avenida Doutor Freitas, Belém, Pará',
    // POINT RELEVÉ SUR PLACE, et non géocodé depuis l'adresse : le parc des
    // expositions couvre plusieurs hectares, et son adresse postale place un
    // marqueur loin du pavillon.
    latitude: -1.455833,
    longitude: -48.503889,
    has_pavilion: true,
    // Première version du programme rendue publique ; elle s'enrichit au fil des
    // décisions du comité.
    programme_published_at: '2026-08-03T14:00:00Z',
    highlights: {
      fr: "Le pavillon accueille chaque jour une session d'ouverture à 9 h et un point de presse à 17 h 30. L'accès aux salles physiques exige un badge CCNUCC ; les sessions en ligne sont ouvertes à tous, sur inscription.",
      en: 'The pavilion opens each day at 9 a.m. and closes with a press briefing at 5:30 p.m. Physical access requires a UNFCCC badge; online sessions are open to all, upon registration.',
    },
    created_by: PERSON.bakayoko,
    created_at: '2026-02-10T10:00:00Z',
    updated_at: '2026-08-03T14:00:00Z',
  },

  // -------------------------------------------------------------------------
  // Éditions passées — consultables depuis le sélecteur d'année de la page
  // publique. `status: 'completed'`, programme publié : une COP terminée garde
  // son programme en ligne, c'est la mémoire de ce que la Francophonie y a
  // porté. L'appel à propositions, lui, est clos.
  // -------------------------------------------------------------------------
  {
    id: EVENT.cop30,
    series_id: SERIES.copClimate,
    edition_label: 'COP30',
    edition_year: 2025,
    title: {
      fr: 'COP30 — Conférence des Nations unies sur les changements climatiques',
      en: 'COP30 — United Nations Climate Change Conference',
    },
    acronym: 'COP30',
    slug: 'cop30-belem-2025',
    description: {
      fr: "Trentième Conférence des Parties à la CCNUCC, tenue à Belém. Le pavillon de la Francophonie y a accueilli soixante-douze activités en onze jours.",
      en: 'Thirtieth Conference of the Parties to the UNFCCC, held in Belém. The Francophonie pavilion hosted seventy-two activities over eleven days.',
    },
    status: 'completed',
    participation_mode: 'hybrid',
    timezone: 'America/Belem',
    starts_at: '2025-11-10T09:00:00-03:00',
    ends_at: '2025-11-21T18:00:00-03:00',
    country_id: COUNTRY.br,
    city: 'Belém',
    address: 'Parc du Hangar, avenida Doutor Freitas, Belém, Pará',
    latitude: null,
    longitude: null,
    has_pavilion: true,
    programme_published_at: '2025-10-06T14:00:00Z',
    highlights: null,
    created_by: PERSON.bakayoko,
    created_at: '2025-03-11T10:00:00Z',
    updated_at: '2025-11-24T09:00:00Z',
  },
  {
    id: EVENT.cop29,
    series_id: SERIES.copClimate,
    edition_label: 'COP29',
    edition_year: 2024,
    title: {
      fr: 'COP29 — Conférence des Nations unies sur les changements climatiques',
      en: 'COP29 — United Nations Climate Change Conference',
    },
    acronym: 'COP29',
    slug: 'cop29-bakou-2024',
    description: {
      fr: "Vingt-neuvième Conférence des Parties à la CCNUCC, tenue à Bakou. Édition consacrée au nouvel objectif collectif de financement.",
      en: 'Twenty-ninth Conference of the Parties to the UNFCCC, held in Baku, focused on the new collective finance goal.',
    },
    status: 'completed',
    participation_mode: 'hybrid',
    // Fuseau différent de celui des autres éditions : c'est ce qui prouve que le
    // fuseau est porté par l'ÉDITION et non par la plateforme.
    timezone: 'Asia/Baku',
    starts_at: '2024-11-11T09:00:00+04:00',
    ends_at: '2024-11-22T18:00:00+04:00',
    country_id: COUNTRY.az,
    city: 'Bakou',
    address: 'Stade olympique de Bakou, zone bleue',
    latitude: null,
    longitude: null,
    has_pavilion: true,
    programme_published_at: '2024-10-14T12:00:00Z',
    highlights: null,
    created_by: PERSON.bakayoko,
    created_at: '2024-03-18T10:00:00Z',
    updated_at: '2024-11-25T09:00:00Z',
  },

  // -------------------------------------------------------------------------
  // Hors COP — le cycle de webinaires
  //
  // Aucun pavillon, donc aucun appel à propositions : l'IFDD programme
  // directement (les séances ont `proposal_id` nul). Aucune ville non plus, le
  // cycle étant entièrement en ligne — `ck_events_physical_location` ne l'exige
  // que pour les éditions qui ne sont pas `online`.
  // -------------------------------------------------------------------------
  {
    id: EVENT.paco2026,
    series_id: SERIES.paco,
    edition_label: 'PACO 2026',
    edition_year: 2026,
    title: {
      fr: 'PACO 2026 — Cycle de webinaires de préparation',
      en: 'PACO 2026 — Preparatory webinar series',
    },
    acronym: 'PACO26',
    slug: 'paco-2026',
    description: {
      fr: "Cinq rendez-vous en ligne répartis sur l'année, ouverts à tous et sans frais : négociation, financement, contributions déterminées au niveau national, adaptation, puis bilan avant la conférence.",
      en: 'Five free online sessions spread over the year: negotiation, finance, nationally determined contributions, adaptation, and a pre-conference review.',
    },
    status: 'ongoing',
    participation_mode: 'online',
    timezone: 'America/Toronto',
    starts_at: '2026-02-12T13:00:00-05:00',
    ends_at: '2026-12-10T15:00:00-05:00',
    country_id: null,
    city: null,
    address: null,
    latitude: null,
    longitude: null,
    has_pavilion: false,
    programme_published_at: '2026-01-20T15:00:00Z',
    highlights: {
      fr: "Les webinaires sont enregistrés et rediffusés ; l'inscription reste ouverte jusqu'à la veille de chaque séance.",
      en: 'Sessions are recorded and replayed; registration stays open until the day before each session.',
    },
    created_by: PERSON.tremblay,
    created_at: '2025-12-08T14:00:00Z',
    updated_at: '2026-06-15T10:00:00Z',
  },
  {
    /*
     * ÉDITION ANNONCÉE, PROGRAMME NON PUBLIÉ — et c'est sa raison d'être.
     *
     * La page publique doit tenir ce cas : « tant que le programme n'est pas
     * publié, la section reste présente et annonce qu'il le sera après
     * sélection ». Sans une édition dans cet état, cette branche de l'écran
     * n'aurait aucune donnée et ne serait jamais vue. Le cycle suivant est
     * annoncé — les dates de principe sont connues — mais ses webinaires ne sont
     * pas encore arrêtés : `programme_published_at` est nul, et il n'existe
     * aucune séance.
     */
    id: EVENT.paco2027,
    series_id: SERIES.paco,
    edition_label: 'PACO 2027',
    edition_year: 2027,
    title: {
      fr: 'PACO 2027 — Cycle de webinaires de préparation',
      en: 'PACO 2027 — Preparatory webinar series',
    },
    acronym: 'PACO27',
    slug: 'paco-2027',
    description: {
      fr: "Le cycle reprend en février 2027. Les sujets et les dates des séances seront publiés à l'automne 2026.",
      en: 'The series resumes in February 2027. Topics and dates will be published in autumn 2026.',
    },
    status: 'announced',
    participation_mode: 'online',
    timezone: 'America/Toronto',
    starts_at: '2027-02-11T13:00:00-05:00',
    ends_at: '2027-12-09T15:00:00-05:00',
    country_id: null,
    city: null,
    address: null,
    latitude: null,
    longitude: null,
    has_pavilion: false,
    programme_published_at: null,
    highlights: null,
    created_by: PERSON.tremblay,
    created_at: '2026-07-02T14:00:00Z',
    updated_at: '2026-07-02T14:00:00Z',
  },
] satisfies EventEdition[]

// ---------------------------------------------------------------------------
// Calendrier
//
// Une ligne par jour, rien de plus. UNE JOURNÉE SPÉCIALE N'EST PAS UN JOUR DU
// CALENDRIER : « Journée finance durable » est un fil composé à la main dans
// `mocks/tracks.ts`, et toutes les activités du 12 novembre n'en font pas partie.
// ---------------------------------------------------------------------------

/** Raccourci d'écriture : un jour ordinaire du calendrier. */
function day(
  id: string,
  day_date: string,
  sort_order: number,
  options: Partial<Pick<EventDay, 'title' | 'slug' | 'description' | 'is_featured' | 'color_hex'>> = {},
): EventDay {
  return {
    id,
    event_id: EVENT.cop31,
    day_date,
    title: options.title ?? null,
    slug: options.slug ?? null,
    description: options.description ?? null,
    is_featured: options.is_featured ?? false,
    color_hex: options.color_hex ?? null,
    sort_order,
    created_at: '2026-02-10T10:05:00Z',
    updated_at: '2026-02-10T10:05:00Z',
  }
}

/**
 * Calendrier complet d'une édition passée : un jour par date, sans titre ni
 * couleur. Écrit comme une boucle plutôt qu'à la main — douze lignes vides
 * recopiées n'apprennent rien à qui lit ce fichier, et une date sautée y
 * passerait inaperçue.
 */
function calendar(
  eventId: string,
  year: number,
  month: number,
  fromDay: number,
  toDay: number,
  id: (dayOfMonth: number) => string,
): EventDay[] {
  const days: EventDay[] = []
  for (let d = fromDay; d <= toDay; d += 1) {
    const day_date = `${year}-${String(month).padStart(2, '0')}-${String(d).padStart(2, '0')}` as IsoDate
    days.push({
      id: id(d),
      event_id: eventId,
      day_date,
      title: null,
      slug: null,
      description: null,
      is_featured: false,
      color_hex: null,
      sort_order: (d - fromDay + 1) * 10,
      created_at: `${year}-0${month === 11 ? 3 : 1}-01T10:00:00Z`,
      updated_at: `${year}-0${month === 11 ? 3 : 1}-01T10:00:00Z`,
    })
  }
  return days
}

export const eventDays = [
  day(EVENT_DAY.nov09, '2027-11-09', 10, {
    title: { fr: 'Ouverture du pavillon', en: 'Pavilion opening' },
    slug: 'ouverture',
    description: {
      fr: "Inauguration du pavillon de la Francophonie et présentation du programme des douze jours.",
    },
    is_featured: true,
    color_hex: '#0B6C9E',
  }),
  day(EVENT_DAY.nov10, '2027-11-10', 20),
  day(EVENT_DAY.nov11, '2027-11-11', 30),
  day(EVENT_DAY.nov12, '2027-11-12', 40, {
    title: { fr: 'Journée finance', en: 'Finance day' },
    slug: 'journee-finance',
    description: {
      fr: "Journée de la CCNUCC consacrée à la finance : le pavillon y adosse sa journée spéciale.",
    },
    is_featured: true,
    color_hex: '#1F5F8B',
  }),
  day(EVENT_DAY.nov13, '2027-11-13', 50),
  day(EVENT_DAY.nov14, '2027-11-14', 60),
  day(EVENT_DAY.nov15, '2027-11-15', 70, {
    title: { fr: 'Journée de repos', en: 'Rest day' },
    slug: 'repos',
    description: { fr: "Pas de séance de négociation ; le pavillon accueille des visites de terrain." },
  }),
  day(EVENT_DAY.nov16, '2027-11-16', 80, {
    title: { fr: 'Journée jeunesse', en: 'Youth day' },
    slug: 'journee-jeunesse',
    is_featured: true,
    color_hex: '#8C6D1F',
  }),
  day(EVENT_DAY.nov17, '2027-11-17', 90),
  day(EVENT_DAY.nov18, '2027-11-18', 100),
  day(EVENT_DAY.nov19, '2027-11-19', 110, {
    title: { fr: 'Segment de haut niveau', en: 'High-level segment' },
    slug: 'segment-haut-niveau',
    is_featured: true,
    color_hex: '#5B4A9E',
  }),
  day(EVENT_DAY.nov20, '2027-11-20', 120, {
    title: { fr: 'Clôture', en: 'Closing' },
    slug: 'cloture',
    is_featured: true,
    color_hex: '#0B6C9E',
  }),

  // Calendriers des éditions passées : une ligne par jour, sans titre. Une COP
  // archivée n'a plus besoin d'être commentée jour par jour, mais son calendrier
  // reste complet — la vue grille regroupe par jour, et un jour sans séance doit
  // pouvoir être constaté plutôt que deviné.
  ...calendar(EVENT.cop30, 2025, 11, 10, 21, EVENT_DAY_COP30),
  ...calendar(EVENT.cop29, 2024, 11, 11, 22, EVENT_DAY_COP29),

  // Le cycle PACO n'a AUCUN jour de calendrier, et c'est juste : ses cinq
  // webinaires sont dispersés sur l'année, ils ne composent pas une conférence.
  // Leurs séances portent donc `event_day_id = null` — cas que la page publique
  // doit tenir sans se rabattre sur un jour inventé.
] satisfies EventDay[]
