/**
 * UTILISATEURS ET RÔLES (A12) — la fiche d'une personne, son historique et ses
 * permissions effectives.
 *
 * ── « VOICI CE QUE CETTE PERSONNE PEUT FAIRE, ET OÙ » ───────────────────────
 *
 * `identity.effective_permissions()` rend des lignes (permission, portée). C'est
 * assez pour AUTORISER, et insuffisant pour EXPLIQUER — or l'écran demandé est un
 * écran d'explication. « Pourquoi cette personne peut-elle décider d'un
 * dossier ? » n'a pas pour réponse « parce qu'elle a `programme.proposal.decide`
 * sur `event:…` », mais « parce qu'elle est administratrice de la COP31 ». La
 * composition remonte donc jusqu'au RÔLE et à l'ATTRIBUTION qui l'apportent, et
 * une même permission peut en avoir plusieurs.
 *
 * ── CE QU'ELLE NE PEUT PAS FAIRE COMPTE AUTANT ──────────────────────────────
 *
 * `missing` liste les permissions du catalogue qu'elle n'a pas. Sans cette
 * moitié-là, l'écran répond « voici quatre permissions » et laisse croire qu'il
 * n'en existe que quatre — celui qui cherche pourquoi un bouton manque n'y trouve
 * rien. La question qui amène sur cet écran est presque toujours négative.
 *
 * ── L'HISTORIQUE SE LIT DANS LA TABLE, PAS DANS L'AUDIT ─────────────────────
 *
 * `role_assignments` ne supprime jamais : une ligne porte son octroi (`granted_at`,
 * `granted_by`, `note`) et, le cas échéant, son retrait (`revoked_at`,
 * `revoked_by`, `revoked_reason` — les deux dernières ajoutées au prompt A12).
 * Une même ligne produit donc DEUX entrées d'historique. `platform.audit_log`
 * existe et le trigger `tg_role_assignments_audit` l'alimente, mais il porte des
 * différences champ par champ : illisible là où la table dit déjà l'essentiel.
 */

import type {
  AssignmentHistoryEntry,
  ConsentView,
  EffectivePermissionRow,
  EffectivePermissionsView,
  PermissionGrant,
  PermissionModuleGroup,
  PrivacyQueueScreen,
  PrivacyRequestView,
  UserDetail,
} from '~/types/admin-users'
import type { AdministeredEvents } from '~/types/identity'
import type { PermissionCode, Uuid } from '~/types/shared'
import { countries } from '../reference'
import { organizations } from '../org'
import { accounts } from '../auth'
import { permissions, rolePermissions } from '../permissions'
import { currentConsents } from '../privacy'
import { moduleByCode, moduleRank } from '../platform'
import { administeredEventsFrom, isAssignmentActive } from '~/utils/role-scope'
import { activeAssignmentsOf, allAssignmentsOf, roleView } from './core'
import { assignmentsOf, effectivePeople, effectivePerson, effectivePrivacyRequests } from './session'

const DAY_MS = 24 * 60 * 60 * 1000

const permissionsByRole = new Map<string, PermissionCode[]>()
for (const link of rolePermissions) {
  permissionsByRole.set(link.role_code, [...(permissionsByRole.get(link.role_code) ?? []), link.permission_code])
}

// ---------------------------------------------------------------------------
// Permissions effectives
// ---------------------------------------------------------------------------

export function effectivePermissionsView(personId: Uuid): EffectivePermissionsView {
  // `.filter(isAssignmentActive)` passerait l'INDEX en second argument, c'est-à-dire
  // en « maintenant » : la première attribution serait comparée à l'époque Unix et
  // toutes paraîtraient à venir. La lambda est ici indispensable.
  const active = assignmentsOf(personId)
    .filter((assignment) => isAssignmentActive(assignment))
    .map(roleView)

  /** permission → tous les octrois qui la portent. */
  const grantsByPermission = new Map<PermissionCode, PermissionGrant[]>()

  for (const assignment of active) {
    for (const code of permissionsByRole.get(assignment.role_code) ?? []) {
      grantsByPermission.set(code, [
        ...(grantsByPermission.get(code) ?? []),
        {
          scope_type: assignment.scope_type,
          scope_id: assignment.scope_id,
          scope_label: assignment.scope_label,
          scope_hint: assignment.scope_hint,
          is_dangling: assignment.is_dangling,
          role_code: assignment.role_code,
          role_label: assignment.role_label,
          assignment_id: assignment.id,
          valid_until: assignment.valid_until,
        },
      ])
    }
  }

  const rows: EffectivePermissionRow[] = [...grantsByPermission.entries()].flatMap(([code, grants]) => {
    const permission = permissions.find((entry) => entry.code === code)
    if (!permission) return []
    return [
      {
        permission_code: code,
        label: permission.label,
        module_code: permission.module_code,
        is_global: grants.some((grant) => grant.scope_type === 'global'),
        grants,
      },
    ]
  })

  const byModule = new Map<string, EffectivePermissionRow[]>()
  for (const row of rows) {
    byModule.set(row.module_code, [...(byModule.get(row.module_code) ?? []), row])
  }

  const groups: PermissionModuleGroup[] = [...byModule.entries()]
    .map(([module_code, moduleRows]) => ({
      module_code,
      module_label: moduleByCode(module_code)?.display_name ?? { fr: module_code },
      rows: moduleRows.sort((a, b) => a.permission_code.localeCompare(b.permission_code)),
    }))
    .sort((a, b) => moduleRank(a.module_code) - moduleRank(b.module_code))

  const held = new Set(rows.map((row) => row.permission_code))

  return {
    person_id: personId,
    groups,
    administered: administeredEventsFrom(
      active.flatMap((assignment) =>
        (permissionsByRole.get(assignment.role_code) ?? []).map((permission_code) => ({
          permission_code,
          scope_type: assignment.scope_type,
          scope_id: assignment.scope_id,
        })),
      ),
    ),
    total: rows.length,
    missing: permissions
      .filter((permission) => !held.has(permission.code))
      .map((permission) => ({
        permission_code: permission.code,
        label: permission.label,
        module_code: permission.module_code,
      }))
      .sort((a, b) => moduleRank(a.module_code) - moduleRank(b.module_code)),
  }
}

