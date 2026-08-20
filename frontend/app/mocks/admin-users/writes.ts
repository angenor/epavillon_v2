/**
 * UTILISATEURS ET RÔLES (A12) — les quatre écritures, et ce qu'elles REFUSENT.
 *
 * Les données simulées restent en lecture seule : ces fonctions calculent la
 * réponse que l'API rendra et empilent l'effet dans le journal de session. Ce
 * qu'elles refusent n'est pas une prudence d'écran — ce sont les invariants de
 * `030_identity.sql`, traduits en français :
 *
 *   `ux_role_assignments_active`      la même personne, le même rôle, la même
 *                                     portée, deux fois. L'index ne couvre QUE
 *                                     les attributions non révoquées : une
 *                                     attribution EXPIRÉE compte encore, et la
 *                                     réattribuer échoue — c'est le piège.
 *   `tg_role_assignments_check_scope` « Référent d'organisation » attribué
 *                                     globalement, « Super administrateur » sur
 *                                     une seule COP : refusés, avec le message
 *                                     que la base elle-même écrit.
 *   `ck_role_assignment_window`       une fin avant le début.
 *   `ck_people_suspension_window`     une suspension sans terme.
 *
 * ET UN REFUS QUI N'EST PAS UNE CONTRAINTE : `forbidden_scope`. L'acteur n'a pas
 * `identity.role.assign` sur la portée visée. Le panneau ne l'offre pas, mais
 * masquer un bouton n'a jamais empêché une requête, et c'est l'un des deux cas
 * où la règle métier n° 8 se joue vraiment.
 */

import type {
  GrantRolePayload,
  HandlePrivacyRequestPayload,
  PersonWriteResult,
  PrivacyWriteResult,
  RevokeRolePayload,
  RoleWriteResult,
  SetPersonStatusPayload,
} from '~/types/admin-users'
import type { AdministeredEvents, EffectivePermission, RoleAssignment } from '~/types/identity'
import type { Uuid } from '~/types/shared'
import { canGrant, findConflictingAssignment } from '~/utils/role-scope'
import { roles } from '../permissions'
import { activeAssignmentsOf, roleView } from './core'
import { privacyRequestView, privacyQueue, userDetail } from './detail'
import {
  assignmentById,
  assignmentsOf,
  effectivePerson,
  nextAssignmentId,
  recordAnonymization,
  recordGrant,
  recordPrivacyPatch,
  recordRevocation,
  recordStatus,
} from './session'

const now = (): string => new Date().toISOString()

// ---------------------------------------------------------------------------
// Attribuer un rôle
// ---------------------------------------------------------------------------

export function grantRole(
  payload: GrantRolePayload,
  actorId: Uuid | null,
  granted: EffectivePermission[],
): RoleWriteResult {
  const empty = { assignment: null, assignments: activeAssignmentsOf(payload.person_id), conflict_with: null, message: null }

  if (!effectivePerson(payload.person_id)) return { status: 'not_found', ...empty }

  const role = roles.find((entry) => entry.code === payload.role_code)
  if (!role) return { status: 'not_found', ...empty }

  // `tg_role_assignments_check_scope` : le rôle admet-il cette portée ? Le
  // message reprend celui du trigger, mot pour mot — c'est lui que l'API rend.
  if (!role.allowed_scopes.includes(payload.scope_type)) {
    return {
      status: 'scope_not_allowed',
      ...empty,
      message: `Le rôle « ${role.code} » ne peut pas être attribué sur la portée « ${payload.scope_type} » (portées autorisées : ${role.allowed_scopes.join(', ')}).`,
    }
  }

  // `identity.role.assign` sur la portée VISÉE, pas « quelque part ».
  if (!canGrant(granted, role.allowed_scopes, payload.scope_type, payload.scope_id)) {
    return { status: 'forbidden_scope', ...empty }
  }

  // `ux_role_assignments_active`. L'INDEX COUVRE TOUTES LES ATTRIBUTIONS NON
  // RÉVOQUÉES, pas seulement les actives : une attribution arrivée à son terme
  // reste en base et interdit toujours le doublon. Filtrer sur les attributions
  // ACTIVES ferait passer l'écran, puis échouer l'API — le piège de cet index.
  const conflict = findConflictingAssignment(
    nonRevokedAssignmentsOf(payload.person_id),
    payload.role_code,
    payload.scope_type,
    payload.scope_id,
  )
  if (conflict) return { status: 'duplicate', ...empty, conflict_with: conflict }

  const validFrom = payload.valid_from ?? now()
  // `ck_role_assignment_window` : une fin doit suivre le début.
  if (payload.valid_until !== null && Date.parse(payload.valid_until) <= Date.parse(validFrom)) {
    return { status: 'scope_not_allowed', ...empty }
  }

  const assignment: RoleAssignment = {
    id: nextAssignmentId(),
    person_id: payload.person_id,
    role_code: payload.role_code,
    scope_type: payload.scope_type,
    scope_id: payload.scope_type === 'global' ? null : payload.scope_id,
    granted_by: actorId,
    granted_at: now(),
    valid_from: validFrom,
    valid_until: payload.valid_until,
    revoked_at: null,
    revoked_by: null,
    revoked_reason: null,
    note: payload.note?.trim() ? payload.note.trim() : null,
  }

  recordGrant(assignment)

  return {
    status: 'granted',
    assignment: roleView(assignment),
    assignments: activeAssignmentsOf(payload.person_id),
    conflict_with: null,
    message: null,
  }
}

