/**
 * Les fils d'un dossier tels qu'une personne les lit : destinataires d'un fil du
 * comité (`programme.proposal_comment_recipients`) et lectures message par
 * message (`programme.proposal_comment_reads`), `070` § 6.
 *
 * CE QUE LE JEU RACONTE, pour Mme Perret (le compte montré par défaut) :
 *  · deux fils restreints, dont un où elle est destinataire sans l'avoir écrit ;
 *  · des réponses qu'elle n'a pas lues, d'où le point et le compteur de la liste ;
 *  · un message partagé qu'elle a écrit, lu par deux membres du comité et par
 *    l'organisation ; un autre que l'organisation n'a pas encore ouvert.
 *
 * Ce module porte aussi le tampon des messages écrits pendant la visite : la
 * fiche (A8) et la liste (A7) le lisent toutes deux, et le laisser dans
 * `proposal-review.ts` aurait fait boucler les imports de la liste.
 */

import type { PostCommentPayload } from '~/types/admin-review'
import type { UnreadCommentCount } from '~/types/admin-proposals'
import type {
  AddCommentRecipientsPayload,
  CommentPerson,
  CommentReader,
  MarkCommentsReadPayload,
  Proposal,
  ProposalComment,
  ProposalCommentView,
} from '~/types/programme/proposal'
import type { PersonId, Uuid } from '~/types/shared'
import { PERSON, PROPOSAL_COMMENT } from './ids'
import { people } from './people'
import { effectivePermissions } from './permissions'
import { allProposals, proposalComments } from './proposals'
import { reviewAssignments } from './reviews'

const personById = new Map(people.map((person) => [person.id, person]))

// ---------------------------------------------------------------------------
// Données
// ---------------------------------------------------------------------------

const recipientsByRoot = new Map<Uuid, PersonId[]>([
  [PROPOSAL_COMMENT(7), [PERSON.duchesne, PERSON.perretAdmin]],
  [PROPOSAL_COMMENT(12), [PERSON.perretAdmin, PERSON.rasoanaivo]],
])

const readKey = (commentId: Uuid, personId: PersonId): string => `${commentId}:${personId}`

/** (message, lecteur) vers l'heure de lecture ; « non lu » est l'absence de clé. */
const reads = new Map<string, string>(
  (
    [
      [1, PERSON.moreau, '2026-08-10T12:02:00Z'],
      [1, PERSON.perretAdmin, '2026-08-10T09:15:00Z'],
      [2, PERSON.bakayoko, '2026-08-11T15:00:00Z'],
      [7, PERSON.duchesne, '2026-07-25T12:00:00Z'],
      [8, PERSON.lemoine, '2026-07-25T14:10:00Z'],
      [12, PERSON.rasoanaivo, '2026-08-05T16:30:00Z'],
      [12, PERSON.perretAdmin, '2026-08-06T08:00:00Z'],
      [13, PERSON.duchesne, '2026-08-06T09:20:00Z'],
      [14, PERSON.duchesne, '2026-08-07T09:00:00Z'],
      [14, PERSON.ngoBassong, '2026-08-07T10:40:00Z'],
      [14, PERSON.rasoanaivo, '2026-08-08T07:30:00Z'],
      [15, PERSON.perretAdmin, '2026-08-08T12:00:00Z'],
    ] as const
  ).map(([n, personId, at]) => [readKey(PROPOSAL_COMMENT(n), personId), at]),
)

/** Messages écrits pendant la visite. */
const sessionComments: ProposalComment[] = []
let commentSeq = 900

function allComments(): ProposalComment[] {
  return [...proposalComments, ...sessionComments]
}

// ---------------------------------------------------------------------------
// Qui est du comité, qui lit quoi
// ---------------------------------------------------------------------------

const COMMITTEE_PERMISSIONS: readonly string[] = [
  'programme.review.write',
  'programme.proposal.decide',
  'event.call.manage',
]

/** Le comité d'un dossier : les permissions sur son édition, plus les révisionnistes affectés. */
function committeeOf(proposal: Proposal): Set<PersonId> {
  const ids = new Set<PersonId>()
  for (const person of people) {
    const sits = effectivePermissions(person.id).some(
      (grant) =>
        COMMITTEE_PERMISSIONS.includes(grant.permission_code) &&
        (grant.scope_type === 'global' ||
          (grant.scope_type === 'event' && grant.scope_id === proposal.event_id)),
    )
    if (sits) ids.add(person.id)
  }
  for (const assignment of reviewAssignments) {
    if (assignment.proposal_id === proposal.id && assignment.recused_at === null) ids.add(assignment.reviewer_id)
  }
  return ids
}

