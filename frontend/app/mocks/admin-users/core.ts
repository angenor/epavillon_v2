/**
 * UTILISATEURS ET RÔLES (A12) — la portée résolue, et la liste.
 *
 * ── RÉSOUDRE UNE PORTÉE EST UNE JOINTURE APPLICATIVE, PAS UNE JOINTURE SQL ──
 *
 * `role_assignments.scope_id` n'a AUCUNE clé étrangère : la cible vit dans un
 * autre module, qui peut devenir un service distant. Ce fichier fait donc ce que
 * l'API Rust fera — lire l'identifiant, aller chercher le nom dans le module
 * concerné, et ADMETTRE QU'IL PUISSE MANQUER. Une édition supprimée laisse une
 * attribution orpheline : la taire donnerait un rôle sans portée lisible, et
 * l'écran afficherait « Administrateur » à quelqu'un qui n'administre plus rien.
 *
 * ── LE PÉRIMÈTRE D'ADMINISTRATION APPLIQUÉ À DES PERSONNES ──────────────────
 *
 * Une personne n'appartient à aucune édition. « Les utilisateurs de la COP31 »
 * n'existe donc pas comme colonne : c'est un calcul, et il faut le poser
 * explicitement. Quatre liens rattachent une personne à une édition, et les
 * quatre comptent :
 *   1. une attribution de rôle portée sur cette édition ;
 *   2. un dossier déposé pour cette édition par une organisation dont elle est
 *      membre ;
 *   3. une intervention annoncée dans un dossier de cette édition ;
 *   4. une inscription à l'une de ses séances.
 * N'en retenir que le premier réduirait la liste au comité, et le responsable
 * d'un webinaire ne verrait pas les personnes inscrites au sien.
 */

import type {
  AssignableRole,
  RoleAssignmentOptions,
  RoleAssignmentView,
  ScopeChoice,
  ScopeRef,
  UserFacet,
  UserListRow,
  UserListScreen,
} from '~/types/admin-users'
import type { AdministeredEvents, EffectivePermission, RoleAssignment, ScopeType } from '~/types/identity'
import type { Uuid } from '~/types/shared'
import { countries } from '../reference'
import { organizations } from '../org'
import { events } from '../event'
import { memberships } from '../memberships'
import { allProposals } from '../proposals'
import { proposalSpeakers } from '../proposals'
import { allSessions } from '../sessions'
import { registrations } from '../registrations'
import { accounts } from '../auth'
import { permissions, rolePermissions, roles } from '../permissions'
import {
  assignmentState,
  grantableScopeIds,
  isAssignmentActive,
  sortAssignments,
} from '~/utils/role-scope'
import { assignmentsOf, effectiveAssignments, effectivePeople, effectivePrivacyRequests } from './session'

// ---------------------------------------------------------------------------
// Résolution des portées
// ---------------------------------------------------------------------------

const personName = (id: Uuid | null): string | null =>
  id === null ? null : (effectivePeople().find((person) => person.id === id)?.display_name ?? null)

/**
 * La portée, avec le nom de ce qu'elle désigne.
 *
 * `negotiation_space` n'a aucune donnée simulée — le module est hors du jalon.
 * Une attribution qui en porterait une serait donc rendue orpheline, ce qui est
 * la vérité de l'écran plutôt qu'un défaut : personne ne peut aujourd'hui dire
 * de quel espace il s'agit.
 */