// ---------------------------------------------------------------------------
// Historique
// ---------------------------------------------------------------------------

/**
 * Une ligne de table, deux entrées d'historique.
 *
 * L'octroi et le retrait ne sont pas le même événement : ils ont deux dates, deux
 * auteurs et deux motifs. Les fondre en une ligne « attribué puis retiré »
 * obligerait à choisir laquelle des deux dates classer, et l'historique cesserait
 * d'être chronologique — ce qui est la seule chose qu'on lui demande.
 */
export function assignmentHistoryOf(personId: Uuid): AssignmentHistoryEntry[] {
  const entries: AssignmentHistoryEntry[] = []

  for (const assignment of allAssignmentsOf(personId)) {
    const scope = {
      scope_type: assignment.scope_type,
      scope_id: assignment.scope_id,
      scope_label: assignment.scope_label,
      scope_hint: assignment.scope_hint,
      is_dangling: assignment.is_dangling,
    }

    entries.push({
      assignment_id: assignment.id,
      kind: 'granted',
      occurred_at: assignment.granted_at,
      role_code: assignment.role_code,
      role_label: assignment.role_label,
      scope,
      actor_name: assignment.granted_by_name,
      reason: assignment.note,
      valid_until: assignment.valid_until,
    })

    if (assignment.revoked_at !== null) {
      entries.push({
        assignment_id: assignment.id,
        kind: 'revoked',
        occurred_at: assignment.revoked_at,
        role_code: assignment.role_code,
        role_label: assignment.role_label,
        scope,
        actor_name: assignment.revoked_by_name,
        reason: assignment.revoked_reason,
        valid_until: null,
      })
    }
  }

  return entries.sort((a, b) => b.occurred_at.localeCompare(a.occurred_at))
}

// ---------------------------------------------------------------------------
// Demandes RGPD
// ---------------------------------------------------------------------------

export function privacyRequestView(requestId: Uuid): PrivacyRequestView | null {
  const request = effectivePrivacyRequests().find((entry) => entry.id === requestId)
  if (!request) return null

  const person = effectivePerson(request.person_id)
  const handler = request.handled_by ? effectivePerson(request.handled_by) : null
  // Jours PLEINS restants : arrondir vers le bas ferait afficher « 0 jour » à
  // douze heures de l'échéance, ce qui se lit comme « échue aujourd'hui ».
  const daysLeft = Math.ceil((Date.parse(request.due_at) - Date.now()) / DAY_MS)
  const isClosed = request.status === 'completed' || request.status === 'rejected'

  return {
    id: request.id,
    person_id: request.person_id,
    person_name: person?.display_name ?? '—',
    person_email: person?.primary_email ?? '—',
    request_type: request.request_type,
    status: request.status,
    due_at: request.due_at,
    days_left: daysLeft,
    // Une demande close n'est jamais « en retard » : elle est traitée. Ne pas le
    // dire ferait clignoter en rouge une file entièrement honorée.
    is_overdue: !isClosed && daysLeft < 0,
    handled_by_name: handler?.display_name ?? null,
    resolution: request.resolution,
    result_asset_id: request.result_asset_id,
    created_at: request.created_at,
    completed_at: request.completed_at,
  }
}

export function privacyRequestsOf(personId: Uuid): PrivacyRequestView[] {
  return effectivePrivacyRequests()
    .filter((request) => request.person_id === personId)
    .flatMap((request) => {
      const view = privacyRequestView(request.id)
      return view ? [view] : []
    })
    .sort((a, b) => b.created_at.localeCompare(a.created_at))
}

