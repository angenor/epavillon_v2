/**
 * Filtrage et validation des messages d'incident (A13) — fonctions PURES.
 *
 * Même motif qu'en A7, A10, A11 et A12 : hors de la page, lisibles seules, et
 * appelées à disparaître au prompt B6 quand le filtrage partira au serveur.
 *
 * IL N'Y A PAS DE TRI ICI, ET C'EST VOULU. L'ordre de cette liste n'est pas une
 * préférence d'affichage : c'est l'ordre dans lequel l'équipe agit — les actifs
 * d'abord, puis ce qui va parler, puis ce qui attend une décision, puis
 * l'historique. `live.event_incidents()` le rend déjà trié ainsi, gravité
 * décroissante à état égal. Le réordonner côté client ferait remonter un
 * brouillon devant une panne en cours.
 */

import type {
  IncidentFilters,
  IncidentPayload,
  IncidentState,
  IncidentStateCounts,
  ManagedIncident,
} from '~/types/admin-incidents'
import type { I18nText } from '~/types/shared'

/**
 * Aucun filtre posé — la valeur que représente une URL nue.
 *
 * Ici et non dans `types/` : un fichier de types ne s'auto-importe pas dans les
 * pages, et une constante que chaque écran devrait importer à la main finirait
 * recopiée. Même raison que `NO_USER_FILTERS`.
 */
export const NO_INCIDENT_FILTERS: IncidentFilters = {
  search: '',
  states: [],
  severities: [],
  scopes: [],
  kinds: [],
}

/** Les cinq états, dans l'ordre où l'équipe les parcourt. */
export const INCIDENT_STATES: readonly IncidentState[] = [
  'active',
  'scheduled',
  'draft',
  'expired',
  'unpublished',
] as const

export function hasActiveIncidentFilters(filters: IncidentFilters): boolean {
  return (
    filters.search.trim().length > 0 ||
    filters.states.length > 0 ||
    filters.severities.length > 0 ||
    filters.scopes.length > 0 ||
    filters.kinds.length > 0
  )
}

/**
 * La recherche porte sur ce qui est ÉCRIT — titre et message, dans les deux
 * langues — et sur la cible résolue. Chercher « Amazonie » doit ramener le
 * changement de salle, que le mot soit dans le message ou dans le nom de la
 * séance visée.
 */
export function filterIncidents(
  rows: ManagedIncident[],
  filters: IncidentFilters,
): ManagedIncident[] {
  const needle = filters.search.trim().toLocaleLowerCase('fr')

  return rows.filter((row) => {
    if (filters.states.length && !filters.states.includes(row.state)) return false
    if (filters.severities.length && !filters.severities.includes(row.severity)) return false
    if (filters.scopes.length && !filters.scopes.includes(row.scope)) return false
    if (filters.kinds.length && !filters.kinds.includes(row.kind_code)) return false
    if (!needle) return true

    return searchableText(row).some((text) => text.toLocaleLowerCase('fr').includes(needle))
  })
}

function searchableText(row: ManagedIncident): string[] {
  return [
    ...Object.values(row.title ?? {}),
    ...Object.values(row.message),
    row.target_label ?? '',
  ].filter((value): value is string => typeof value === 'string' && value.length > 0)
}

/** Compteurs par état, établis sur la liste ENTIÈRE — jamais après filtrage. */
export function countIncidentStates(rows: ManagedIncident[]): IncidentStateCounts {
  const counts: IncidentStateCounts = {
    active: 0,
    scheduled: 0,
    draft: 0,
    expired: 0,
    unpublished: 0,
  }
  for (const row of rows) counts[row.state] += 1
  return counts
}

// ---------------------------------------------------------------------------
// Validation du formulaire
// ---------------------------------------------------------------------------

/**
 * Ce qui manque à un message pour être enregistrable.
 *
 * TROIS RÈGLES, DEUX VENUES DE LA BASE. `ck_incidents_scope_target` exige une
 * cible cohérente avec la portée, `ck_incidents_window` exige une fin postérieure
 * au début. La troisième — LES DEUX LANGUES — est une exigence d'interface : la
 * base accepte un message en français seul, mais un bandeau que la moitié du
 * public ne comprend pas n'informe personne.
 */
export type IncidentFormIssue = 'missing_target' | 'missing_message' | 'invalid_window'

export function validateIncident(payload: IncidentPayload): IncidentFormIssue[] {
  const issues: IncidentFormIssue[] = []

  if (targetIdFor(payload) === null && payload.scope !== 'global') issues.push('missing_target')
  if (!isFilledInBothLocales(payload.message)) issues.push('missing_message')
  if (
    payload.display_until !== null &&
    Date.parse(payload.display_until) <= Date.parse(payload.display_from)
  ) {
    issues.push('invalid_window')
  }

  return issues
}

/** La cible que la portée désigne — la lecture de `ck_incidents_scope_target`. */
export function targetIdFor(payload: IncidentPayload): string | null {
  switch (payload.scope) {
    case 'global':
      return null
    case 'event':
      return payload.event_id
    case 'event_day':
      return payload.event_day_id
    case 'session':
      return payload.session_id
    case 'organization':
      return payload.organization_id
  }
}

function isFilledInBothLocales(text: I18nText | null): boolean {
  return Boolean(text?.fr?.trim()) && Boolean(text?.en?.trim())
}

/**
 * Un texte multilingue prêt pour la base, ou `null` s'il est vide.
 *
 * Une chaîne vide n'est pas une absence de titre : elle produirait un
 * `{"fr": "", "en": ""}` en base, que `platform.t()` rendrait comme un titre
 * présent et vide. Le titre est facultatif — il doit alors être NUL.
 */
export function trimmedI18n(text: I18nText): I18nText | null {
  const fr = text.fr?.trim() ?? ''
  const en = text.en?.trim() ?? ''
  if (!fr && !en) return null

  return en ? { fr, en } : { fr }
}