export function resolveScope(scope_type: ScopeType, scope_id: Uuid | null): ScopeRef {
  if (scope_type === 'global') {
    return { scope_type, scope_id: null, scope_label: null, scope_hint: null, is_dangling: false }
  }

  if (scope_type === 'event') {
    const edition = events.find((entry) => entry.id === scope_id)
    if (!edition) {
      return { scope_type, scope_id, scope_label: null, scope_hint: null, is_dangling: true }
    }
    return {
      scope_type,
      scope_id,
      // LE SIGLE, PAS LE TITRE. « Administrateur · COP31 » se lit d'un coup
      // d'œil dans une pastille de tableau ; « Administrateur · COP31 —
      // Conférence des Nations unies sur les changements climatiques » chasse
      // les colonnes suivantes hors de l'écran. Le titre complet reste
      // accessible par l'infobulle et par la fiche de l'édition.
      scope_label: { fr: edition.acronym ?? edition.edition_label ?? edition.title.fr },
      scope_hint: [edition.city, String(edition.edition_year)].filter(Boolean).join(' · '),
      is_dangling: false,
    }
  }

  if (scope_type === 'organization') {
    const organization = organizations.find((entry) => entry.id === scope_id)
    if (!organization) {
      return { scope_type, scope_id, scope_label: null, scope_hint: null, is_dangling: true }
    }
    const country = countries.find((entry) => entry.id === organization.country_id)
    return {
      scope_type,
      scope_id,
      // Le sigle quand il existe, pour la même raison qu'au-dessus.
      scope_label: { fr: organization.acronym ?? organization.legal_name },
      scope_hint: [organization.legal_name, country?.name.fr].filter(Boolean).join(' · ') || null,
      is_dangling: false,
    }
  }

  return { scope_type, scope_id, scope_label: null, scope_hint: null, is_dangling: true }
}

// ---------------------------------------------------------------------------
// Attributions, vues par l'écran
// ---------------------------------------------------------------------------

const permissionsByRole = new Map<string, string[]>()
for (const link of rolePermissions) {
  permissionsByRole.set(link.role_code, [...(permissionsByRole.get(link.role_code) ?? []), link.permission_code])
}

export function roleView(assignment: RoleAssignment): RoleAssignmentView {
  const role = roles.find((entry) => entry.code === assignment.role_code)

  return {
    ...assignment,
    ...resolveScope(assignment.scope_type, assignment.scope_id),
    // Un rôle absent du catalogue ne peut pas exister — clé étrangère sur
    // `roles.code` — mais son libellé doit rester lisible si le semis change.
    role_label: role?.label ?? { fr: assignment.role_code },
    role_description: role?.description ?? null,
    role_is_system: role?.is_system ?? false,
    role_permissions: permissionsByRole.get(assignment.role_code) ?? [],
    granted_by_name: personName(assignment.granted_by),
    revoked_by_name: personName(assignment.revoked_by),
    state: assignmentState(assignment),
  }
}

/** Attributions EN COURS d'une personne — celles qui donnent des droits. */
export function activeAssignmentsOf(personId: Uuid): RoleAssignmentView[] {
  // La lambda est obligatoire : `.filter(isAssignmentActive)` passerait l'INDEX
  // comme second argument, donc comme instant de référence.
  return sortAssignments(
    assignmentsOf(personId)
      .filter((assignment) => isAssignmentActive(assignment))
      .map(roleView),
  )
}

/** Toutes ses attributions, révoquées et expirées comprises. */
export function allAssignmentsOf(personId: Uuid): RoleAssignmentView[] {
  return sortAssignments(assignmentsOf(personId).map(roleView))
}

// ---------------------------------------------------------------------------
// Le périmètre, appliqué à des personnes
// ---------------------------------------------------------------------------

/**
 * Les personnes rattachées aux éditions administrées, par l'un des quatre liens.
 *
 * Renvoie `null` quand le périmètre est global : il n'y a alors rien à filtrer,
 * et un `Set` de vingt-huit identifiants ne dirait pas la même chose qu'« aucune
 * restriction » le jour où une personne s'ajoute.
 */
function peopleInScope(scope: AdministeredEvents): Set<Uuid> | null {
  if (scope.is_global) return null

  const eventIds = new Set(scope.event_ids)
  const reached = new Set<Uuid>()

  // 1. Une attribution portée sur l'une de ces éditions.
  for (const assignment of effectiveAssignments()) {
    if (assignment.scope_type === 'event' && assignment.scope_id && eventIds.has(assignment.scope_id)) {
      reached.add(assignment.person_id)
    }
  }

  // 2 et 3. Les dossiers de ces éditions : leurs porteurs et leurs intervenants.
  const proposalsInScope = allProposals.filter((proposal) => eventIds.has(proposal.event_id))
  const organizationIds = new Set(proposalsInScope.map((proposal) => proposal.organization_id))
  const proposalIds = new Set(proposalsInScope.map((proposal) => proposal.id))

  for (const membership of memberships) {
    if (organizationIds.has(membership.organization_id)) reached.add(membership.person_id)
  }
  for (const speaker of proposalSpeakers) {
    if (proposalIds.has(speaker.proposal_id)) reached.add(speaker.person_id)
  }

  // 4. Les personnes inscrites à une séance de ces éditions.
  const sessionIds = new Set(
    allSessions.filter((session) => eventIds.has(session.event_id)).map((session) => session.id),
  )
  for (const registration of registrations) {
    if (registration.person_id && sessionIds.has(registration.session_id)) reached.add(registration.person_id)
  }

  return reached
}

