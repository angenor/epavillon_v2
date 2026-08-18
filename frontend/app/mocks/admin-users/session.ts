/**
 * LE JOURNAL D'ÉCRITURES DE LA SESSION DE DÉMONSTRATION (A12).
 *
 * Même principe qu'`admin-organizations/session.ts`, et pour la même raison :
 * l'EFFET de l'action est le sujet de l'écran. Attribuer « Administrateur de la
 * COP31 » à quelqu'un et voir la ligne réapparaître inchangée une seconde plus
 * tard donnerait à voir un panneau qui ne fait rien — or c'est précisément ce
 * panneau que le prompt appelle « le point central de cet écran ».
 *
 * ICI, LA PERSISTANCE PORTE UNE CHOSE DE PLUS : les permissions effectives. Une
 * attribution change ce que la personne PEUT FAIRE, et l'écran voisin le montre.
 * Si le journal ne tenait que la liste des rôles, on attribuerait « Révisionniste
 * de la COP31 » et l'onglet des permissions continuerait d'afficher l'ancien
 * état — c'est-à-dire exactement le mensonge que cet écran existe pour dissiper.
 *
 * Portée : un module, donc jusqu'au prochain rechargement de la page. Rien de ce
 * qui est écrit dans `people.ts`, `permissions.ts` ou `privacy.ts` n'est modifié.
 */

import type { PersonStatus, PrivacyRequest, RoleAssignment } from '~/types/identity'
import type { IsoDateTime, Uuid } from '~/types/shared'
import { people, roleAssignments } from '../people'
import { privacyRequests } from '../privacy'

// ---------------------------------------------------------------------------
// L'état de la session
// ---------------------------------------------------------------------------

/** Attributions créées pendant la session. */
const addedAssignments: RoleAssignment[] = []
/** Révocations posées pendant la session, par identifiant d'attribution. */
const revocations = new Map<Uuid, Pick<RoleAssignment, 'revoked_at' | 'revoked_by' | 'revoked_reason'>>()
/** Changements de statut de personne. */
const statusPatches = new Map<
  Uuid,
  {
    status: PersonStatus
    status_reason: string | null
    status_changed_at: IsoDateTime
    status_changed_by: Uuid | null
    suspended_until: IsoDateTime | null
  }
>()
/** Sessions révoquées lors d'une suspension — comptées, jamais listées. */
const revokedSessionCounts = new Map<Uuid, number>()
/** Demandes RGPD traitées pendant la session. */
const privacyPatches = new Map<Uuid, Partial<PrivacyRequest>>()

/**
 * Compteur d'identifiants de la session.
 *
 * Les identifiants écrits à la main s'arrêtent à `ROLE_ASSIGNMENT(33)` ; ceux-ci
 * partent de 900 pour qu'un identifiant croisé dans une console se rattache sans
 * ambiguïté à une écriture de démonstration plutôt qu'à une donnée du jeu.
 */
let nextAssignmentSeq = 900

export function nextAssignmentId(): Uuid {
  nextAssignmentSeq += 1
  return `01930000-7006-7000-8000-${String(nextAssignmentSeq).padStart(12, '0')}`
}

// ---------------------------------------------------------------------------
// Lectures effectives
// ---------------------------------------------------------------------------

/**
 * Toutes les attributions, écritures de la session comprises.
 *
 * LES RÉVOCATIONS NE SUPPRIMENT RIEN. Une attribution retirée reste dans la
 * liste, avec sa date, son auteur et son motif : c'est l'historique que le
 * prompt demande, et c'est aussi ce que fait la base — `role_assignments` n'a
 * pas de suppression, seulement un `revoked_at`.
 */
export function effectiveAssignments(): RoleAssignment[] {
  return [...roleAssignments, ...addedAssignments].map((assignment) => {
    const revocation = revocations.get(assignment.id)
    return revocation ? { ...assignment, ...revocation } : assignment
  })
}

export function assignmentsOf(personId: Uuid): RoleAssignment[] {
  return effectiveAssignments().filter((assignment) => assignment.person_id === personId)
}

export function assignmentById(assignmentId: Uuid): RoleAssignment | null {
  return effectiveAssignments().find((assignment) => assignment.id === assignmentId) ?? null
}

/** Une personne, statut de la session appliqué. */
export function effectivePerson(personId: Uuid): (typeof people)[number] | null {
  const person = people.find((entry) => entry.id === personId)
  if (!person) return null

  const patch = statusPatches.get(personId)
  return patch ? { ...person, ...patch } : person
}

export function effectivePeople(): (typeof people)[number][] {
  return people.map((person) => {
    const patch = statusPatches.get(person.id)
    return patch ? { ...person, ...patch } : person
  })
}

export function effectivePrivacyRequests(): PrivacyRequest[] {
  return privacyRequests.map((request) => {
    const patch = privacyPatches.get(request.id)
    return patch ? { ...request, ...patch } : request
  })
}

export function revokedSessionsOf(personId: Uuid): number {
  return revokedSessionCounts.get(personId) ?? 0
}

// ---------------------------------------------------------------------------
// Écritures
// ---------------------------------------------------------------------------

export function recordGrant(assignment: RoleAssignment): void {
  addedAssignments.push(assignment)
}

export function recordRevocation(
  assignmentId: Uuid,
  actorId: Uuid | null,
  reason: string,
  at: IsoDateTime,
): void {
  revocations.set(assignmentId, {
    revoked_at: at,
    revoked_by: actorId,
    revoked_reason: reason,
  })
}

export function recordStatus(
  personId: Uuid,
  patch: {
    status: PersonStatus
    status_reason: string | null
    status_changed_at: IsoDateTime
    status_changed_by: Uuid | null
    suspended_until: IsoDateTime | null
  },
  revokedSessions: number,
): void {
  statusPatches.set(personId, patch)
  if (revokedSessions > 0) {
    revokedSessionCounts.set(personId, (revokedSessionCounts.get(personId) ?? 0) + revokedSessions)
  }
}

export function recordPrivacyPatch(requestId: Uuid, patch: Partial<PrivacyRequest>): void {
  privacyPatches.set(requestId, { ...(privacyPatches.get(requestId) ?? {}), ...patch })
}

/**
 * ANONYMISATION — `identity.anonymize_person()`, rejouée.
 *
 * La fonction en base fait cinq choses, et l'ordre importe peu, mais aucune ne
 * peut manquer : elle remplace l'identité par un jeton dérivé de l'identifiant,
 * purge téléphone, fonction, biographie et ville, retire la personne de
 * l'annuaire, supprime les comptes et révoque les sessions. Ce qu'elle NE FAIT
 * PAS est aussi important : elle ne touche ni aux inscriptions ni aux revues —
 * les agrégats d'une COP passée ne doivent pas s'effondrer parce qu'une personne
 * exerce son droit à l'effacement.
 */
export function recordAnonymization(personId: Uuid, actorId: Uuid | null, reason: string): void {
  const token = personId.replace(/-/g, '').slice(0, 12)
  const person = effectivePerson(personId)
  if (!person) return

  statusPatches.set(personId, {
    status: 'anonymized',
    status_reason: reason,
    status_changed_at: new Date().toISOString(),
    status_changed_by: actorId,
    suspended_until: null,
  })
  anonymized.set(personId, token)
}

/** Personnes anonymisées pendant la session, avec leur jeton d'identité. */
const anonymized = new Map<Uuid, string>()

export function anonymizationTokenOf(personId: Uuid): string | null {
  return anonymized.get(personId) ?? null
}
