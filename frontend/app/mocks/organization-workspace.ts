/**
 * ESPACE ORGANISATION (A5) — les compositions que l'écran lit, et les trois
 * écritures qu'il permet.
 *
 * POURQUOI CE FICHIER PLUTÔT QUE DES LECTURES DANS LA PAGE. L'écran répond à
 * deux questions — « où en est chacun de mes dossiers ? », « qu'est-ce qui
 * attend une action de ma part ? » — et aucune des deux ne se lit dans une
 * table. La première suppose de croiser le dossier, son édition, son journal de
 * transitions, ses demandes de correction ouvertes, ses séances, leurs inscrits
 * et leurs rappels ; la seconde de balayer tout cela plus la file d'adhésions.
 * Composé dans la page, ce travail se recopierait au premier écran voisin.
 *
 * IL N'EXISTE PAS DE VUE « MES PROPOSITIONS » DANS LE MODÈLE — écart n° 8,
 * ouvert depuis le 16/08. `v_proposal_dashboard` répond au COMITÉ : elle porte
 * les notes, le rang dans l'événement et les revues manquantes, dont rien ne
 * doit parvenir au soumissionnaire. C'est donc une composition, et elle
 * appartiendra à l'API (prompt B4).
 *
 * CE QUE L'ORGANISATION VOIT DE SES DOSSIERS, ET CE QU'ELLE NE VOIT PAS :
 *  · les COMMENTAIRES `submitter` — le fil partagé avec elle. Jamais les notes
 *    du comité (`committee`) ni les notes personnelles (`private`) ;
 *  · le NOMBRE d'inscrits à ses séances, jamais leur identité : ce sont les
 *    données personnelles de tiers, pas les siennes ;
 *  · aucune note, aucun rang, aucun nom de membre du comité.
 *
 * LES DOSSIERS LISTÉS SONT CEUX QU'ELLE PORTE (`proposals.organization_id`),
 * pas ceux où elle est co-organisatrice. C'est le porteur principal qui répond
 * du dossier, reçoit les demandes de correction et est notifié de la décision —
 * le modèle le dit en toutes lettres (`070` § 3 bis). Une co-organisation
 * apparaît autrement : comme une chose à confirmer, dans le bloc des actions.
 */

import type { EventEdition } from '~/types/event/edition'
import type { Membership } from '~/types/org'
import type { Person } from '~/types/identity'
import type { ProposalComment } from '~/types/programme/proposal'
import type {
  DecideMembershipPayload,
  InviteMemberPayload,
  InviteMemberResult,
  MemberEntry,
  ProposalFile,
  ProposalTracking,
  ReplyToCommentPayload,
  ResolveCommentPayload,
  TrackedSession,
  WorkspaceAction,
  WorkspaceOverview,
} from '~/types/organization-workspace'
import { MEMBERSHIP, PERSON_INVITED, PROPOSAL_COMMENT } from './ids'
import { events } from './event'
import { rooms } from './rooms'
import { people } from './people'
import { openCallForProposals } from './proposal-submission'
import {
  addSessionMembership,
  allMemberships,
  nextSessionMembershipIndex,
  organizationById,
} from './organization-search'
import { allProposals, proposalComments, proposalOrganizations, proposalTransitions } from './proposals'
import { proposalHistory } from './proposals/history'
import { editedProposal } from './proposal-edit'
import { allSessions } from './sessions'
import { registrations } from './registrations'
import { sessionReminderSchedule } from './reminders'

// ---------------------------------------------------------------------------
// Le journal d'écriture de la session
//
// Même principe qu'en A2 et A4 : les données écrites à la main ne bougent pas,
// on empile par-dessus. Ce qui a été fait pendant la démonstration le reste
// jusqu'au prochain rechargement de page — sans quoi une réponse envoyée
// disparaîtrait du fil sous les yeux de qui vient de l'écrire.
// ---------------------------------------------------------------------------

/** Messages écrits pendant la session. */
const sessionComments: ProposalComment[] = []
/** Adhésions tranchées pendant la session, par identifiant. */
const sessionMembershipDecisions = new Map<string, Membership>()
/** Résolutions posées ou retirées pendant la session, par identifiant de message. */
const sessionResolutions = new Map<string, { resolved_at: string | null; resolved_by: string | null }>()

/** Les messages du jeu, tels qu'ils sont APRÈS les écritures de la session. */
function commentsWithSession(): ProposalComment[] {
  const merged = [...proposalComments, ...sessionComments].map((comment) => {
    const resolution = sessionResolutions.get(comment.id)
    return resolution ? { ...comment, ...resolution } : comment
  })
  return merged
}

