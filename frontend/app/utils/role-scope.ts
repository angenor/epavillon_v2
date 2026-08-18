/**
 * LA PORTÉE D'UNE ATTRIBUTION DE RÔLE (A12) — fonctions PURES.
 *
 * Ce fichier ne fait qu'une chose, et c'est la chose difficile de l'écran :
 * distinguer un rôle de son PÉRIMÈTRE. « Administrateur » et « Administrateur de
 * la COP31 » portent le même `role_code` ; tout ce qui les sépare vit dans
 * `scope_type` et `scope_id`. La v1 n'avait pas cette notion et l'a payée d'une
 * page d'administration entière, écrite en urgence pour un seul webinaire.
 *
 * NI RÉSEAU, NI TRADUCTION. Ces fonctions rendent des CODES et des VALEURS ; le
 * libellé « Administrateur de la COP31 » se compose dans le composant, qui a la
 * locale et le nom résolu de la cible. Une fonction pure qui renverrait une
 * phrase française serait intraduisible et intestable.
 *
 * LE FRONT MONTRE, L'API REFUSE. `grantableScopes()` sert à ne pas offrir un
 * choix qui sera rejeté — c'est du confort, pas une sécurité. L'API revérifie
 * `identity.role.assign` sur la portée visée, et une URL forgée ne gagne rien.
 */

import type { AssignmentState, RoleAssignmentView, ScopeRef } from '~/types/admin-users'
import type { AdministeredEvents, EffectivePermission, RoleAssignment, ScopeType } from '~/types/identity'
import type { PermissionCode, Uuid } from '~/types/shared'

/** Les quatre portées du modèle, dans l'ordre où l'écran les présente. */
export const SCOPE_TYPES: ScopeType[] = ['global', 'event', 'organization', 'negotiation_space']

/**
 * État d'une attribution, déduit de ses trois dates.
 *
 * L'ORDRE DES TESTS EST LE SENS MÉTIER. Une attribution révoquée le reste même si
 * son terme n'est pas atteint : c'est un retrait, pas une expiration, et les deux
 * ne s'expliquent pas de la même façon à qui demande pourquoi il n'a plus accès.
 * `effective_permissions()` fait la même chose en base — `revoked_at IS NULL`
 * d'abord, puis la fenêtre de validité.
 */
export function assignmentState(
  assignment: Pick<RoleAssignment, 'revoked_at' | 'valid_from' | 'valid_until'>,
  now: number = Date.now(),
): AssignmentState {
  if (assignment.revoked_at !== null) return 'revoked'
  if (Date.parse(assignment.valid_from) > now) return 'scheduled'
  if (assignment.valid_until !== null && Date.parse(assignment.valid_until) <= now) return 'expired'
  return 'active'
}

/** L'attribution compte-t-elle en ce moment ? Le seul test qui vaut autorisation. */
export function isAssignmentActive(
  assignment: Pick<RoleAssignment, 'revoked_at' | 'valid_from' | 'valid_until'>,
  now: number = Date.now(),
): boolean {
  return assignmentState(assignment, now) === 'active'
}

/**
 * Deux portées désignent-elles la même chose ?
 *
 * `global` n'a pas d'identifiant : comparer les `scope_id` seuls ferait passer
 * deux portées globales pour égales à n'importe quelle portée sans cible.
 */
export function isSameScope(a: Pick<ScopeRef, 'scope_type' | 'scope_id'>, b: Pick<ScopeRef, 'scope_type' | 'scope_id'>): boolean {
  if (a.scope_type !== b.scope_type) return false
  if (a.scope_type === 'global') return true
  return a.scope_id === b.scope_id
}

/**
 * SUR QUELLES PORTÉES CET ACTEUR PEUT-IL ATTRIBUER CE RÔLE ?
 *
 * Le croisement de trois choses, et aucune ne peut être oubliée :
 *   1. `roles.allowed_scopes` — ce que le rôle admet. `tg_check_role_scope()`
 *      refuse le reste, avec un message explicite.
 *   2. `identity.role.assign` détenue par l'acteur SUR CETTE PORTÉE. Un compte
 *      détaché sur la COP31 la détient sur `event:COP31` et nulle part ailleurs.
 *   3. Une attribution GLOBALE de l'acteur couvre toutes les portées — c'est ce
 *      que fait `has_permission` : `ra.scope_type = 'global' OR (…)`.
 *
 * On ne peut donc PAS se contenter de tester la permission « quelque part » : ce
 * serait offrir à l'administratrice de la COP31 d'attribuer un rôle global, que
 * l'API refuserait ensuite sans qu'elle comprenne pourquoi.
 */
export function grantableScopes(
  granted: EffectivePermission[] | null | undefined,
  allowedScopes: ScopeType[],
): ScopeType[] {
  if (!granted?.length) return []

  const entries = granted.filter((entry) => entry.permission_code === 'identity.role.assign')
  if (entries.length === 0) return []

  // Une attribution globale de la permission ouvre toutes les portées du rôle.
  if (entries.some((entry) => entry.scope_type === 'global')) return [...allowedScopes]

  return allowedScopes.filter((scope) => entries.some((entry) => entry.scope_type === scope))
}

/**
 * Les cibles précises sur lesquelles l'acteur peut attribuer, pour une portée
 * donnée. Vide ne veut pas dire « aucune » : pour un acteur global, TOUTES les
 * cibles conviennent, et c'est ce que dit `unrestricted`.
 */
