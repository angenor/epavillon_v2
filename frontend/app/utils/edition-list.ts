/**
 * Filtrage et tri de la liste des éditions (A10) — fonctions PURES.
 *
 * Séparées de la page pour la même raison qu'en A7 : elles se lisent et se
 * corrigent sans ouvrir un composant de six cents lignes, et le jour où le
 * filtrage part au serveur (prompt B3), ce fichier disparaît sans que l'écran
 * bouge.
 *
 * LE TRI EST FAIT SUR CE QUE L'ÉCRAN AFFICHE, pas sur la colonne brute : trier par
 * série trie sur le nom résolu dans la locale active, pas sur `series_id`. Une
 * comparaison d'UUID donnerait un ordre stable et incompréhensible.
 */

import type { EditionListFilters, EditionListRow, EditionSortKey } from '~/types/admin-events'
import type { SortDirection } from '~/types/ui'

/** Aucun filtre posé — la valeur que l'URL vide représente. */
export const NO_EDITION_FILTERS: EditionListFilters = {
  search: '',
  series: [],
  years: [],
  statuses: [],
  has_pavilion: null,
  published: null,
}

export function hasActiveFilters(filters: EditionListFilters): boolean {
  return (
    filters.search.trim().length > 0 ||
    filters.series.length > 0 ||
    filters.years.length > 0 ||
    filters.statuses.length > 0 ||
    filters.has_pavilion !== null ||
    filters.published !== null
  )
}

/**
 * La recherche porte sur ce qui sert à RETROUVER une édition : son titre dans les
 * deux langues, son sigle, son libellé d'édition, sa ville. Pas sur la
 * description — chercher « adaptation » ramènerait toutes les COP climat.
 */
function matchesSearch(row: EditionListRow, needle: string): boolean {
  const haystack = [
    row.title.fr,
    row.title.en,
    row.acronym,
    row.edition_label,
    row.city,
    row.slug,
    String(row.edition_year),
  ]
    .filter(Boolean)
    .join(' ')
    .toLocaleLowerCase('fr')

  return haystack.includes(needle)
}

export function filterEditions(
  rows: EditionListRow[],
  filters: EditionListFilters,
): EditionListRow[] {
  const needle = filters.search.trim().toLocaleLowerCase('fr')

  return rows.filter((row) => {
    if (needle && !matchesSearch(row, needle)) return false
    if (filters.series.length > 0 && (!row.series_id || !filters.series.includes(row.series_id))) {
      return false
    }
    if (filters.years.length > 0 && !filters.years.includes(row.edition_year)) return false
    if (filters.statuses.length > 0 && !filters.statuses.includes(row.status)) return false
    if (filters.has_pavilion !== null && row.has_pavilion !== filters.has_pavilion) return false
    if (filters.published !== null) {
      const isPublished = row.programme_published_at !== null
      if (isPublished !== filters.published) return false
    }
    return true
  })
}

/** Comparateur d'une colonne. `resolve` rend le nom de série dans la locale active. */
function compare(
  a: EditionListRow,
  b: EditionListRow,
  key: EditionSortKey,
  resolveSeries: (row: EditionListRow) => string,
): number {
  switch (key) {
    case 'title':
      return (a.title.fr ?? '').localeCompare(b.title.fr ?? '', 'fr')
    case 'series':
      return resolveSeries(a).localeCompare(resolveSeries(b), 'fr')
    case 'edition_year':
      return a.edition_year - b.edition_year
    case 'starts_at':
      return a.starts_at.localeCompare(b.starts_at)
    case 'location':
      // Le pays d'abord, la ville ensuite : c'est l'ordre dans lequel on cherche
      // une édition sur une carte.
      return (
        (a.country_name?.fr ?? '').localeCompare(b.country_name?.fr ?? '', 'fr') ||
        (a.city ?? '').localeCompare(b.city ?? '', 'fr')
      )
    case 'status':
      return a.status.localeCompare(b.status)
    case 'proposal_count':
      return a.proposal_count - b.proposal_count
    case 'programme':
      // Les non publiées valent la chaîne vide : elles se rangent d'un côté, ce
      // qui est exactement ce qu'on cherche en triant sur cette colonne.
      return (a.programme_published_at ?? '').localeCompare(b.programme_published_at ?? '')
  }
}

export function sortEditions(
  rows: EditionListRow[],
  key: EditionSortKey,
  direction: SortDirection,
  resolveSeries: (row: EditionListRow) => string,
): EditionListRow[] {
  const sign = direction === 'asc' ? 1 : -1
  return [...rows].sort(
    (a, b) => sign * compare(a, b, key, resolveSeries) || a.slug.localeCompare(b.slug),
  )
}