// ---------------------------------------------------------------------------
// Lectures
// ---------------------------------------------------------------------------

/** Personnes de l'organisation, décisions de la session appliquées. */
function membersOf(organizationId: string): MemberEntry[] {
  const entries: MemberEntry[] = []
  for (const stored of allMemberships()) {
    const membership = sessionMembershipDecisions.get(stored.id) ?? stored
    if (membership.organization_id !== organizationId) continue
    // Une adhésion révoquée sort de la liste : elle est conservée en base pour
    // l'historique, elle n'est pas un membre.
    if (membership.status === 'revoked') continue
    const person = people.find((p) => p.id === membership.person_id)
    if (!person) continue
    entries.push({
      membership,
      person,
      // La DIRECTION de la demande, et non son statut : les deux « pending » ne
      // se traitent pas pareil — l'un se relance, l'autre s'accepte.
      is_invitation: membership.invited_at !== null,
    })
  }
  // Référents d'abord, puis par ancienneté : c'est l'ordre dans lequel on
  // cherche quelqu'un à qui parler.
  const rank: Record<Membership['role'], number> = { manager: 0, contributor: 1, member: 2 }
  return entries.sort(
    (a, b) =>
      rank[a.membership.role] - rank[b.membership.role] ||
      a.membership.created_at.localeCompare(b.membership.created_at),
  )
}

/** Demandes de correction encore ouvertes sur un dossier. */
function openChangeRequests(proposalId: string): number {
  return commentsWithSession().filter(
    (comment) =>
      comment.proposal_id === proposalId &&
      comment.is_change_request &&
      comment.resolved_at === null &&
      comment.deleted_at === null,
  ).length
}

/**
 * Les séances d'un dossier retenu, avec leurs inscrits et leurs rappels.
 *
 * Le décompte suit `ux_registrations_person_session` : une annulation libère la
 * place et sort du compte. La liste d'attente est comptée à part — elle ne
 * remplit pas la salle, mais elle dit qu'il y a plus de demande que de places.
 */
function trackedSessions(proposalId: string, at: number): TrackedSession[] {
  return allSessions
    .filter((session) => session.proposal_id === proposalId)
    .sort((a, b) => a.starts_at.localeCompare(b.starts_at))
    .map((session) => {
      const own = registrations.filter((registration) => registration.session_id === session.id)
      return {
        session,
        room: rooms.find((room) => room.id === session.room_id) ?? null,
        registered_count: own.filter((r) => r.status !== 'cancelled' && r.status !== 'waitlisted').length,
        waitlisted_count: own.filter((r) => r.status === 'waitlisted').length,
        capacity: session.capacity,
        reminders: sessionReminderSchedule(session.id, at),
      }
    })
}

/** Un dossier et tout ce qui permet d'en suivre l'avancement. */
function trackingOf(proposalId: string, at: number): ProposalTracking | null {
  const stored = allProposals.find((p) => p.id === proposalId)
  if (!stored) return null
  // Les modifications faites pendant la session s'appliquent par-dessus le
  // dossier écrit à la main : un titre corrigé doit s'afficher là où on vient
  // de le corriger, sinon la modification paraît perdue.
  const changes = editedProposal(proposalId)
  const proposal = changes ? { ...stored, ...changes } : stored
  const edition = events.find((e) => e.id === proposal.event_id)
  if (!edition) return null

  return {
    proposal,
    edition,
    transitions: proposalTransitions
      .filter((transition) => transition.proposal_id === proposalId)
      .sort((a, b) => a.occurred_at.localeCompare(b.occurred_at)),
    open_change_requests: openChangeRequests(proposalId),
    sessions: proposal.status === 'accepted' ? trackedSessions(proposalId, at) : [],
  }
}

/**
 * CE QUI ATTEND UNE ACTION DE L'ORGANISATION — et rien d'autre.
 *
 * Le critère d'entrée est unique et sans exception : l'organisation doit
 * pouvoir le débloquer elle-même. Ce qu'attend le comité — une revue, une
 * décision — n'y figure pas. Une liste où l'on trouve ce qu'on ne peut pas
 * traiter cesse d'être lue, et c'est alors la ligne qui comptait qu'on rate.
 *
 * L'ORDRE EST CELUI DE L'URGENCE, pas celui des tables : ce qui a une échéance
 * proche d'abord, ce qui n'en a pas ensuite.
 */
