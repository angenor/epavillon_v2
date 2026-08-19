import type { EventId } from '~/types/shared'
import type {
  EditionHistory,
  EditionHistoryCounts,
  EditionHistoryGroup,
  EditionPeriod,
} from '~/types/home'
import type { EditionStatsRow, PublicEditionRow } from '~/types/views'

/**
 * L'HISTORIQUE DES ÉDITIONS, SA PART DE LOGIQUE PURE (A15).
 *
 * Filtrer par période, grouper par millésime, compter. Trois gestes sans DOM,
 * écrits une fois pour la section de l'accueil et pour la page d'historique qui
 * viendra — c'est exactement le raisonnement qui a sorti la programmation de la
 * page d'édition.
 *
 * ── POURQUOI LE FILTRE EST CÔTÉ ÉCRAN, ET PAS UNE REQUÊTE ───────────────────
 *
 * `api.home.screen()` rend l'historique COMPLET : cinq éditions aujourd'hui,
 * quelques dizaines dans dix ans. Changer d'onglet ne doit donc rien coûter, et
 * surtout les onglets doivent annoncer leurs décomptes — « Passés (2) » se lit
 * avant d'y aller, ce qu'une liste déjà filtrée ne permettrait pas.
 *
 * Les DÉCOMPTES se calculent sur l'ensemble NON FILTRÉ. C'est la seule règle de
 * ce fichier qu'on peut se tromper à écrire, et elle se trahit en silence :
 * des onglets qui n'annoncent que ce qu'ils contiennent déjà.
 *
 * ── L'URL PORTE L'ÉTAT, EN FRANÇAIS ────────────────────────────────────────
 *
 * `?periode=a-venir`, comme `?onglet=journees` ou `?tri=lieu` ailleurs dans le
 * projet. La valeur `all` n'a PAS de paramètre : l'absence de filtre est
 * l'absence de paramètre, ce qui garde `/` propre et partageable.
 */

// ---------------------------------------------------------------------------
// L'URL
// ---------------------------------------------------------------------------

/** Ordre des onglets — « Tous » d'abord, puis le temps qui passe. */
export const EDITION_PERIODS: readonly EditionPeriod[] = ['all', 'upcoming', 'ongoing', 'past']

/** `?periode=` → période. Les codes du modèle ne sortent jamais dans l'URL. */
const PERIOD_BY_PARAM: Record<string, EditionPeriod> = {
  'a-venir': 'upcoming',
  'en-cours': 'ongoing',
  passes: 'past',
}

/** Période → `?periode=`. `all` n'écrit rien : pas de filtre, pas de paramètre. */
const PARAM_BY_PERIOD: Record<EditionPeriod, string | null> = {
  all: null,
  upcoming: 'a-venir',
  ongoing: 'en-cours',
  past: 'passes',
}

/** La période demandée par l'URL. Toute valeur inconnue retombe sur « Tous ». */
export function periodFromQuery(value: unknown): EditionPeriod {
  const raw = Array.isArray(value) ? value[0] : value
  if (typeof raw !== 'string') return 'all'
  return PERIOD_BY_PARAM[raw] ?? 'all'
}

/** La valeur à écrire dans l'URL, ou `null` pour l'en retirer. */
export function queryForPeriod(period: EditionPeriod): string | null {
  return PARAM_BY_PERIOD[period]
}

// ---------------------------------------------------------------------------
// Filtre, décomptes, groupement
// ---------------------------------------------------------------------------

/** Les décomptes des onglets, TOUJOURS calculés sur l'ensemble non filtré. */
export function countEditions(editions: PublicEditionRow[]): EditionHistoryCounts {
  const counts: EditionHistoryCounts = { all: editions.length, upcoming: 0, ongoing: 0, past: 0 }
  for (const edition of editions) counts[edition.temporal_state] += 1
  return counts
}

/**
 * Le filtre lui-même. `temporal_state` vient de la vue et ne compte que trois
 * valeurs : aucune branche morte à écrire pour un report ou une annulation, une
 * édition ne connaît ni l'un ni l'autre (`v_public_editions` les écarte).
 */
export function filterEditionsByPeriod(
  editions: PublicEditionRow[],
  period: EditionPeriod,
): PublicEditionRow[] {
  if (period === 'all') return [...editions]
  return editions.filter((edition) => edition.temporal_state === period)
}

/**
 * Groupement par millésime décroissant ; à l'intérieur, du plus récent au plus
 * ancien. Le millésime sert de repère de colonne sur écran large : c'est ce qui
 * transforme une liste de cartes en une frise qu'on parcourt des yeux.
 *
 * `edition_year` est une COLONNE de la vue, et non l'année de `starts_at` : une
 * édition peut chevaucher le 1er janvier, et c'est le millésime annoncé qui
 * fait autorité.
 */
export function groupEditionsByYear(editions: PublicEditionRow[]): EditionHistoryGroup[] {
  const byYear = new Map<number, PublicEditionRow[]>()
  for (const edition of editions) {
    const bucket = byYear.get(edition.edition_year)
    if (bucket) bucket.push(edition)
    else byYear.set(edition.edition_year, [edition])
  }

  return [...byYear.entries()]
    .sort((a, b) => b[0] - a[0])
    .map(([year, group]) => ({
      year,
      editions: [...group].sort((a, b) => b.starts_at.localeCompare(a.starts_at)),
    }))
}

/**
 * L'historique tel que la section le rend, composé depuis la réponse ENTIÈRE de
 * `api.home.screen()`. Même forme que `api.home.editions()`, et c'est voulu :
 * le jour où l'historique aura sa propre adresse paginée, la section changera de
 * source sans changer de contrat.
 */
export function buildEditionHistory(
  editions: PublicEditionRow[],
  stats: Record<EventId, EditionStatsRow>,
  period: EditionPeriod,
): EditionHistory {
  return {
    period,
    groups: groupEditionsByYear(filterEditionsByPeriod(editions, period)),
    counts: countEditions(editions),
    stats,
  }
}

/**
 * LES PROCHAINS RENDEZ-VOUS — le troisième bloc du panneau « À venir ».
 *
 * Ce qui commence ou ce qui se tient, du plus proche au plus lointain. C'est
 * l'ordre INVERSE de l'historique, et c'est normal : on ne consulte pas un
 * agenda comme on consulte des archives. `screen().editions` arrive décroissant
 * — le tri est donc ici, explicite, plutôt que supposé de l'appelant.
 */
export function nextEditions(editions: PublicEditionRow[], limit: number): PublicEditionRow[] {
  return editions
    .filter((edition) => edition.temporal_state !== 'past')
    .sort((a, b) => a.starts_at.localeCompare(b.starts_at))
    .slice(0, limit)
}

// ---------------------------------------------------------------------------
// Le volume du programme
// ---------------------------------------------------------------------------

/**
 * ABSENT VAUT ZÉRO. Une édition sans programme publié n'a AUCUNE ligne dans
 * `programme.v_edition_stats` : la clé manque, et lire `stats[id].count`
 * planterait. C'est le piège que l'indexation par `event_id` rend visible, et
 * cette fonction est le seul endroit où on le traite.
 */
export function publishedSessionCount(
  stats: Record<EventId, EditionStatsRow>,
  eventId: EventId,
): number {
  return stats[eventId]?.published_session_count ?? 0
}