function rootOf(comment: ProposalComment, all: ProposalComment[]): ProposalComment {
  let current = comment
  const seen = new Set<Uuid>()
  while (current.parent_id && !seen.has(current.id)) {
    seen.add(current.id)
    const parent = all.find((entry) => entry.id === current.parent_id)
    if (!parent) break
    current = parent
  }
  return current
}

/** `programme.comment_in_committee_circle()` rejouée, plus le fil partagé pour l'organisation. */
function canRead(
  comment: ProposalComment,
  viewerId: PersonId,
  committee: Set<PersonId>,
  all: ProposalComment[],
): boolean {
  if (comment.visibility === 'private') return comment.author_id === viewerId
  if (comment.visibility === 'submitter') return true
  const root = rootOf(comment, all)
  const circle = recipientsByRoot.get(root.id) ?? []
  if (circle.length > 0) return root.author_id === viewerId || circle.includes(viewerId)
  return committee.has(viewerId) || comment.author_id === viewerId
}

function audienceOf(comment: ProposalComment, committee: Set<PersonId>, all: ProposalComment[]): number {
  if (comment.visibility === 'private') return 0
  const root = rootOf(comment, all)
  const circle = comment.visibility === 'committee' ? (recipientsByRoot.get(root.id) ?? []) : []
  const audience = circle.length > 0 ? new Set([root.author_id, ...circle]) : new Set(committee)
  audience.delete(comment.author_id)
  return audience.size
}

function namedPerson(personId: PersonId): CommentPerson {
  return { person_id: personId, display_name: personById.get(personId)?.display_name ?? '' }
}

function readersOf(comment: ProposalComment, committee: Set<PersonId>): CommentReader[] {
  const prefix = `${comment.id}:`
  const readers: CommentReader[] = []
  for (const [key, at] of reads) {
    if (!key.startsWith(prefix)) continue
    const personId = key.slice(prefix.length)
    if (personId === comment.author_id) continue
    readers.push({
      ...namedPerson(personId),
      read_at: at,
      side: committee.has(personId) ? 'committee' : 'organisation',
    })
  }
  return readers.sort((a, b) => a.read_at.localeCompare(b.read_at))
}

function viewOf(
  comment: ProposalComment,
  viewerId: PersonId,
  committee: Set<PersonId>,
  all: ProposalComment[],
): ProposalCommentView {
  const mine = comment.author_id === viewerId
  const restricted = comment.parent_id === null && comment.visibility === 'committee'
  return {
    ...comment,
    recipients: restricted ? (recipientsByRoot.get(comment.id) ?? []).map(namedPerson) : [],
    is_read: mine || reads.has(readKey(comment.id, viewerId)),
    receipts: mine
      ? { audience_count: audienceOf(comment, committee, all), readers: readersOf(comment, committee) }
      : null,
  }
}

function proposalOf(proposalId: Uuid): Proposal | null {
  return allProposals.find((proposal) => proposal.id === proposalId) ?? null
}

// ---------------------------------------------------------------------------
// Lectures
// ---------------------------------------------------------------------------

/** Le tampon de session, pour les compositions qui filtrent elles-mêmes (A8). */
export function sessionProposalComments(): ProposalComment[] {
  return sessionComments
}

/**
 * `GET /proposals/{id}/comments`. Sans lecteur (hors ligne, pas de session),
 * le seul fil que tout lecteur a le droit de voir : le fil partagé.
 */
export function proposalCommentViews(proposalId: Uuid, viewerId: PersonId | null): ProposalCommentView[] {
  const proposal = proposalOf(proposalId)
  if (!proposal) return []
  const committee = committeeOf(proposal)
  const all = allComments()
  return all
    .filter((comment) => comment.proposal_id === proposalId)
    .filter((comment) =>
      viewerId ? canRead(comment, viewerId, committee, all) : comment.visibility === 'submitter',
    )
    .sort((a, b) => a.created_at.localeCompare(b.created_at))
    .map((comment) => viewOf(comment, viewerId ?? '', committee, all))
}