// ---------------------------------------------------------------------------
// La liste
// ---------------------------------------------------------------------------

function accountsOf(personId: Uuid) {
  return accounts.filter((account) => account.person_id === personId)
}

export function userListRow(personId: Uuid): UserListRow | null {
  const person = effectivePeople().find((entry) => entry.id === personId)
  if (!person) return null

  const own = accountsOf(personId)
  const organization = organizations.find((entry) => entry.id === person.primary_organization_id)
  const country = countries.find((entry) => entry.id === person.country_id)
  const openRequest = effectivePrivacyRequests().find(
    (request) => request.person_id === personId && (request.status === 'received' || request.status === 'in_progress'),
  )

  return {
    person_id: person.id,
    display_name: person.display_name,
    primary_email: person.primary_email,
    email_verified_at: person.email_verified_at,
    job_title: person.job_title,
    country_name: country?.name ?? null,
    country_id: person.country_id,
    organization_id: person.primary_organization_id,
    organization_name: organization?.legal_name ?? null,
    organization_acronym: organization?.acronym ?? null,
    status: person.status,
    status_reason: person.status_reason,
    suspended_until: person.suspended_until,
    roles: activeAssignmentsOf(personId),
    // `max(last_login_at)` : une personne peut cumuler plusieurs fournisseurs, et
    // la plus récente des connexions est la seule qui réponde à « est-elle
    // encore là ? ».
    last_login_at: own.reduce<string | null>(
      (latest, account) =>
        account.last_login_at && (!latest || account.last_login_at > latest) ? account.last_login_at : latest,
      null,
    ),
    has_account: own.length > 0,
    mfa_enabled: own.some((account) => account.mfa_enabled_at !== null),
    locked_until: own.reduce<string | null>(
      (latest, account) =>
        account.locked_until && (!latest || account.locked_until > latest) ? account.locked_until : latest,
      null,
    ),
    open_privacy_request: openRequest?.request_type ?? null,
    created_at: person.created_at,
  }
}

/** Un rôle du catalogue, avec ses permissions résolues et sa charge actuelle. */
export function assignableRoles(): AssignableRole[] {
  const active = effectiveAssignments().filter((assignment) => isAssignmentActive(assignment))

  return roles.map((role) => ({
    code: role.code,
    label: role.label,
    description: role.description,
    allowed_scopes: role.allowed_scopes,
    is_system: role.is_system,
    permissions: (permissionsByRole.get(role.code) ?? []).flatMap((code) => {
      const permission = permissions.find((entry) => entry.code === code)
      return permission ? [{ code, label: permission.label, module_code: permission.module_code }] : []
    }),
    active_count: active.filter((assignment) => assignment.role_code === role.code).length,
  }))
}

function facets(rows: UserListRow[]): { countries: UserFacet[]; organizations: UserFacet[] } {
  const countryCounts = new Map<Uuid, number>()
  const organizationCounts = new Map<Uuid, number>()

  for (const row of rows) {
    if (row.country_id) countryCounts.set(row.country_id, (countryCounts.get(row.country_id) ?? 0) + 1)
    if (row.organization_id) {
      organizationCounts.set(row.organization_id, (organizationCounts.get(row.organization_id) ?? 0) + 1)
    }
  }

  return {
    countries: [...countryCounts.entries()]
      .map(([value, count]) => ({
        value,
        label: countries.find((entry) => entry.id === value)?.name ?? value,
        count,
      }))
      .sort((a, b) => String(typeof a.label === 'string' ? a.label : a.label.fr).localeCompare(
        String(typeof b.label === 'string' ? b.label : b.label.fr),
        'fr',
      )),
    organizations: [...organizationCounts.entries()]
      .map(([value, count]) => ({
        value,
        label: organizations.find((entry) => entry.id === value)?.legal_name ?? value,
        count,
      }))
      .sort((a, b) => String(a.label).localeCompare(String(b.label), 'fr')),
  }
}