function actionsFor(
  organizationId: string,
  membership: Membership,
  trackings: ProposalTracking[],
  at: number,
): WorkspaceAction[] {
  const actions: WorkspaceAction[] = []
  const call = openCallForProposals(at)
  const deadline = call ? (call.extended_until ?? call.closes_at) : null

  for (const tracking of trackings) {
    const { proposal } = tracking
    const target = `/mon-organisation/dossiers/${proposal.id}`

    if (proposal.status === 'changes_requested') {
      actions.push({
        kind: 'changes_requested',
        proposal_id: proposal.id,
        reference_code: proposal.reference_code,
        subject: proposal.title.fr,
        count: tracking.open_change_requests,
        // Un dossier renvoyé pour correction doit repartir avant l'échéance de
        // l'appel : c'est la même date, et personne ne la rappelle autrement.
        due_at: deadline,
        target,
      })
    }

    if (proposal.status === 'draft' && call !== null && proposal.call_id === call.id) {
      actions.push({
        kind: 'draft_before_deadline',
        proposal_id: proposal.id,
        reference_code: proposal.reference_code,
        subject: proposal.title.fr,
        count: 1,
        due_at: deadline,
        // Vers CE dossier, pas vers un formulaire vierge : la personne a
        // commencé quelque chose, elle vient le finir.
        target: `/deposer-une-proposition?dossier=${proposal.id}`,
      })
    }

    // Séance TENUE et sans compte rendu : `sessions.report_submitted_at` est nul
    // alors que la séance est terminée. C'est la dernière chose que l'IFDD
    // attend d'une organisation, et la plus facile à oublier une fois la COP
    // finie.
    for (const tracked of tracking.sessions) {
      if (tracked.session.status === 'completed' && tracked.session.report_submitted_at === null) {
        actions.push({
          kind: 'session_report_missing',
          proposal_id: proposal.id,
          reference_code: proposal.reference_code,
          subject: tracked.session.title.fr,
          count: 1,
          due_at: null,
          target,
        })
      }
    }
  }

  // Co-organisations annoncées par des tiers et pas encore confirmées. Elles
  // engagent l'organisation sans qu'elle ait rien signé : c'est à elle de dire
  // oui, et le porteur attend.
  for (const link of proposalOrganizations) {
    if (link.organization_id !== organizationId) continue
    if (link.role === 'lead' || link.confirmed_at !== null) continue
    const proposal = allProposals.find((p) => p.id === link.proposal_id)
    if (!proposal) continue
    actions.push({
      kind: 'coorganization_to_confirm',
      proposal_id: proposal.id,
      reference_code: proposal.reference_code,
      subject: proposal.title.fr,
      count: 1,
      due_at: null,
      target: `/mon-organisation/dossiers/${proposal.id}`,
    })
  }

  // Demandes d'adhésion à trancher — le référent seul les voit, et il n'y a que
  // lui pour les traiter.
  if (membership.role === 'manager') {
    const requests = membersOf(organizationId).filter(
      (entry) => entry.membership.status === 'pending' && !entry.is_invitation,
    )
    for (const request of requests) {
      actions.push({
        kind: 'membership_request',
        proposal_id: null,
        reference_code: null,
        subject: `${request.person.first_name} ${request.person.last_name}`,
        count: 1,
        due_at: null,
        target: '/mon-organisation#membres',
      })
    }
  }

  return actions.sort((a, b) => {
    if (a.due_at && b.due_at) return a.due_at.localeCompare(b.due_at)
    if (a.due_at) return -1
    if (b.due_at) return 1
    return 0
  })
}

/**
 * TOUT L'ÉCRAN D'ACCUEIL EN UNE RÉPONSE.
 *
 * Rend `null` quand la personne n'a pas d'adhésion vivante à cette
 * organisation : l'écran affiche alors son état « accès refusé », plutôt qu'une
 * page vide qui laisserait croire à une organisation sans dossier.
 */
export function workspaceOverview(
  organizationId: string,
  personId: string,
  at: number = Date.now(),
): WorkspaceOverview | null {
  const organization = organizationById(organizationId)
  if (!organization) return null

  const membership = allMemberships().find(
    (m) => m.organization_id === organizationId && m.person_id === personId && m.status === 'active',
  )
  if (!membership) return null

  const trackings = allProposals
    .filter((proposal) => proposal.organization_id === organizationId && proposal.deleted_at === null)
    .map((proposal) => trackingOf(proposal.id, at))
    .filter((tracking) => tracking !== null)
    // Le plus récemment touché en tête : c'est celui dont on vient s'occuper.
    .sort((a, b) => b.proposal.updated_at.localeCompare(a.proposal.updated_at))

  const call = openCallForProposals(at)

  return {
    organization,
    membership,
    proposals: trackings,
    members: membersOf(organizationId),
    actions: actionsFor(organizationId, membership, trackings, at),
    open_call: call,
    call_edition: call ? (events.find((e) => e.id === call.event_id) ?? null) : null,
  }
}

