/**
 * L'assistance (`/assistance`), hors ligne. Même partage qu'en base
 * (`programme.support_visible_comments`) : l'équipe lit les messages partagés
 * de tous les dossiers et les sujets sur la plateforme ; une organisation, ceux
 * de ses dossiers et ses propres sujets.
 */
import type {
  AssistanceFeed,
  AssistanceMessage,
  AssistanceStatus,
  MarkAssistanceReadPayload,
  OpenSupportSubjectPayload,
  ReplySupportSubjectPayload,
} from '~/types/programme/assistance'
import type { ProposalComment } from '~/types/programme/proposal'
import type { PersonId, Uuid } from '~/types/shared'
import { PERSON } from './ids'
import { memberships } from './memberships'
import { organizations } from './org'
import { people } from './people'
import { effectivePermissions } from './permissions'
import { allComments, hasReadComment, markCommentsRead, platformComments as platform } from './proposal-comments'
import { allProposals } from './proposals'

const OPEN_STATUSES = new Set(['submitted', 'under_review', 'changes_requested', 'accepted'])
const HOUR = 3_600_000
let seq = 0

const nameOf = (personId: PersonId) => people.find((p) => p.id === personId)?.display_name ?? ''

function platformComment(author: PersonId, body: string, hoursAgo: number, root?: ProposalComment): ProposalComment {
  return {
    id: `0198c1a0-0000-7099-8000-${String(++seq).padStart(12, '0')}`,
    proposal_id: null,
    parent_id: root?.id ?? null,
    author_id: author,
    visibility: 'submitter',
    body,
    is_change_request: false,
    resolved_at: null,
    resolved_by: null,
    edited_at: null,
    created_at: new Date(Date.now() - hoursAgo * HOUR).toISOString(),
    topic: null,
    topic_other: null,
    subject: null,
    closed_at: null,
    closed_by: null,
  }
}

const logo = { ...platformComment(PERSON.sowFall, 'Le bouton « Changer le logo » ne réagit pas sur notre fiche.', 3), topic: 'technical_issue', subject: 'Impossible de changer notre logo' }
const idea = { ...platformComment(PERSON.sowFall, 'Une page qui montre tous nos dossiers serait plus simple.', 50), topic: 'suggestion', subject: 'Voir tous nos dossiers sur une page' }
if (platform.length === 0) {
  platform.push(logo, idea, platformComment(PERSON.bakayoko, 'Merci, c’est noté pour une prochaine version.', 49, idea))
}

const isTeam = (personId: PersonId) =>
  effectivePermissions(personId).some((grant) => grant.permission_code === 'programme.support.answer')

const memberOf = (personId: PersonId, organizationId: Uuid) =>
  memberships.some((m) => m.person_id === personId && m.organization_id === organizationId && m.status !== 'revoked')

const followed = (personId: PersonId) =>
  allProposals.filter((p) => OPEN_STATUSES.has(p.status) && memberOf(personId, p.organization_id))

function rootOf(comment: ProposalComment, all: ProposalComment[]): ProposalComment {
  let current = comment
  while (current.parent_id) {
    const parent = all.find((c) => c.id === current.parent_id)
    if (!parent) break
    current = parent
  }
  return current
}

function messagesFor(personId: PersonId): AssistanceMessage[] {
  const team = isTeam(personId)
  const dossiers = allComments().filter((c) => {
    if (c.visibility !== 'submitter' || !c.proposal_id) return false
    const proposal = allProposals.find((p) => p.id === c.proposal_id)
    return Boolean(proposal) && (team || memberOf(personId, proposal!.organization_id))
  })
  const own = platform.filter((c) => team || rootOf(c, platform).author_id === personId)

  return [...dossiers, ...own]
    .map((c): AssistanceMessage => {
      const proposal = c.proposal_id ? allProposals.find((p) => p.id === c.proposal_id) : undefined
      const fromTeam = proposal ? !memberOf(c.author_id, proposal.organization_id) : c.author_id !== rootOf(c, platform).author_id
      const organizationId =
        proposal?.organization_id ?? memberships.find((m) => m.person_id === c.author_id && m.status === 'active')?.organization_id
      const organization = organizations.find((o) => o.id === organizationId)
      return {
        id: c.id,
        proposal_id: c.proposal_id,
        parent_id: c.parent_id,
        author_id: c.author_id,
        author_name: !team && fromTeam ? null : nameOf(c.author_id),
        from_team: fromTeam,
        body: c.body,
        is_change_request: c.is_change_request,
        resolved_at: c.resolved_at,
        edited_at: c.edited_at,
        created_at: c.created_at,
        topic: c.topic,
        topic_other: c.topic_other,
        subject: c.subject,
        closed_at: c.closed_at,
        closed_by: c.closed_by,
        is_read: c.author_id === personId || hasReadComment(c.id, personId),
        reference_code: proposal?.reference_code ?? null,
        proposal_title: proposal?.title ?? null,
        organization_name: c.parent_id ? null : (organization?.acronym ?? organization?.legal_name ?? null),
      }
    })
    .sort((a, b) => a.created_at.localeCompare(b.created_at))
}

export function assistanceStatus(personId: PersonId): AssistanceStatus {
  const side = isTeam(personId) ? 'team' : followed(personId).length > 0 ? 'organization' : null
  if (!side) return { side: null, unread: 0 }
  return { side, unread: messagesFor(personId).filter((m) => !m.is_read).length }
}

export function assistanceFeed(personId: PersonId): AssistanceFeed {
  const team = isTeam(personId)
  const proposals = team ? [] : followed(personId)
  return {
    side: team ? 'team' : 'organization',
    messages: messagesFor(personId),
    proposals: proposals.map((p) => ({ id: p.id, reference_code: p.reference_code ?? '', title: p.title })),
    organization_ids: [...new Set(proposals.map((p) => p.organization_id))],
  }
}

export function openSupportSubject(personId: PersonId, payload: OpenSupportSubjectPayload): ProposalComment {
  const root = { ...platformComment(personId, payload.body.trim(), 0), topic: payload.topic, subject: payload.subject.trim() }
  platform.push(root)
  return root
}

export function replySupportSubject(personId: PersonId, rootId: Uuid, payload: ReplySupportSubjectPayload): ProposalComment {
  const root = platform.find((c) => c.id === rootId && c.parent_id === null)
  if (!root) throw new Error('Sujet introuvable.')
  const reply = platformComment(personId, payload.body.trim(), 0, root)
  platform.push(reply)
  root.closed_at = null
  root.closed_by = null
  return reply
}

export function markAssistanceRead(personId: PersonId, payload: MarkAssistanceReadPayload): void {
  markCommentsRead(personId, payload)
}