/** `GET /proposals/{id}/comment-audience`, par ordre alphabétique. */
export function commentAudience(proposalId: Uuid): CommentPerson[] {
  const proposal = proposalOf(proposalId)
  if (!proposal) return []
  return [...committeeOf(proposal)]
    .map(namedPerson)
    .filter((person) => person.display_name)
    .sort((a, b) => a.display_name.localeCompare(b.display_name, 'fr'))
}

/** Habille un message écrit ailleurs (réponse de l'organisation, A5). */
export function asCommentView(comment: ProposalComment, viewerId: PersonId): ProposalCommentView {
  const proposal = proposalOf(comment.proposal_id)
  const committee = proposal ? committeeOf(proposal) : new Set<PersonId>()
  return viewOf(comment, viewerId, committee, [...allComments(), comment])
}

/** `unread_comments` de la liste (A7) : seuls les dossiers qui en ont. */
export function unreadCommentCounts(eventId: Uuid, viewerId: PersonId | null): UnreadCommentCount[] {
  if (!viewerId) return []
  return allProposals
    .filter((proposal) => proposal.event_id === eventId && proposal.deleted_at === null)
    .map((proposal) => ({
      proposal_id: proposal.id,
      count: proposalCommentViews(proposal.id, viewerId).filter((view) => !view.is_read).length,
    }))
    .filter((entry) => entry.count > 0)
}

// ---------------------------------------------------------------------------
// Écritures
// ---------------------------------------------------------------------------

/**
 * ÉCRIRE SUR LE DOSSIER. Une demande de correction est forcée en `submitter` :
 * une demande que le déposant ne verrait pas bloquerait son dossier sans qu'il
 * sache pourquoi. Les destinataires ne tiennent que sur un message du comité
 * qui ouvre un fil (`tg_check_comment_recipient`).
 */
export function postProposalComment(
  personId: PersonId,
  payload: PostCommentPayload,
  at: number = Date.now(),
): ProposalCommentView {
  const comment: ProposalComment = {
    id: PROPOSAL_COMMENT(++commentSeq),
    proposal_id: payload.proposal_id,
    parent_id: payload.parent_id,
    author_id: personId,
    visibility: payload.is_change_request ? 'submitter' : payload.visibility,
    body: payload.body.trim(),
    is_change_request: payload.is_change_request,
    resolved_at: null,
    resolved_by: null,
    edited_at: null,
    created_at: new Date(at).toISOString(),
    topic: null,
    topic_other: null,
  }
  const recipients = [...new Set(payload.recipient_ids ?? [])].filter((id) => id !== personId)
  if (recipients.length > 0) {
    if (comment.parent_id !== null || comment.visibility !== 'committee') {
      throw new Error('Seul un message du comité qui ouvre un fil a des destinataires.')
    }
    recipientsByRoot.set(comment.id, recipients)
  }
  sessionComments.push(comment)
  return asCommentView(comment, personId)
}

/** `POST /proposal-comments/{id}/recipients` : la personne ajoutée voit tout le fil. */
export function addCommentRecipients(
  personId: PersonId,
  commentId: Uuid,
  payload: AddCommentRecipientsPayload,
): void {
  const root = allComments().find((comment) => comment.id === commentId)
  if (!root || root.parent_id !== null || root.visibility !== 'committee') {
    throw new Error('Seul un message du comité qui ouvre un fil a des destinataires.')
  }
  const current = recipientsByRoot.get(commentId) ?? []
  const added = payload.person_ids.filter((id) => id !== root.author_id && id !== personId)
  recipientsByRoot.set(commentId, [...new Set([...current, ...added])])
}

export function hasReadComment(commentId: Uuid, personId: PersonId): boolean {
  return reads.has(readKey(commentId, personId))
}

/** `POST /proposals/{id}/comments/read` : idempotent, la première lecture reste. */
export function markCommentsRead(personId: PersonId, payload: MarkCommentsReadPayload): void {
  const at = new Date().toISOString()
  for (const commentId of payload.comment_ids) {
    const key = readKey(commentId, personId)
    if (!reads.has(key)) reads.set(key, at)
  }
}

/** `DELETE /proposal-comments/{id}/read`. */
export function markCommentUnread(personId: PersonId, commentId: Uuid): void {
  reads.delete(readKey(commentId, personId))
}
