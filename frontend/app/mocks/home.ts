/**
 * ACCUEIL PUBLIC (A15) — la composition de `/`.
 *
 * `content.ts` porte la DONNÉE et la vue `content.v_showcase` ; ce fichier porte
 * ce que la page d'accueil en fait, plus les deux vues qu'elle consomme en
 * propre. Le BACK-OFFICE de la vitrine vit dans `mocks/admin-showcase.ts`, qui
 * lit ce fichier — jamais l'inverse : la dépendance va de l'administration vers
 * le public, comme `writes.ts → detail.ts → core.ts` en A10.
 *
 * Découpé en deux fichiers parce que l'ensemble dépassait le garde-fou de mille
 * lignes de `CLAUDE.md`, et sur la même couture que les types (`types/home.ts`
 * et `types/admin-showcase.ts`) : l'unité de découpage du projet est l'ÉCRAN.
 *
 * ── DEUX VUES RECONSTITUÉES ICI, ET POURQUOI PAS DANS `mocks/views.ts` ──────
 *
 * `event.v_public_editions` et `programme.v_edition_stats` sont ajoutées au
 * modèle en même temps que cet écran, et l'accueil est aujourd'hui leur seul
 * consommateur. Elles vivent donc avec lui. Le jour où un second écran les lit —
 * la page publique d'une édition, par exemple —, elles rejoindront
 * `mocks/views.ts` sans qu'aucune signature change : elles sont exportées.
 *
 * ── LA PERSISTANCE DE SESSION ───────────────────────────────────────────────
 *
 * Même principe qu'en A11, A12 et A13 : l'EFFET de l'action est le sujet de
 * l'écran. Publier une diapositive au back-office et ne pas la voir apparaître
 * sur l'accueil ferait mentir la démonstration. Le journal est donc tenu ICI,
 * et non du côté administration : c'est l'accueil qui doit refléter ce qui vient
 * d'être écrit. Portée : un module, donc jusqu'au prochain rechargement. Rien de
 * ce qui est écrit dans `content.ts` n'est modifié.
 */

import type { Highlight, HighlightPlacement } from '~/types/content'
import type {
  EditionHistory,
  EditionHistoryCounts,
  EditionHistoryGroup,
  EditionPeriod,
  HomeScreen,
} from '~/types/home'
import type {
  EditionStatsRow,
  EditionTemporalState,
  PublicEditionRow,
  PublicScheduleRow,
  ScheduleThemeBadge,
  ShowcaseRow,
} from '~/types/views'
import type { CallForProposals } from '~/types/event/call'
import type { EventEdition } from '~/types/event/edition'
import type { EventId, Uuid } from '~/types/shared'
import { HIGHLIGHT_CREATED } from './ids'
import { highlights, showcaseRowOf } from './content'
import { attachedImage } from './covers'
import { callsForProposals } from './calls'
import { countries, entityTerms, taxonomyTerms } from './reference'
import { events, eventSeries } from './event'
import { allSessions } from './sessions'
import { publicSchedule } from './views'

// ===========================================================================
// 0. Le journal d'écritures de la session de démonstration
// ===========================================================================

/**
 * Le journal est PARTAGÉ avec `mocks/admin-showcase.ts`, qui l'écrit : deux
 * journaux, et l'accueil ne verrait jamais ce que le back-office vient de
 * publier. Il vit du côté public parce que c'est là que l'effet doit se voir.
 */

/** Diapositives rédigées pendant la session. */
export const addedHighlights: Highlight[] = []
/** Corrections apportées à une diapositive existante, par identifiant. */
export const patches = new Map<Uuid, Partial<Highlight>>()
/** Numéros d'ordre des diapositives créées à l'exécution — au-delà du jeu. */
let runtimeCounter = 0

/** L'identifiant de la prochaine diapositive écrite pendant la session. */
export function nextHighlightId(): Uuid {
  runtimeCounter += 1
  return HIGHLIGHT_CREATED(runtimeCounter)
}

/** Toutes les diapositives, écritures de la session comprises. */
export function effectiveHighlights(): Highlight[] {
  return [...highlights, ...addedHighlights].map((highlight) => {
    const patch = patches.get(highlight.id)
    return patch ? { ...highlight, ...patch } : highlight
  })
}

/** Une diapositive brute, écritures comprises. */
export function rawHighlight(id: Uuid): Highlight | null {
  return effectiveHighlights().find((highlight) => highlight.id === id) ?? null
}

/**
 * `content.v_showcase` recalculée sur l'état COURANT — écritures comprises.
 *
 * `showcase()` de `content.ts` ne connaît que le jeu figé : publier une
 * diapositive depuis le back-office et ne pas la voir apparaître sur l'accueil
 * ferait mentir la démonstration.
 */
function currentShowcase(at: number): ShowcaseRow[] {
  return effectiveHighlights()
    .filter((highlight) => {
      if (highlight.status !== 'published') return false
      if (highlight.starts_at !== null && Date.parse(highlight.starts_at) > at) return false
      if (highlight.ends_at !== null && Date.parse(highlight.ends_at) <= at) return false
      return true
    })
    .map((highlight) => showcaseRowOf(highlight))
    .sort((a, b) => a.sort_order - b.sort_order)
}