/**
 * LE DÉTAIL D'UN DOSSIER : son suivi, son fil et son historique.
 *
 * Le fil est filtré sur `visibility = 'submitter'` — la seule visibilité que le
 * modèle destine au soumissionnaire. Les notes du comité et les notes
 * personnelles ne sortent pas d'ici : ce filtre est le même que celui de
 * `useApi().proposals.comments()`, appliqué une fois, à la source.
 */
export function proposalFile(
  proposalId: string,
  organizationId: string,
  at: number = Date.now(),
): ProposalFile | null {
  const tracking = trackingOf(proposalId, at)
  if (!tracking) return null
  // Un dossier ne se consulte que depuis l'organisation qui le porte. Ce n'est
  // pas le contrôle d'accès — l'API le refait — mais l'écran ne doit pas
  // afficher le dossier d'un tiers à qui forge une URL.
  if (tracking.proposal.organization_id !== organizationId) return null

  const comments = commentsWithSession()
    .filter(
      (comment) =>
        comment.proposal_id === proposalId &&
        comment.visibility === 'submitter' &&
        comment.deleted_at === null,
    )
    .sort((a, b) => a.created_at.localeCompare(b.created_at))

  const authorIds = new Set(comments.map((comment) => comment.author_id))
  const participants: Person[] = people.filter((person) => authorIds.has(person.id))

  return {
    tracking,
    comments,
    participants,
    history: proposalHistory(proposalId),
  }
}

/** Éditions de l'organisation, pour grouper la liste sans recharger le tout. */
export function workspaceEditions(organizationId: string): EventEdition[] {
  const ids = new Set(
    allProposals
      .filter((proposal) => proposal.organization_id === organizationId)
      .map((proposal) => proposal.event_id),
  )
  return events
    .filter((edition) => ids.has(edition.id))
    .sort((a, b) => b.starts_at.localeCompare(a.starts_at))
}

// ---------------------------------------------------------------------------
// Écritures
// ---------------------------------------------------------------------------

/**
 * INVITATION D'UN MEMBRE PAR SON ADRESSE.
 *
 * TROIS CHOSES SE PASSENT EN BASE, et l'écran doit les rendre dans cet ordre :
 *  1. la PERSONNE est créée si l'adresse est inconnue — `identity.people` existe
 *     sans compte, c'est ce que la séparation personne / compte permet et ce que
 *     la v1 ne savait pas faire ;
 *  2. l'ADHÉSION naît `pending` avec `invited_by` et `invited_at` : c'est ce qui
 *     la distingue d'une demande spontanée, et ce qui évite qu'un référent
 *     approuve sa propre invitation ;
 *  3. le JETON part par courriel (`identity.one_time_tokens`, finalité
 *     `invitation`). Il n'est pas simulé ici : aucun écran ne le lit, seul le
 *     lien reçu le consomme.
 *
 * ON N'INVITE PAS DEUX FOIS. Une invitation déjà en vol se relance, elle ne se
 * réémet pas : `ux_memberships` refuserait la seconde ligne, et l'écran doit le
 * dire avant de la tenter.
 */
export function inviteMember(actorId: string, payload: InviteMemberPayload): InviteMemberResult {
  const email = payload.email.trim().toLowerCase()
  const now = new Date().toISOString()

  const existingPerson = people.find((person) => person.primary_email.toLowerCase() === email)

  if (existingPerson) {
    const existing = allMemberships().find(
      (m) =>
        m.organization_id === payload.organization_id &&
        m.person_id === existingPerson.id &&
        m.status !== 'revoked',
    )
    if (existing) {
      const entry: MemberEntry = {
        membership: existing,
        person: existingPerson,
        is_invitation: existing.invited_at !== null,
      }
      return {
        status: existing.status === 'active' ? 'already_member' : 'already_invited',
        entry,
      }
    }
  }

  // Personne inconnue : la fiche est créée, SANS COMPTE. C'est ce que la
  // séparation personne / compte permet, et l'invitation qui lui en fera ouvrir
  // un — pas l'inverse. Le nom reste vide tant qu'elle ne l'a pas donné : le
  // déduire de l'adresse fabriquerait un « a.diallo » qu'aucun écran ne saurait
  // corriger ensuite.
  const person: Person =
    existingPerson ??
    ({
      id: PERSON_INVITED(nextSessionMembershipIndex()),
      primary_email: email,
      email_verified_at: null,
      first_name: '',
      last_name: '',
      civility: null,
      display_name: '',
      phone: null,
      job_title: payload.job_title,
      biography: null,
      country_id: null,
      city: null,
      preferred_locale: 'fr',
      timezone: 'Africa/Dakar',
      primary_organization_id: null,
      status: 'active',
      status_reason: null,
      status_changed_by: null,
      status_changed_at: null,
      suspended_until: null,
      is_directory_visible: false,
      created_at: now,
      updated_at: now,
    } satisfies Person)

  const membership: Membership = {
    id: MEMBERSHIP(900 + nextSessionMembershipIndex()),
    organization_id: payload.organization_id,
    person_id: person.id,
    role: payload.role,
    status: 'pending',
    // `tg_default_primary_membership` ne pose la primauté que sur une adhésion
    // ACTIVE : une invitation en attente n'en est pas une.
    is_primary: false,
    job_title: payload.job_title,
    invited_by: actorId,
    invited_at: now,
    approved_by: null,
    approved_at: null,
    revoked_at: null,
    created_at: now,
    updated_at: now,
  }
  addSessionMembership(membership)

  return { status: 'invited', entry: { membership, person, is_invitation: true } }
}

