/**
 * Données simulées du schéma `event`, partie 1 — la série, l'édition, son
 * calendrier.
 *
 * L'ÉDITION SIMULÉE : COP31 Climat, Belém, du 9 au 20 novembre 2027, hybride,
 * fuseau `America/Belem`, pavillon francophone tenu. Toute heure affichée porte
 * ce fuseau : « 14:30 — 16:00, heure de Belém ». Les horaires sont écrits avec
 * leur décalage (`-03:00`) pour que la conversion soit vérifiable à l'œil.
 *
 * RÉSERVE, déjà consignée dans `docs/PROGRESSION.md` : la ville et l'année de
 * cette édition attendent confirmation du commanditaire — Belém a accueilli la
 * COP30. Ces valeurs sont celles du prompt A0.3 ; elles se changent ici, à un
 * seul endroit, le jour où l'arbitrage est rendu.
 *
 * L'APPEL EST ENCORE OUVERT alors qu'une partie de la programmation est déjà
 * publiée. Ce n'est pas une incohérence : l'IFDD publie une première version du
 * programme dès les premières décisions et continue de recevoir des dossiers
 * pour les créneaux restants. Les écrans doivent tenir les deux à la fois.
 */

import type { EventDay, EventEdition } from '~/types/event/edition'
import type { EventSeries } from '~/types/event/series'
import { COUNTRY, EVENT, EVENT_DAY, PERSON, SERIES } from './ids'

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
] satisfies EventDay[]
