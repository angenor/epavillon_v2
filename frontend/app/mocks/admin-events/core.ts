/**
 * GESTION DES ÉVÉNEMENTS (A10) — le socle partagé.
 *
 * Ce fichier ne compose aucun écran et n'écrit rien. Il tient ce dont les deux
 * autres ont besoin : les tableaux de données rendus mutables, la fabrique
 * d'identifiants d'exécution, l'arithmétique des dates CIVILES, la ligne de la
 * liste des éditions et les listes de référence du formulaire.
 *
 * POURQUOI TROIS FICHIERS. La composition des six onglets et les quinze écritures
 * qui les nourrissent dépassaient à elles seules le garde-fou de mille lignes de
 * `CLAUDE.md`. Le découpage suit la règle du projet — par écran, puis par
 * responsabilité — et la dépendance ne va que dans un sens :
 *
 *     writes.ts  →  detail.ts  →  core.ts
 *
 * Aucun cycle : les écritures lisent la composition pour la rendre, la
 * composition ne connaît aucune écriture.
 */

import type {
  EditionFormOptions,
  EditionListRow,
  EditionListScreen,
  EditionSeriesOption,
  TimeZoneOption,
} from '~/types/admin-events'
import type { AdministeredEvents } from '~/types/identity'
import type { EventDay, EventEdition, EventStatus, ProgrammeTrack } from '~/types/event/edition'
import type { BroadcastChannel, Room, Venue } from '~/types/event/venue'
import type { CallForProposals, CallReviewer, ReviewCriterion } from '~/types/event/call'
import type { ScheduleThemeBadge } from '~/types/views'
import type { Uuid } from '~/types/shared'
import { mockUuid } from '../ids'
import { eventDays, events, eventSeries } from '../event'
import { broadcastChannels, rooms, venues } from '../rooms'
import { programmeTracks } from '../tracks'
import { callsForProposals, callReviewers } from '../calls'
import { reviewCriteria } from '../criteria'
import { allProposals } from '../proposals'
import { allSessions } from '../sessions'
import { people } from '../people'
import { organizations } from '../org'
import { countries, entityTerms, taxonomyTerms } from '../reference'

// ---------------------------------------------------------------------------
// Tableaux mutables
//
// Les jeux de données sont déclarés avec `satisfies`, ce qui en infère des types
// littéraux : y insérer une ligne construite à l'exécution serait refusé. Ces
// alias disent, à UN SEUL endroit, qu'on se met ici à la place de la base — même
// procédé que `MutableSession` dans `admin-planner.ts`.
// ---------------------------------------------------------------------------

export const editions = events as EventEdition[]
export const days = eventDays as EventDay[]
export const tracks = programmeTracks as ProgrammeTrack[]
export const venueRows = venues as Venue[]
export const roomRows = rooms as Room[]
export const channelRows = broadcastChannels as BroadcastChannel[]
export const calls = callsForProposals as CallForProposals[]
export const criteria = reviewCriteria as ReviewCriterion[]
export const reviewers = callReviewers as CallReviewer[]

export const personById = new Map(people.map((p) => [p.id, p]))
export const organizationById = new Map(organizations.map((o) => [o.id, o]))
export const countryById = new Map(countries.map((c) => [c.id, c]))
export const termById = new Map(taxonomyTerms.map((t) => [t.id, t]))

/** Numéros d'ordre des entités créées à l'exécution — au-delà des jeux de données. */
let runtimeCounter = 900
export const newId = (family: string): Uuid => mockUuid(family, ++runtimeCounter)

// ---------------------------------------------------------------------------
// Dates civiles
//
// Toutes les bornes de cet écran sont des DATES CIVILES dans le fuseau de
// l'édition : le calendrier d'une COP qui commence le 9 novembre à Belém commence
// le 9, pas le 8 au soir en heure de Paris. Une conversion faite avec le fuseau du
// navigateur décalerait un jour sur deux pour quiconque travaille depuis Québec.
// ---------------------------------------------------------------------------

export function civilDate(instant: string, timeZone: string): string {
  return new Intl.DateTimeFormat('en-CA', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    timeZone,
  }).format(new Date(instant))
}

/** Les dates civiles de la période, bornes comprises. */
export function datesBetween(first: string, last: string): string[] {
  const result: string[] = []
  const cursor = new Date(`${first}T12:00:00Z`)
  const end = new Date(`${last}T12:00:00Z`)
  while (cursor.getTime() <= end.getTime()) {
    result.push(cursor.toISOString().slice(0, 10))
    cursor.setUTCDate(cursor.getUTCDate() + 1)
  }
  return result
}

export function periodOf(edition: EventEdition): { first_day: string; last_day: string } {
  return {
    first_day: civilDate(edition.starts_at, edition.timezone),
    last_day: civilDate(edition.ends_at, edition.timezone),
  }
}