/**
 * DÉCISION D'UN RÉFÉRENT SUR UNE DEMANDE D'ADHÉSION.
 *
 * Approuvée, l'adhésion passe `active` et porte son approbateur ; refusée, elle
 * passe `revoked` plutôt que d'être effacée — la v1 supprimait la ligne, et
 * personne ne pouvait plus dire si une demande avait été refusée ou jamais faite.
 *
 * UNE INVITATION NE S'APPROUVE PAS. Elle attend la personne, pas le référent :
 * la fonction refuse donc de la traiter, et c'est `invited_at` qui les
 * distingue.
 */
export function decideMembership(
  actorId: string,
  payload: DecideMembershipPayload,
): Membership | null {
  const membership = allMemberships().find((m) => m.id === payload.membership_id)
  if (!membership || membership.status !== 'pending' || membership.invited_at !== null) return null

  const now = new Date().toISOString()
  const decided: Membership = payload.approved
    ? {
        ...membership,
        status: 'active',
        approved_by: actorId,
        approved_at: now,
        // `tg_default_primary_membership` : la première adhésion ACTIVE d'une
        // personne devient sa principale. On reproduit la règle de la base, on
        // ne la réinvente pas.
        is_primary: !allMemberships().some(
          (m) => m.person_id === membership.person_id && m.status === 'active' && m.is_primary,
        ),
        updated_at: now,
      }
    : { ...membership, status: 'revoked', revoked_at: now, updated_at: now }

  sessionMembershipDecisions.set(membership.id, decided)
  return decided
}

/**
 * RÉPONSE À UN MESSAGE DU COMITÉ.
 *
 * La réponse est toujours `submitter` : elle appartient au fil partagé, et une
 * organisation n'a de toute façon pas accès aux deux autres visibilités. Elle
 * n'est jamais une demande de correction — seul le comité en formule.
 */
export function replyToComment(authorId: string, payload: ReplyToCommentPayload): ProposalComment {
  const now = new Date().toISOString()
  const reply: ProposalComment = {
    id: PROPOSAL_COMMENT(900 + sessionComments.length),
    proposal_id: payload.proposal_id,
    parent_id: payload.parent_id,
    author_id: authorId,
    visibility: 'submitter',
    body: payload.body.trim(),
    is_change_request: false,
    resolved_at: null,
    resolved_by: null,
    edited_at: null,
    deleted_at: null,
    created_at: now,
  }
  sessionComments.push(reply)
  return reply
}

/**
 * MARQUAGE « RÉSOLU » D'UNE DEMANDE DE CORRECTION.
 *
 * Le modèle porte `resolved_at` et `resolved_by` sans dire QUI peut les écrire.
 * L'écran l'ouvre au soumissionnaire — c'est lui qui sait qu'il a corrigé — et
 * le laisse revenir en arrière : une case cochée trop vite ne doit pas exiger un
 * courriel à l'IFDD pour être décochée. La règle d'autorisation appartient à
 * l'API (`identity.has_permission`), pas à ce formulaire.
 */
export function resolveComment(personId: string, payload: ResolveCommentPayload): ProposalComment | null {
  const comment = commentsWithSession().find((c) => c.id === payload.comment_id)
  if (!comment) return null

  const resolution = payload.resolved
    ? { resolved_at: new Date().toISOString(), resolved_by: personId }
    : { resolved_at: null, resolved_by: null }
  sessionResolutions.set(comment.id, resolution)

  return { ...comment, ...resolution }
}