// ===========================================================================
// 1. `event.v_public_editions`, reconstituée
// ===========================================================================

/**
 * Les jeux de données sont déclarés avec `satisfies`, ce qui en infère des types
 * LITTÉRAUX : `edition.status !== 'draft'` deviendrait une comparaison
 * « sans recouvrement » parce qu'aucune édition du jeu n'est en brouillon. Ces
 * deux alias disent, à un seul endroit, qu'on se met ici à la place de la base —
 * même procédé que `editions` dans `mocks/admin-events/core.ts`.
 */
export const editions = events as EventEdition[]
const calls = callsForProposals as CallForProposals[]

const seriesById = new Map(eventSeries.map((series) => [series.id, series]))
export const countryById = new Map(countries.map((country) => [country.id, country]))
const termById = new Map(taxonomyTerms.map((term) => [term.id, term]))

/** Thématiques d'une édition — `reference.term_badges()`, même tri qu'ailleurs. */
function editionThemeBadges(eventId: EventId): ScheduleThemeBadge[] {
  return entityTerms
    .filter(
      (link) =>
        link.entity_schema === 'event' &&
        link.entity_table === 'events' &&
        link.entity_id === eventId,
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

/** Reproduit le `CASE` de la vue, dans le même ordre de tests. */
function editionTemporalState(edition: EventEdition, at: number): EditionTemporalState {
  if (at < Date.parse(edition.starts_at)) return 'upcoming'
  if (at <= Date.parse(edition.ends_at)) return 'ongoing'
  return 'past'
}

/**
 * `event.v_public_editions` — brouillons et éditions annulées exclus par le
 * `WHERE` de la vue, triées par `starts_at`.
 *
 * L'appel à propositions tient en quatre colonnes : son état ne se déduit ni de
 * `has_pavilion` ni de l'existence de la ligne — un appel peut être en
 * brouillon, clos ou annulé. `call_is_open` et l'échéance EFFECTIVE
 * (prolongation comprise) viennent des fonctions du module.
 */
export function publicEditions(at: number = Date.now()): PublicEditionRow[] {
  return editions
    .filter((edition) => edition.status !== 'draft' && edition.status !== 'cancelled')
    .map((edition) => {
      const series = edition.series_id ? seriesById.get(edition.series_id) : undefined
      const country = edition.country_id ? countryById.get(edition.country_id) : undefined
      // `LEFT JOIN … AND cfp.status <> 'cancelled'` : un appel annulé n'existe
      // plus pour la vitrine, mais l'édition reste.
      const call = calls.find(
        (candidate) => candidate.event_id === edition.id && candidate.status !== 'cancelled',
      )
      // `event.effective_deadline()` = `extended_until ?? closes_at`. La
      // PROLONGATION prime sur l'échéance initiale : afficher `closes_at` à sa
      // place ferait rater l'échéance à toute organisation qui compte sur la
      // date annoncée.
      const deadline = call ? (call.extended_until ?? call.closes_at) : null
      const themes = editionThemeBadges(edition.id)

      return {
        id: edition.id,
        slug: edition.slug,
        title: edition.title,
        description: edition.description,
        acronym: edition.acronym,
        edition_label: edition.edition_label,
        edition_year: edition.edition_year,
        status: edition.status,
        participation_mode: edition.participation_mode,
        timezone: edition.timezone,
        starts_at: edition.starts_at,
        ends_at: edition.ends_at,
        has_pavilion: edition.has_pavilion,
        programme_published_at: edition.programme_published_at,
        highlights: edition.highlights,

        series_id: edition.series_id,
        series_kind: series?.kind ?? null,
        series_name: series?.name ?? null,
        series_slug: series?.slug ?? null,

        country_id: edition.country_id,
        country_code: country?.iso2 ?? null,
        country_name: country?.name ?? null,
        city: edition.city,

        // Rôle `banner` — et non `cover` : seul `banner` est déclaré pour
        // `event.events`. Souvent nul, la carte doit rester présentable sans.
        banner: attachedImage('event', 'events', edition.id, 'banner'),

        temporal_state: editionTemporalState(edition, at),

        call_id: call?.id ?? null,
        call_status: call?.status ?? null,
        // `event.is_call_open()` : le statut ET les dates, pas l'un des deux.
        call_is_open:
          call !== undefined &&
          call.status === 'open' &&
          Date.parse(call.opens_at) <= at &&
          (deadline === null || Date.parse(deadline) > at),
        call_deadline: deadline,

        theme_codes: themes.map((theme) => theme.code),
        themes,
      }
    })
    .sort((a, b) => a.starts_at.localeCompare(b.starts_at))
}

// ===========================================================================
// 2. `programme.v_edition_stats`, reconstituée
// ===========================================================================

/**
 * Le volume du programme PUBLIÉ, par édition.
 *
 * `GROUP BY` : **une édition sans séance publiée n'a AUCUNE clé** dans l'objet
 * rendu, exactement comme elle n'a aucune ligne dans la vue. Le front traite
 * l'absence comme zéro — et c'est justement pour qu'il ne l'oublie pas que la
 * clé manque au lieu de valoir zéro.
 */
export function editionStats(): Record<EventId, EditionStatsRow> {
  const published = allSessions.filter(
    (session) => session.published_at !== null && session.status !== 'cancelled',
  )

  const byEvent = new Map<EventId, typeof published>()
  for (const session of published) {
    const bucket = byEvent.get(session.event_id) ?? []
    bucket.push(session)
    byEvent.set(session.event_id, bucket)
  }

  const stats: Record<EventId, EditionStatsRow> = {}
  for (const [eventId, sessions] of byEvent) {
    const organizations = new Set(
      sessions
        .map((session) => session.organization_id)
        .filter((id): id is Uuid => id !== null),
    )
    stats[eventId] = {
      event_id: eventId,
      published_session_count: sessions.length,
      streamed_session_count: sessions.filter((session) => session.is_streamed).length,
      organization_count: organizations.size,
      programme_starts_at: sessions
        .map((session) => session.starts_at)
        .sort((a, b) => a.localeCompare(b))[0] ?? null,
      programme_ends_at: sessions
        .map((session) => session.ends_at)
        .sort((a, b) => b.localeCompare(a))[0] ?? null,
    }
  }
  return stats
}

// ===========================================================================
// 3. LA PAGE D'ACCUEIL
// ===========================================================================

/** Les six prochaines séances, TOUTES ÉDITIONS CONFONDUES. */
function upcomingSessions(): PublicScheduleRow[] {
  return publicSchedule()
    .filter(
      (session) =>
        (session.temporal_state === 'upcoming' || session.temporal_state === 'ongoing') &&
        session.status !== 'cancelled',
    )
    .sort((a, b) => a.starts_at.localeCompare(b.starts_at))
    .slice(0, 6)
}

/**
 * L'ÉDITION EN COURS, CHOISIE PAR LES DONNÉES.
 *
 * La première édition À PAVILLON non terminée, par `starts_at` ; à défaut, la
 * plus récente. C'était déjà la logique de l'ancien `pages/index.vue`, qui
 * redirigeait vers elle : la redirection est révoquée, le choix reste.
 *
 * Sans pavillon, pas d'appel à propositions (règle métier n° 5) : une édition
 * qui n'en tient pas ne peut pas porter cette section, même si elle est la
 * prochaine dans le calendrier.
 */
export function currentEdition(at: number = Date.now()): PublicEditionRow | null {
  const withPavilion = publicEditions(at).filter((edition) => edition.has_pavilion)
  if (withPavilion.length === 0) return null

  const notFinished = withPavilion.filter((edition) => Date.parse(edition.ends_at) >= at)
  if (notFinished.length > 0) return notFinished[0] ?? null

  return withPavilion[withPavilion.length - 1] ?? null
}

/** Tout ce que `/` affiche, en une réponse. */
export function homeScreen(at: number = Date.now()): HomeScreen {
  const rows = currentShowcase(at)

  return {
    hero: rows.filter((row) => row.placement === 'home_hero'),
    aside: rows.filter((row) => row.placement === 'home_aside'),
    upcomingSessions: upcomingSessions(),
    // Décroissant : l'accueil ouvre sur ce qui vient, pas sur 2024.
    editions: publicEditions(at).sort((a, b) => b.starts_at.localeCompare(a.starts_at)),
    stats: editionStats(),
    currentEdition: currentEdition(at),
    generated_at: new Date(at).toISOString(),
  }
}

// ===========================================================================
// 4. L'HISTORIQUE DES ÉVÉNEMENTS
// ===========================================================================

/**
 * L'historique filtré par période et groupé par millésime décroissant.
 *
 * Les DÉCOMPTES sont calculés sur l'ensemble non filtré : un onglet qui
 * n'annoncerait que ce qu'il contient déjà n'apprendrait rien, et « Passés (0) »
 * doit se voir avant d'y aller.
 */
export function editionHistory(
  period: EditionPeriod = 'all',
  at: number = Date.now(),
): EditionHistory {
  const all = publicEditions(at)

  const counts: EditionHistoryCounts = {
    all: all.length,
    upcoming: all.filter((edition) => edition.temporal_state === 'upcoming').length,
    ongoing: all.filter((edition) => edition.temporal_state === 'ongoing').length,
    past: all.filter((edition) => edition.temporal_state === 'past').length,
  }

  const retained =
    period === 'all' ? all : all.filter((edition) => edition.temporal_state === period)

  const byYear = new Map<number, PublicEditionRow[]>()
  for (const edition of retained) {
    const bucket = byYear.get(edition.edition_year) ?? []
    bucket.push(edition)
    byYear.set(edition.edition_year, bucket)
  }

  const groups: EditionHistoryGroup[] = [...byYear.entries()]
    .sort((a, b) => b[0] - a[0])
    .map(([year, editions]) => ({
      year,
      editions: [...editions].sort((a, b) => b.starts_at.localeCompare(a.starts_at)),
    }))

  return { period, groups, counts, stats: editionStats() }
}