/** Thématiques d'une entité, avec libellé et couleur — comme `reference.term_badges()`. */
export function badgesOf(schema: string, table: string, entityId: string): ScheduleThemeBadge[] {
  return entityTerms
    .filter(
      (link) =>
        link.entity_schema === schema && link.entity_table === table && link.entity_id === entityId,
    )
    .map((link) => ({ link, term: termById.get(link.term_id) }))
    .filter((pair) => pair.term?.is_active && pair.term.taxonomy_code === 'activity_theme')
    .sort((a, b) => a.link.sort_order - b.link.sort_order)
    .map((pair) => ({
      code: pair.term!.code,
      label: pair.term!.label,
      color: pair.term!.color_hex,
      icon: pair.term!.icon,
    }))
}

// ---------------------------------------------------------------------------
// 1. LISTE DES ÉDITIONS
// ---------------------------------------------------------------------------

/** Salles d'une édition : elles portent leur LIEU, et c'est le lieu qui porte l'édition. */
export function roomsOfEvent(eventId: Uuid): Room[] {
  const venueIds = new Set(venueRows.filter((v) => v.event_id === eventId).map((v) => v.id))
  return roomRows.filter((r) => venueIds.has(r.venue_id))
}

export function callOf(eventId: Uuid): CallForProposals | null {
  // `ux_calls_one_per_event` exclut les appels annulés : c'est ce qui permet de
  // repartir après une annulation sans supprimer l'historique.
  return calls.find((c) => c.event_id === eventId && c.status !== 'cancelled') ?? null
}

/** Dossiers DÉPOSÉS, brouillons et suppressions exclus. */
export function proposalCountOf(eventId: Uuid): number {
  return allProposals.filter(
    (p) => p.event_id === eventId && p.status !== 'draft' && p.deleted_at === null,
  ).length
}

export function listRow(edition: EventEdition): EditionListRow {
  const series = eventSeries.find((s) => s.id === edition.series_id) ?? null
  const country = edition.country_id ? countryById.get(edition.country_id) : undefined
  const call = callOf(edition.id)
  const sessions = allSessions.filter((s) => s.event_id === edition.id)

  return {
    id: edition.id,
    title: edition.title,
    acronym: edition.acronym,
    slug: edition.slug,
    series_id: edition.series_id,
    series_name: series?.name ?? null,
    series_kind: series?.kind ?? null,
    edition_label: edition.edition_label,
    edition_year: edition.edition_year,
    status: edition.status,
    participation_mode: edition.participation_mode,
    timezone: edition.timezone,
    starts_at: edition.starts_at,
    ends_at: edition.ends_at,
    country_id: edition.country_id,
    country_name: country?.name ?? null,
    city: edition.city,
    address: edition.address,
    latitude: edition.latitude,
    longitude: edition.longitude,
    has_pavilion: edition.has_pavilion,
    programme_published_at: edition.programme_published_at,
    proposal_count: proposalCountOf(edition.id),
    session_count: sessions.length,
    scheduled_session_count: sessions.filter((s) => s.room_id !== null).length,
    call_status: call?.status ?? null,
    // `event.effective_deadline()` — la prolongation d'abord, la clôture ensuite.
    call_deadline: call ? (call.extended_until ?? call.closes_at) : null,
    day_count: days.filter((d) => d.event_id === edition.id).length,
  }
}

export function seriesOptions(): EditionSeriesOption[] {
  return eventSeries
    .map((series) => ({
      id: series.id,
      name: series.name,
      kind: series.kind,
      is_active: series.is_active,
      edition_count: editions.filter((e) => e.series_id === series.id).length,
    }))
    .sort((a, b) => b.edition_count - a.edition_count)
}

/**
 * LA LISTE DES ÉDITIONS, FILTRÉE PAR PÉRIMÈTRE D'ADMINISTRATION.
 *
 * Règle métier n° 8 : une administratrice détachée sur la seule COP31 ne voit
 * qu'elle — et pas une liste où les autres apparaîtraient grisées, ce qui
 * reviendrait à divulguer ce qu'elle n'administre pas.
 */
export function editionListScreen(scope: AdministeredEvents): EditionListScreen {
  const visible = editions.filter((e) => scope.is_global || scope.event_ids.includes(e.id))
  const rows = visible
    .map(listRow)
    .sort((a, b) => b.starts_at.localeCompare(a.starts_at))

  return {
    rows,
    series: seriesOptions(),
    years: [...new Set(rows.map((row) => row.edition_year))].sort((a, b) => b - a),
    is_global_scope: scope.is_global,
  }
}

// 2. OPTIONS DU FORMULAIRE
// ---------------------------------------------------------------------------