/** Ce que couvre `ux_role_assignments_active` : tout ce qui n'est pas révoqué. */
function nonRevokedAssignmentsOf(personId: Uuid) {
  return assignmentsOf(personId)
    .filter((assignment) => assignment.revoked_at === null)
    .map(roleView)
}

// ---------------------------------------------------------------------------
// Révoquer un rôle
// ---------------------------------------------------------------------------

export function revokeRole(
  payload: RevokeRolePayload,
  actorId: Uuid | null,
  granted: EffectivePermission[],
): RoleWriteResult {
  const assignment = assignmentById(payload.assignment_id)
  if (!assignment) {
    return { status: 'not_found', assignment: null, assignments: [], conflict_with: null, message: null }
  }

  const role = roles.find((entry) => entry.code === assignment.role_code)
  const empty = {
    assignment: null,
    assignments: activeAssignmentsOf(assignment.person_id),
    conflict_with: null,
    message: null,
  }

  // RETIRER DEMANDE LE MÊME DROIT QU'ATTRIBUER, SUR LA MÊME PORTÉE. Sans cette
  // symétrie, un administrateur détaché sur la COP31 pourrait retirer un rôle
  // global qu'il n'aurait jamais pu accorder.
  if (!canGrant(granted, role?.allowed_scopes ?? [], assignment.scope_type, assignment.scope_id)) {
    return { status: 'forbidden_scope', ...empty }
  }

  recordRevocation(payload.assignment_id, actorId, payload.reason.trim(), now())

  const revoked = assignmentById(payload.assignment_id)

  return {
    status: 'revoked',
    assignment: revoked ? roleView(revoked) : null,
    assignments: activeAssignmentsOf(assignment.person_id),
    conflict_with: null,
    message: null,
  }
}

// ---------------------------------------------------------------------------
// Suspendre, bloquer, rétablir
// ---------------------------------------------------------------------------

/**
 * Changement de statut.
 *
 * `ck_people_suspension_window` EXIGE UN TERME À TOUTE SUSPENSION, et c'est une
 * décision du modèle, pas une formalité : une suspension sans date de fin est un
 * blocage qui n'ose pas dire son nom, et c'est ainsi qu'un compte reste fermé
 * trois ans parce que personne ne se souvient pourquoi. Le blocage, lui, est
 * durable et assumé — il n'a pas de terme.
 */
export function setPersonStatus(payload: SetPersonStatusPayload, actorId: Uuid | null, scope: AdministeredEvents): PersonWriteResult {
  const person = effectivePerson(payload.person_id)
  if (!person) return { status: 'not_found', detail: null }

  if (payload.status === 'suspended' && !payload.suspended_until) {
    return { status: 'missing_deadline', detail: userDetail(payload.person_id, scope) }
  }

  recordStatus(
    payload.person_id,
    {
      status: payload.status,
      status_reason: payload.status === 'active' ? null : payload.reason.trim(),
      status_changed_at: now(),
      status_changed_by: actorId,
      suspended_until: payload.status === 'suspended' ? payload.suspended_until : null,
    },
    // Le nombre de sessions révoquées n'est pas simulé ligne à ligne :
    // `identity.sessions` n'a pas de données. Une suspension en révoque au moins
    // une quand la personne a un compte — c'est ce que l'écran annonce.
    payload.revoke_sessions ? 1 : 0,
  )

  return { status: 'saved', detail: userDetail(payload.person_id, scope) }
}

// ---------------------------------------------------------------------------
// Traiter une demande RGPD
// ---------------------------------------------------------------------------

export function handlePrivacyRequest(
  payload: HandlePrivacyRequestPayload,
  actorId: Uuid | null,
): PrivacyWriteResult {
  const request = privacyRequestView(payload.request_id)
  if (!request) return { status: 'not_found', request: null, requests: privacyQueue().requests }

  if (payload.action === 'anonymize') {
    // L'ANONYMISATION NE RÉPOND QU'À UNE DEMANDE D'EFFACEMENT. L'exécuter sur une
    // demande d'export détruirait l'identité de quelqu'un qui demandait
    // simplement une copie de ses données.
    if (request.request_type !== 'erasure') {
      return { status: 'wrong_type', request, requests: privacyQueue().requests }
    }

    recordAnonymization(request.person_id, actorId, payload.resolution.trim())
    recordPrivacyPatch(payload.request_id, {
      status: 'completed',
      handled_by: actorId,
      resolution: payload.resolution.trim(),
      completed_at: now(),
    })

    return {
      status: 'anonymized',
      request: privacyRequestView(payload.request_id),
      requests: privacyQueue().requests,
    }
  }

  const patch =
    payload.action === 'start'
      ? { status: 'in_progress' as const, handled_by: actorId }
      : {
          status: payload.action === 'complete' ? ('completed' as const) : ('rejected' as const),
          handled_by: actorId,
          resolution: payload.resolution.trim(),
          completed_at: now(),
        }

  recordPrivacyPatch(payload.request_id, patch)

  return {
    status: 'saved',
    request: privacyRequestView(payload.request_id),
    requests: privacyQueue().requests,
  }
}
