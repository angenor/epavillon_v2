/**
 * Filtrage et tri de la liste des organisations (A11) — fonctions PURES.
 *
 * Séparées de la page pour la même raison qu'en A7 et A10 : elles se lisent et
 * se corrigent sans ouvrir un composant de six cents lignes, et le jour où le
 * filtrage part au serveur (prompt B3), ce fichier disparaît sans que l'écran
 * bouge.
 *
 * LE TRI PAR DÉFAUT EST LE SCORE DE CONFIANCE CROISSANT — c'est la demande du
 * prompt, et ce n'est pas un détail d'ergonomie : une liste alphabétique de
 * plusieurs centaines de fiches ne dit pas par où commencer, tandis que la
 * première ligne d'un tri par score est toujours la fiche qui mérite un regard.
 * Le § 7 du SQL le dit dans les mêmes termes : « traiter en priorité les fiches
 * faibles, plutôt que de parcourir une liste alphabétique ».
 *
 * LE RATIO NUL N'EST PAS UN RATIO DE ZÉRO. `ratio_acceptation` vaut `null` pour
 * une organisation qui n'a jamais rien déposé — le `COMMENT ON` de la colonne est
 * explicite. Le tri les range donc TOUJOURS en fin de liste, quel que soit le
 * sens : une fiche sans dépôt n'est ni la meilleure ni la pire, elle est hors
 * sujet, et la faire remonter en tête d'un tri croissant ferait passer douze
 * fiches inertes devant celle qui échoue vraiment.
 */

import type {
  OrganizationListFilters,
  OrganizationListRow,
  OrganizationSortKey,
} from '~/types/admin-organizations'
import type { SortDirection } from '~/types/ui'

/**
 * Aucun filtre posé — la valeur que l'URL vide représente.
 *
 * Elle vit ici et non dans `types/`, comme `NO_EDITION_FILTERS` : un fichier de
 * types ne s'auto-importe pas dans les pages, et une constante que chaque écran
 * devrait importer à la main finirait recopiée.
 */
export const NO_ORGANIZATION_FILTERS: OrganizationListFilters = {
  search: '',
  countries: [],
  types: [],
  statuses: [],
  verified: null,
  has_duplicate: null,
  max_trust_score: null,
}

export function hasActiveOrganizationFilters(filters: OrganizationListFilters): boolean {
  return (
    filters.search.trim().length > 0 ||
    filters.countries.length > 0 ||
    filters.types.length > 0 ||
    filters.statuses.length > 0 ||
    filters.verified !== null ||
    filters.has_duplicate !== null ||
    filters.max_trust_score !== null
  )
}

/**
 * La recherche porte sur TOUTES LES MANIÈRES DE DÉSIGNER l'organisation — nom
 * légal, sigle, slug —, ce qui est la règle métier n° 1 du projet. La liste ne
 * cherche pas dans `organization_names` : le tableau n'en dispose pas, et la
 * recherche complète est celle de `find_similar_organizations()`, appelée par
 * l'écran de fusion pour choisir une seconde fiche. Deux recherches différentes
 * pour deux besoins différents — mais aucune des deux ne se contente du nom
 * légal, ce qui était le défaut de la v1.
 */
function matchesSearch(row: OrganizationListRow, needle: string): boolean {
  return [row.legal_name, row.acronym, row.slug]
    .filter(Boolean)
    .join(' ')
    .toLocaleLowerCase('fr')
    .includes(needle)
}

export function filterOrganizations(
  rows: OrganizationListRow[],
  filters: OrganizationListFilters,
): OrganizationListRow[] {
  const needle = filters.search.trim().toLocaleLowerCase('fr')

  return rows.filter((row) => {
    if (needle && !matchesSearch(row, needle)) return false
    if (filters.countries.length > 0 && (!row.country_id || !filters.countries.includes(row.country_id))) {
      return false
    }
    if (filters.types.length > 0 && !filters.types.includes(row.organization_type_code)) return false
    if (filters.statuses.length > 0 && !filters.statuses.includes(row.statut)) return false
    if (filters.verified !== null && row.est_verifiee !== filters.verified) return false
    if (filters.has_duplicate !== null && row.pending_duplicate_count > 0 !== filters.has_duplicate) {
      return false
    }
    if (filters.max_trust_score !== null && row.score_confiance > filters.max_trust_score) return false
    return true
  })
}

/**
 * Comparateur d'une colonne. `resolveType` rend le libellé du type dans la
 * locale active : trier sur `organization_type_code` classerait par code
 * technique, c'est-à-dire dans un ordre qui n'a de sens pour personne.
 */
function compare(
  a: OrganizationListRow,
  b: OrganizationListRow,
  key: OrganizationSortKey,
  resolveType: (row: OrganizationListRow) => string,
): number {
  switch (key) {
    case 'legal_name':
      return a.legal_name.localeCompare(b.legal_name, 'fr')
    case 'country':
      return (
        (a.pays_nom?.fr ?? '').localeCompare(b.pays_nom?.fr ?? '', 'fr') ||
        a.legal_name.localeCompare(b.legal_name, 'fr')
      )
    case 'type':
      return resolveType(a).localeCompare(resolveType(b), 'fr')
    case 'membres_actifs':
      return a.membres_actifs - b.membres_actifs
    case 'propositions_deposees':
      return a.propositions_deposees - b.propositions_deposees
    case 'propositions_acceptees':
      return a.propositions_acceptees - b.propositions_acceptees
    case 'ratio_acceptation':
      // Les deux sans ratio : départagés plus bas par le nom. L'un des deux
      // seulement : il descend, dans les deux sens de tri.
      if (a.ratio_acceptation === null && b.ratio_acceptation === null) return 0
      if (a.ratio_acceptation === null) return 1
      if (b.ratio_acceptation === null) return -1
      return a.ratio_acceptation - b.ratio_acceptation
    case 'score_confiance':
      return a.score_confiance - b.score_confiance
    case 'derniere_activite':
      return (a.derniere_activite ?? '').localeCompare(b.derniere_activite ?? '')
  }
}

/**
 * Tri de la liste.
 *
 * L'ABSENCE DE RATIO ÉCHAPPE AU SENS DU TRI : le signe ne s'applique pas aux
 * lignes sans dépôt, qui restent en fin de liste. C'est pourquoi ce cas est
 * traité ici et non dans `compare`, dont le résultat est multiplié par le signe.
 */
export function sortOrganizations(
  rows: OrganizationListRow[],
  key: OrganizationSortKey,
  direction: SortDirection,
  resolveType: (row: OrganizationListRow) => string,
): OrganizationListRow[] {
  const sign = direction === 'asc' ? 1 : -1

  return [...rows].sort((a, b) => {
    if (key === 'ratio_acceptation') {
      const aMissing = a.ratio_acceptation === null
      const bMissing = b.ratio_acceptation === null
      if (aMissing !== bMissing) return aMissing ? 1 : -1
    }
    return sign * compare(a, b, key, resolveType) || a.legal_name.localeCompare(b.legal_name, 'fr')
  })
}

/**
 * Seuil sous lequel une fiche mérite un regard.
 *
 * 50 SUR 100 N'EST PAS UNE MOYENNE ARBITRAIRE : `compute_trust_score()` accorde
 * 40 points au sceau de l'IFDD et 25 à un domaine vérifié. Une fiche qui n'a ni
 * l'un ni l'autre plafonne à 40, même complète — site, pays, description et
 * quatre membres. Le seuil sépare donc exactement les fiches que personne n'a
 * encore regardées de celles qui l'ont été.
 */
export const LOW_TRUST_SCORE = 50