/**
 * LES FUSEAUX PROPOSÉS À LA SAISIE — la base IANA de l'exécution, pas une liste.
 *
 * `Intl.supportedValuesOf('timeZone')` rend les quelque quatre cents identifiants
 * CANONIQUES que connaît le moteur. C'est exhaustif par construction, et cela
 * règle d'un coup le défaut qui avait coûté une demi-heure : une liste écrite à la
 * main contenait `Europe/Geneva`, un ALIAS que certaines bases de fuseaux refusent,
 * et l'exception emportait toute la liste — le formulaire d'une édition restait
 * indéfiniment sur son squelette de chargement.
 *
 * `platform.timezone_name` valide de son côté n'importe quel identifiant connu de
 * PostgreSQL : la liste reste donc une COMMODITÉ DE SAISIE, pas un vocabulaire
 * fermé.
 *
 * LES VILLES SONT ACCENTUÉES QUAND ON SAIT LES ÉCRIRE. Un identifiant IANA est de
 * l'ASCII : « America/Belem » donne « Belem », et la convention du projet est
 * « heure de Belém ». Les villes où l'IFDD tient effectivement des rendez-vous
 * portent donc leur graphie ici ; les autres se contentent du segment de
 * l'identifiant, ce qui reste juste et lisible. L'accent définitif d'une édition
 * vient de toute façon de `event.events.city`.
 */
const ACCENTED_CITIES: Record<string, string> = {
  'America/Belem': 'Belém',
  'America/Sao_Paulo': 'São Paulo',
  'America/Montreal': 'Montréal',
  'America/Mexico_City': 'Mexico',
  'Europe/Zurich': 'Genève',
  'Europe/Brussels': 'Bruxelles',
  'Europe/Bucharest': 'Bucarest',
  'Europe/Lisbon': 'Lisbonne',
  'Europe/Athens': 'Athènes',
  'Europe/Vienna': 'Vienne',
  'Europe/Moscow': 'Moscou',
  'Europe/Istanbul': 'Istanbul',
  'Africa/Ndjamena': "N'Djaména",
  'Africa/Sao_Tome': 'São Tomé',
  'Africa/Cairo': 'Le Caire',
  'Asia/Ho_Chi_Minh': 'Hô Chi Minh-Ville',
  'Asia/Baku': 'Bakou',
  'Asia/Dubai': 'Dubaï',
  'Asia/Beirut': 'Beyrouth',
  'Asia/Riyadh': 'Riyad',
  'Indian/Antananarivo': 'Antananarivo',
}

/** Le segment de ville d'un identifiant IANA, rendu lisible : `Ho_Chi_Minh` → `Ho Chi Minh`. */
function cityOf(timeZone: string): string {
  const accented = ACCENTED_CITIES[timeZone]
  if (accented) return accented
  const segment = timeZone.split('/').pop() ?? timeZone
  return segment.replace(/_/g, ' ')
}

function offsetLabel(timeZone: string): string {
  // Un identifiant que l'exécution ne connaît pas LÈVE. Sans cette garde, un seul
  // fuseau emporterait la liste entière — et le formulaire d'une édition resterait
  // indéfiniment sur son squelette de chargement, sans rien dire.
  try {
    const parts = new Intl.DateTimeFormat('en-US', {
      timeZone,
      timeZoneName: 'longOffset',
    }).formatToParts(new Date('2027-11-09T12:00:00Z'))
    return parts.find((part) => part.type === 'timeZoneName')?.value ?? 'UTC'
  } catch {
    return timeZone
  }
}

/**
 * Les fuseaux que l'IFDD retient le plus souvent, remontés en tête de liste.
 *
 * Quatre cents entrées classées alphabétiquement mettraient `America/Belem` en
 * troisième page. Ces onze-là ne SONT PAS un vocabulaire : ce sont les mêmes
 * options, simplement placées devant.
 */
const FREQUENT_ZONES = [
  'America/Belem',
  'Europe/Paris',
  'America/Montreal',
  'America/Toronto',
  'Africa/Abidjan',
  'Africa/Dakar',
  'Europe/Brussels',
  'Europe/Zurich',
  'Africa/Casablanca',
  'Asia/Dubai',
  'UTC',
]

function timezoneOptions(): TimeZoneOption[] {
  // `supportedValuesOf` n'existe pas partout : le repli garde la liste courte
  // plutôt que de rendre le champ inutilisable.
  const all =
    typeof Intl.supportedValuesOf === 'function'
      ? [...Intl.supportedValuesOf('timeZone'), 'UTC']
      : [...FREQUENT_ZONES]

  const unique = [...new Set(all)]
  const frequent = FREQUENT_ZONES.filter((zone) => unique.includes(zone))
  const others = unique
    .filter((zone) => !frequent.includes(zone))
    .sort((a, b) => a.localeCompare(b, 'fr'))

  return [...frequent, ...others].map((zone) => ({
    value: zone,
    city: cityOf(zone),
    offset_label: offsetLabel(zone),
  }))
}

/** Les statuts, dans l'ordre du cycle de vie — pas dans l'ordre de l'ENUM. */
const STATUS_ORDER: EventStatus[] = [
  'draft',
  'announced',
  'ongoing',
  'completed',
  'suspended',
  'cancelled',
]

export function editionFormOptions(): EditionFormOptions {
  return {
    series: seriesOptions(),
    countries: countries
      .map((c) => ({ id: c.id, name: c.name, iso2: c.iso2 }))
      .sort((a, b) => (a.name.fr ?? '').localeCompare(b.name.fr ?? '', 'fr')),
    timezones: timezoneOptions(),
    statuses: STATUS_ORDER,
  }
}