/**
 * La file RGPD, en une réponse.
 *
 * ELLE N'EST PAS FILTRÉE PAR LE PÉRIMÈTRE, et ce n'est pas un oubli : une demande
 * d'effacement porte sur la plateforme entière, jamais sur une édition. La
 * découper par COP n'aurait aucun sens, et en montrer une part à un
 * administrateur détaché lui donnerait une file dont il ne peut honorer aucune
 * ligne. L'écran s'ouvre donc sur la permission globale, ou pas du tout.
 */
export function privacyQueue(): PrivacyQueueScreen {
  const requests = effectivePrivacyRequests()
    .flatMap((request) => {
      const view = privacyRequestView(request.id)
      return view ? [view] : []
    })
    .sort(
      (a, b) =>
        // En retard d'abord, puis par échéance : l'ordre du travail, pas celui
        // de l'arrivée.
        Number(b.is_overdue) - Number(a.is_overdue) ||
        Number(isOpen(b)) - Number(isOpen(a)) ||
        a.due_at.localeCompare(b.due_at),
    )

  return {
    requests,
    open_count: requests.filter(isOpen).length,
    overdue_count: requests.filter((request) => request.is_overdue).length,
    // Trente jours : c'est le `DEFAULT (now() + interval '30 days')` de
    // `privacy_requests.due_at`, et l'échéance réglementaire d'un mois.
    deadline_days: 30,
  }
}

function isOpen(request: PrivacyRequestView): boolean {
  return request.status === 'received' || request.status === 'in_progress'
}

// ---------------------------------------------------------------------------
// La fiche
// ---------------------------------------------------------------------------

function consentViews(personId: Uuid): ConsentView[] {
  return currentConsents(personId).map(({ purpose, is_granted, policy_version, recorded_at }) => ({
    purpose,
    is_granted,
    policy_version,
    recorded_at,
  }))
}

export function userDetail(personId: Uuid, scope: AdministeredEvents): UserDetail | null {
  const person = effectivePerson(personId)
  if (!person) return null

  const organization = organizations.find((entry) => entry.id === person.primary_organization_id)
  const country = countries.find((entry) => entry.id === person.country_id)
  const changedBy = person.status_changed_by
    ? (effectivePeople().find((entry) => entry.id === person.status_changed_by)?.display_name ?? null)
    : null

  return {
    person_id: person.id,
    display_name: person.display_name,
    first_name: person.first_name,
    last_name: person.last_name,
    civility: person.civility,
    primary_email: person.primary_email,
    email_verified_at: person.email_verified_at,
    // `identity.person_emails` n'a pas de données simulées : le jeu ne porte
    // qu'une adresse par personne. Le tableau reste, parce que le modèle le
    // porte et que l'API le remplira.
    other_emails: [],
    phone: person.phone,
    job_title: person.job_title,
    biography: person.biography,
    country_id: person.country_id,
    country_name: country?.name ?? null,
    city: person.city,
    preferred_locale: person.preferred_locale,
    timezone: person.timezone,
    organization_id: person.primary_organization_id,
    organization_name: organization?.legal_name ?? null,
    is_directory_visible: person.is_directory_visible,
    status: person.status,
    status_reason: person.status_reason,
    status_changed_at: person.status_changed_at,
    status_changed_by_name: changedBy,
    suspended_until: person.suspended_until,
    created_at: person.created_at,
    accounts: accounts
      .filter((account) => account.person_id === personId)
      .map(({ id, provider, last_login_at, password_changed_at, mfa_enabled_at, failed_attempts, locked_until, created_at }) => ({
        id,
        provider,
        last_login_at,
        password_changed_at,
        mfa_enabled_at,
        failed_attempts,
        locked_until,
        created_at,
      })),
    assignments: activeAssignmentsOf(personId),
    history: assignmentHistoryOf(personId),
    permissions: effectivePermissionsView(personId),
    consents: consentViews(personId),
    privacy_requests: privacyRequestsOf(personId),
    /**
     * DANS LE PÉRIMÈTRE, OU EN LECTURE SEULE.
     *
     * Un administrateur détaché sur la COP31 peut avoir à consulter la fiche de
     * quelqu'un — pour vérifier une adresse, comprendre un rôle — sans pouvoir y
     * toucher. Refuser l'accès entier serait excessif ; laisser les boutons
     * actifs le serait davantage. La fiche le dit donc, et l'écran s'y règle.
     */
    in_scope:
      scope.is_global ||
      activeAssignmentsOf(personId).some(
        (assignment) =>
          assignment.scope_type === 'event' &&
          assignment.scope_id !== null &&
          scope.event_ids.includes(assignment.scope_id),
      ),
  }
}