export function grantableScopeIds(
  granted: EffectivePermission[] | null | undefined,
  scopeType: Exclude<ScopeType, 'global'>,
): { unrestricted: boolean; ids: Uuid[] } {
  const entries = (granted ?? []).filter((entry) => entry.permission_code === 'identity.role.assign')

  if (entries.some((entry) => entry.scope_type === 'global')) return { unrestricted: true, ids: [] }

  return {
    unrestricted: false,
    ids: entries
      .filter((entry) => entry.scope_type === scopeType && entry.scope_id !== null)
      .map((entry) => entry.scope_id as Uuid),
  }
}

/**
 * L'acteur peut-il attribuer CE rôle SUR CETTE portée précise ?
 *
 * Le dernier verrou avant l'envoi, et il n'est pas redondant avec l'affichage :
 * la portée choisie peut avoir changé après le choix du rôle.
 */
export function canGrant(
  granted: EffectivePermission[] | null | undefined,
  allowedScopes: ScopeType[],
  scopeType: ScopeType,
  scopeId: Uuid | null,
): boolean {
  if (!allowedScopes.includes(scopeType)) return false
  if (!grantableScopes(granted, allowedScopes).includes(scopeType)) return false
  if (scopeType === 'global') return true
  if (scopeId === null) return false

  const { unrestricted, ids } = grantableScopeIds(granted, scopeType)
  return unrestricted || ids.includes(scopeId)
}

/**
 * ATTRIBUTION EN DOUBLE — `ux_role_assignments_active`.
 *
 * L'index unique ne couvre QUE les attributions non révoquées : réattribuer un
 * rôle retiré est parfaitement légitime, et l'écran ne doit pas le refuser au
 * motif qu'une ligne révoquée traîne. Une attribution expirée, en revanche,
 * n'est pas révoquée : la base la compte encore, et la réattribuer échoue. C'est
 * le piège de cette contrainte, et la raison d'être de cette fonction.
 */
export function findConflictingAssignment(
  assignments: RoleAssignmentView[],
  roleCode: string,
  scopeType: ScopeType,
  scopeId: Uuid | null,
): RoleAssignmentView | null {
  return (
    assignments.find(
      (assignment) =>
        assignment.revoked_at === null &&
        assignment.role_code === roleCode &&
        isSameScope(assignment, { scope_type: scopeType, scope_id: scopeId }),
    ) ?? null
  )
}

/**
 * Le périmètre d'administration que produirait un jeu d'attributions.
 *
 * Rejoue `identity.administered_events()` : le critère est la PERMISSION
 * `programme.proposal.read_all`, jamais une liste de rôles. Sert au panneau
 * d'attribution à répondre, avant l'envoi, à la seule question qui intéresse
 * l'opérateur — « après ça, que verra cette personne ? ».
 */
export function administeredEventsFrom(granted: EffectivePermission[]): AdministeredEvents {
  const scoped = granted.filter((entry) => entry.permission_code === 'programme.proposal.read_all')

  return {
    is_global: scoped.some((entry) => entry.scope_type === 'global'),
    event_ids: [
      ...new Set(
        scoped
          .filter((entry) => entry.scope_type === 'event' && entry.scope_id !== null)
          .map((entry) => entry.scope_id as Uuid),
      ),
    ],
  }
}

/**
 * Ce qu'une attribution AJOUTERAIT aux permissions déjà détenues.
 *
 * Le panneau l'affiche avant d'écrire : « cette personne gagnera 4 permissions
 * sur la COP31 ». Sans cette différence, on ne distingue pas une attribution qui
 * change quelque chose d'une attribution redondante — le cas courant d'une
 * personne déjà administratrice globale à qui on ajoute un rôle sur une édition,
 * qui ne lui apporte STRICTEMENT RIEN.
 */
export function permissionsGainedBy(
  granted: EffectivePermission[] | null | undefined,
  rolePermissions: PermissionCode[],
  scopeType: ScopeType,
  scopeId: Uuid | null,
): PermissionCode[] {
  const already = new Set(
    (granted ?? [])
      .filter(
        (entry) =>
          entry.scope_type === 'global' ||
          isSameScope(entry, { scope_type: scopeType, scope_id: scopeId }),
      )
      .map((entry) => entry.permission_code),
  )

  return rolePermissions.filter((code) => !already.has(code))
}

/**
 * Ordre d'affichage d'une liste d'attributions : les vivantes d'abord, la portée
 * la plus large en tête, puis la plus récente. Une fiche qui ouvrirait sur une
 * attribution révoquée en 2025 ferait chercher longtemps le rôle en cours.
 */
const STATE_ORDER: Record<AssignmentState, number> = {
  active: 0,
  scheduled: 1,
  expired: 2,
  revoked: 3,
}

const SCOPE_ORDER: Record<ScopeType, number> = {
  global: 0,
  event: 1,
  organization: 2,
  negotiation_space: 3,
}

export function sortAssignments(assignments: RoleAssignmentView[]): RoleAssignmentView[] {
  return [...assignments].sort(
    (a, b) =>
      STATE_ORDER[a.state] - STATE_ORDER[b.state] ||
      SCOPE_ORDER[a.scope_type] - SCOPE_ORDER[b.scope_type] ||
      b.granted_at.localeCompare(a.granted_at),
  )
}
