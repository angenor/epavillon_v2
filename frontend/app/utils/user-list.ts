/**
 * Filtrage et tri de la liste des utilisateurs (A12) — fonctions PURES.
 *
 * Même motif qu'en A7, A10 et A11 : hors de la page, lisibles seules, et
 * appelées à disparaître au prompt B1 quand le filtrage partira au serveur.
 *
 * LE TRI PAR DÉFAUT EST LA DERNIÈRE CONNEXION, DÉCROISSANTE. Ni le nom, ni le
 * rôle : cet écran s'ouvre pour agir sur des comptes vivants — retirer un rôle
 * arrivé à terme, suspendre un compte signalé —, et l'ordre alphabétique fait
 * remonter en tête vingt-huit personnes dont rien ne dit laquelle appelle un
 * geste. Les personnes SANS COMPTE, elles, n'ont aucune dernière connexion :
 * elles ne sont pas « les plus anciennes », elles sont hors de cette échelle, et
 * le tri les range en fin de liste dans les deux sens — comme le ratio absent de
 * la liste des organisations.
 */

import type { UserListFilters, UserListRow, UserSortKey } from '~/types/admin-users'
import type { SortDirection } from '~/types/ui'

/**
 * Aucun filtre posé — la valeur que représente une URL nue.
 *
 * Ici et non dans `types/` : un fichier de types ne s'auto-importe pas dans les
 * pages, et une constante que chaque écran devrait importer à la main finirait
 * recopiée. Même raison que `NO_ORGANIZATION_FILTERS`.
 */
export const NO_USER_FILTERS: UserListFilters = {
  search: '',
  roles: [],
  scope_type: null,
  scope_id: null,
  statuses: [],
  countries: [],
  organizations: [],
  without_role: false,
  without_account: false,
}

export function hasActiveUserFilters(filters: UserListFilters): boolean {
  return (
    filters.search.trim().length > 0 ||
    filters.roles.length > 0 ||
    filters.scope_type !== null ||
    filters.statuses.length > 0 ||
    filters.countries.length > 0 ||
    filters.organizations.length > 0 ||
    filters.without_role ||
    filters.without_account
  )
}

/**
 * La recherche porte sur le nom, l'adresse et la fonction.
 *
 * L'ADRESSE EN FAIT PARTIE, et ce n'est pas accessoire : c'est par elle qu'on
 * retrouve quelqu'un dont on ne sait qu'écrire le nom de travers, et c'est la
 * clé de rapprochement du modèle — `people.primary_email` est ce qui relie un
 * invité à son futur compte.
 */
function matchesSearch(row: UserListRow, needle: string): boolean {
  return [row.display_name, row.primary_email, row.job_title, row.organization_name, row.organization_acronym]
    .filter(Boolean)
    .join(' ')
    .toLocaleLowerCase('fr')
    .includes(needle)
}

/**
 * FILTRER PAR RÔLE ET FILTRER PAR PORTÉE SONT DEUX QUESTIONS DIFFÉRENTES.
 *
 *   « Qui est révisionniste ? »            → `roles`, sans considération de portée
 *   « Qui a un rôle sur la COP31 ? »       → `scope_type` + `scope_id`
 *   « Qui est révisionniste de la COP31 ? »→ les deux ensemble
 *
 * Les confondre — un seul filtre « rôle » qui listerait « Révisionniste COP31 »
 * et « Révisionniste COP30 » comme deux entrées distinctes — rendrait la
 * première question impossible à poser, alors que c'est la plus fréquente.
 */
export function filterUsers(rows: UserListRow[], filters: UserListFilters): UserListRow[] {
  const needle = filters.search.trim().toLocaleLowerCase('fr')

  return rows.filter((row) => {
    if (needle && !matchesSearch(row, needle)) return false
    if (filters.statuses.length > 0 && !filters.statuses.includes(row.status)) return false
    if (filters.countries.length > 0 && (!row.country_id || !filters.countries.includes(row.country_id))) {
      return false
    }
    if (
      filters.organizations.length > 0 &&
      (!row.organization_id || !filters.organizations.includes(row.organization_id))
    ) {
      return false
    }
    if (filters.without_role && row.roles.length > 0) return false
    if (filters.without_account && row.has_account) return false
    if (filters.roles.length > 0 && !row.roles.some((role) => filters.roles.includes(role.role_code))) {
      return false
    }
    if (filters.scope_type !== null) {
      const matches = row.roles.some(
        (role) =>
          role.scope_type === filters.scope_type &&
          (filters.scope_id === null || role.scope_id === filters.scope_id),
      )
      if (!matches) return false
    }
    return true
  })
}

/**
 * Poids d'un statut pour le tri : ce qui demande un geste remonte.
 *
 * `blocked` avant `suspended` avant `active` : une exclusion durable se relit
 * plus souvent qu'une suspension de quinze jours, et une personne anonymisée ne
 * se relit jamais — elle ferme la liste.
 */
const STATUS_WEIGHT: Record<UserListRow['status'], number> = {
  blocked: 0,
  suspended: 1,
  active: 2,
  anonymized: 3,
}

function compare(a: UserListRow, b: UserListRow, key: UserSortKey): number {
  switch (key) {
    case 'display_name':
      return a.display_name.localeCompare(b.display_name, 'fr')
    case 'primary_email':
      return a.primary_email.localeCompare(b.primary_email, 'fr')
    case 'organization':
      return (a.organization_name ?? '').localeCompare(b.organization_name ?? '', 'fr')
    case 'country':
      return (a.country_name?.fr ?? '').localeCompare(b.country_name?.fr ?? '', 'fr')
    case 'roles':
      // Le NOMBRE d'attributions, pas leur nom : trier des rôles par ordre
      // alphabétique n'apprend rien, tandis que « qui en cumule le plus ? » dit
      // où regarder d'abord.
      return a.roles.length - b.roles.length
    case 'status':
      return STATUS_WEIGHT[a.status] - STATUS_WEIGHT[b.status]
    case 'last_login_at':
      return (a.last_login_at ?? '').localeCompare(b.last_login_at ?? '')
  }
}

/**
 * Tri de la liste.
 *
 * L'ABSENCE DE CONNEXION ÉCHAPPE AU SENS DU TRI. Deux cas s'y cachent, et aucun
 * n'appartient à l'échelle du temps : la personne sans compte, qui ne peut pas
 * se connecter, et le compte créé qui ne s'est jamais servi. Les faire remonter
 * en tête d'un tri décroissant les ferait passer pour les plus actives.
 */
export function sortUsers(
  rows: UserListRow[],
  key: UserSortKey,
  direction: SortDirection,
): UserListRow[] {
  const sign = direction === 'asc' ? 1 : -1

  return [...rows].sort((a, b) => {
    if (key === 'last_login_at') {
      const aMissing = a.last_login_at === null
      const bMissing = b.last_login_at === null
      if (aMissing !== bMissing) return aMissing ? 1 : -1
    }
    return sign * compare(a, b, key) || a.display_name.localeCompare(b.display_name, 'fr')
  })
}
