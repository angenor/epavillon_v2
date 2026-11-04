import { test } from 'node:test'
import assert from 'node:assert/strict'
import { canOrganizationCancel, proposalCancellation } from '../../app/utils/proposal-cancellation.ts'
import { buildProposalTimeline } from '../../app/utils/proposal-timeline.ts'
import { nextWorkspaceActivity } from '../../app/utils/workspace-editions.ts'
import type { ProposalTracking } from '../../app/types/organization-workspace.ts'
import type { ProposalHistoryEntry, ProposalTransition } from '../../app/types/programme/proposal.ts'

const NOW = Date.parse('2026-11-04T12:00:00Z')

const edition = { id: 'cop31', starts_at: '2026-11-09T08:00:00Z', ends_at: '2026-11-20T18:00:00Z', status: 'announced' }

const session = (id: string, starts_at: string, ends_at: string, status = 'scheduled') => ({
  session: { id, starts_at, ends_at, status },
  room: null,
  registered_count: 0,
  waitlisted_count: 0,
  capacity: 40,
  reminders: [],
})

const transition = (to_status: string, from_status: string | null, occurred_at: string, reason: string | null = null) =>
  ({ id: occurred_at, proposal_id: 'p', from_status, to_status, actor_id: 'aissatou', reason, occurred_at }) as ProposalTransition

const tracking = (status: string, sessions: ReturnType<typeof session>[], transitions: ProposalTransition[] = []) =>
  ({
    proposal: { id: 'p', status, created_at: '2026-09-22T09:00:00Z', submitted_at: '2026-10-03T09:00:00Z', decided_at: '2026-10-27T09:00:00Z', decision_reason: null },
    edition,
    transitions,
    sessions,
  }) as unknown as ProposalTracking

test("l'annulation s'offre à un dossier retenu tant qu'aucune séance n'a commencé", () => {
  const later = session('s1', '2026-11-14T06:50:00Z', '2026-11-14T07:40:00Z')
  assert.equal(canOrganizationCancel(tracking('accepted', [later]), NOW), true)
  assert.equal(canOrganizationCancel(tracking('accepted', []), NOW), true)
  assert.equal(canOrganizationCancel(tracking('under_review', [later]), NOW), false)
  const started = session('s0', '2026-11-04T11:00:00Z', '2026-11-04T13:00:00Z')
  assert.equal(canOrganizationCancel(tracking('accepted', [later, started]), NOW), false)
  assert.equal(canOrganizationCancel(tracking('accepted', [later]), Date.parse('2026-11-21T00:00:00Z')), false)
})

test("l'annulation se lit dans le journal : quand, pourquoi, et le nom que lui donne l'historique", () => {
  const transitions = [
    transition('accepted', 'under_review', '2026-10-27T09:00:00Z'),
    transition('cancelled', 'accepted', '2026-11-04T10:00:00Z', ' Intervenante empêchée. '),
  ]
  const history = [
    { occurred_at: '2026-11-04T10:00:00Z', actor_id: 'aissatou', actor_label: 'Aïssatou Ndiaye', action: 'update', field: 'status', old_value: 'accepted', new_value: 'cancelled' },
  ] as ProposalHistoryEntry[]
  assert.deepEqual(proposalCancellation(transitions, history), {
    at: '2026-11-04T10:00:00Z',
    actorId: 'aissatou',
    actorLabel: 'Aïssatou Ndiaye',
    reason: 'Intervenante empêchée.',
    afterAcceptance: true,
  })
  assert.equal(proposalCancellation(transitions.slice(0, 1), history), null)
})

const LABELS = {
  draft: 'Brouillon',
  submitted: 'Déposé',
  under_review: 'En évaluation',
  changes_requested: 'Corrections demandées',
  decision: 'Décision',
  accepted: 'Retenu',
  rejected: 'Non retenu',
  withdrawn: 'Retiré',
  cancelled: 'Annulé',
  activityCancelled: 'Annulée',
  scheduled: 'Programmé',
}

test('la frise d’une activité annulée garde « Retenu » et se termine sur « Annulée »', () => {
  const steps = buildProposalTimeline(
    tracking('cancelled', [], [
      transition('under_review', 'submitted', '2026-10-06T09:00:00Z'),
      transition('accepted', 'under_review', '2026-10-27T09:00:00Z'),
      transition('cancelled', 'accepted', '2026-11-04T10:00:00Z', 'Intervenante empêchée.'),
    ]),
    LABELS,
  )
  assert.deepEqual(
    steps.map((step) => [step.label, step.state]),
    [['Brouillon', 'done'], ['Déposé', 'done'], ['En évaluation', 'done'], ['Retenu', 'done'], ['Annulée', 'refused']],
  )
  assert.equal(steps.at(-1)?.at, '2026-11-04T10:00:00Z')
})

test('la prochaine activité est la plus proche séance pas encore finie, hors annulées', () => {
  const a = tracking('accepted', [session('s1', '2026-11-16T08:30:00Z', '2026-11-16T09:30:00Z')])
  const b = tracking('accepted', [
    session('s2', '2026-11-04T08:00:00Z', '2026-11-04T09:00:00Z'),
    session('s3', '2026-11-14T06:50:00Z', '2026-11-14T07:40:00Z', 'cancelled'),
    session('s4', '2026-11-15T06:50:00Z', '2026-11-15T07:40:00Z'),
  ])
  assert.equal(nextWorkspaceActivity([a, b], NOW)?.session.session.id, 's4')
  assert.equal(nextWorkspaceActivity([tracking('submitted', [session('s5', '2026-11-10T08:00:00Z', '2026-11-10T09:00:00Z')])], NOW), null)
})