/**
 * L'écran de la liste, en une réponse.
 *
 * LES DEUX DÉCOMPTES DE TÊTE NE SONT PAS DES ORNEMENTS : une file RGPD ouverte
 * et des comptes restreints sont les deux choses qui appellent un geste, et
 * elles n'apparaissent nulle part ailleurs dans le back-office. Les laisser au
 * fond d'un écran annexe revient à ne pas les afficher.
 */
export function userListScreen(scope: AdministeredEvents): UserListScreen {
  const allowed = peopleInScope(scope)

  const rows = effectivePeople()
    .filter((person) => allowed === null || allowed.has(person.id))
    .flatMap((person) => {
      const row = userListRow(person.id)
      return row ? [row] : []
    })

  const openRequests = effectivePrivacyRequests().filter(
    (request) => request.status === 'received' || request.status === 'in_progress',
  )

  return {
    rows,
    roles: assignableRoles(),
    ...facets(rows),
    scoped_to_events: allowed !== null,
    // La file RGPD n'est PAS filtrée par le périmètre : une demande d'effacement
    // porte sur la plateforme entière, pas sur une édition. Un administrateur
    // détaché n'y a pas accès du tout — voir `privacyQueue()`.
    open_privacy_requests: allowed === null ? openRequests.length : 0,
    restricted_accounts: rows.filter((row) => row.status === 'suspended' || row.status === 'blocked').length,
  }
}

// ---------------------------------------------------------------------------
// Ce que le panneau d'attribution a le droit d'offrir
// ---------------------------------------------------------------------------

function eventChoices(granted: EffectivePermission[]): ScopeChoice[] {
  const { unrestricted, ids } = grantableScopeIds(granted, 'event')

  return events
    .map((edition) => ({
      scope_type: 'event' as const,
      scope_id: edition.id,
      label: edition.title.fr,
      hint: [edition.city, String(edition.edition_year)].filter(Boolean).join(' · ') || null,
      disabled: !unrestricted && !ids.includes(edition.id),
    }))
    .sort((a, b) => Number(a.disabled) - Number(b.disabled) || a.label.localeCompare(b.label, 'fr'))
}

function organizationChoices(granted: EffectivePermission[]): ScopeChoice[] {
  const { unrestricted, ids } = grantableScopeIds(granted, 'organization')

  return organizations
    // Une fiche absorbée ne s'attribue pas : ses rattachements ont basculé vers
    // la fiche absorbante (A11), et lui donner un référent recréerait le doublon
    // qu'on vient de résoudre. `merged_into_id` est renseigné si et seulement si
    // le statut vaut `merged` — le test porte sur la colonne, que le jeu simulé
    // ne restreint pas.
    .filter((organization) => organization.merged_into_id === null)
    .map((organization) => ({
      scope_type: 'organization' as const,
      scope_id: organization.id,
      label: organization.legal_name,
      hint: organization.acronym,
      disabled: !unrestricted && !ids.includes(organization.id),
    }))
    .sort((a, b) => Number(a.disabled) - Number(b.disabled) || a.label.localeCompare(b.label, 'fr'))
}

/**
 * Ce que peut offrir le panneau, pour CET acteur.
 *
 * `negotiation_spaces` reste vide : le module Négociations est hors du jalon, et
 * aucune donnée ne décrit ses espaces. Le rôle `negotiator` admet pourtant cette
 * portée en base — le panneau l'affiche donc, désactivée et expliquée. Masquer
 * une portée que le modèle autorise ferait croire à un oubli ; en offrir une sans
 * cible ferait un formulaire qu'on ne peut pas valider.
 */
export function roleAssignmentOptions(granted: EffectivePermission[]): RoleAssignmentOptions {
  const { unrestricted, ids } = grantableScopeIds(granted, 'event')

  return {
    roles: assignableRoles(),
    events: eventChoices(granted),
    organizations: organizationChoices(granted),
    negotiation_spaces: [],
    can_assign_global: granted.some(
      (entry) => entry.permission_code === 'identity.role.assign' && entry.scope_type === 'global',
    ),
    grantable_event_ids: unrestricted ? events.map((edition) => edition.id) : ids,
  }
}
