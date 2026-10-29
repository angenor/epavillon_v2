import { test } from 'node:test'
import assert from 'node:assert/strict'
import {
  defaultEditionTab,
  matchesProposalFilter,
  matchesProposalQuery,
  proposalActionIndex,
  workspaceEditionTabs,
} from '../../app/utils/workspace-editions.ts'
import type { EventEdition } from '../../app/types/event/edition.ts'
import type { ProposalTracking, WorkspaceAction, WorkspaceOverview } from '../../app/types/organization-workspace.ts'

const NOW = Date.parse('2026-10-28T12:00:00Z')

const edition = (id: string, starts_at: string, ends_at: string, status = 'announced') =>
  ({ id, slug: id, acronym: id.toUpperCase(), title: { fr: id }, starts_at, ends_at, status, timezone: 'UTC' }) as unknown as EventEdition

const cop31 = edition('cop31', '2026-11-09T08:00:00Z', '2026-11-20T18:00:00Z')
const cdp30 = edition('cdp30', '2025-11-10T08:00:00Z', '2025-11-21T18:00:00Z', 'completed')

const tracking = (id: string, ed: EventEdition, status = 'submitted', title = id) =>
  ({
    proposal: { id, status, title: { fr: title }, reference_code: `P-${id}` },
    edition: ed,
    sessions: [],
    cover: null,
    edition_thumbnail: null,
    edition_logo: null,
  }) as unknown as ProposalTracking

const action = (kind: WorkspaceAction['kind'], proposal_id: string | null) =>
  ({ kind, proposal_id, reference_code: null, subject: '', count: 1, due_at: null, target: '' }) as WorkspaceAction

const overview = (proposals: ProposalTracking[], actions: WorkspaceAction[], call_edition: EventEdition | null = null) =>
  ({ proposals, actions, call_edition }) as unknown as WorkspaceOverview

test("les comptes rendus d'une édition terminée sont comptés à part, pas listés", () => {
  const tabs = workspaceEditionTabs(
    overview(
      [tracking('a', cop31, 'changes_requested'), tracking('b', cdp30, 'accepted'), tracking('c', cdp30, 'accepted')],
      [action('changes_requested', 'a'), action('session_report_missing', 'b'), action('session_report_missing', 'c')],
    ),
    NOW,
  )
  assert.deepEqual(tabs.map((tab) => tab.edition.id), ['cop31', 'cdp30'])
  assert.equal(tabs[0]?.actions.length, 1)
  assert.equal(tabs[1]?.actions.length, 0)
  assert.equal(tabs[1]?.missingReports, 2)
})

test("une demande d'adhésion suit l'onglet vivant, et l'appel ouvert a son onglet même sans dossier", () => {
  const tabs = workspaceEditionTabs(overview([tracking('b', cdp30)], [action('membership_request', null)], cop31), NOW)
  assert.deepEqual(tabs.map((tab) => [tab.edition.id, tab.actions.length]), [['cop31', 1], ['cdp30', 0]])
  assert.equal(defaultEditionTab(tabs)?.edition.id, 'cop31')
  assert.equal(defaultEditionTab(tabs, 'cdp30')?.edition.id, 'cdp30')
})

test("sans édition vivante, l'onglet ouvert est la plus récente", () => {
  const tabs = workspaceEditionTabs(overview([tracking('b', cdp30)], []), NOW)
  assert.equal(defaultEditionTab(tabs)?.edition.id, 'cdp30')
})

test('la recherche ignore accents et casse, et lit le numéro', () => {
  const t = tracking('42', cop31, 'submitted', 'Forêts tropicales')
  assert.ok(matchesProposalQuery(t, 'FORET'))
  assert.ok(matchesProposalQuery(t, 'p-42'))
  assert.ok(!matchesProposalQuery(t, 'mangrove'))
})

test('le filtre « à vous d’agir » réunit corrections, brouillons et comptes rendus', () => {
  const index = proposalActionIndex([action('changes_requested', 'a'), action('session_report_missing', 'b')])
  assert.ok(matchesProposalFilter(tracking('a', cop31, 'changes_requested'), 'todo', index))
  assert.ok(matchesProposalFilter(tracking('b', cdp30, 'accepted'), 'todo', index))
  assert.ok(matchesProposalFilter(tracking('b', cdp30, 'accepted'), 'report', index))
  assert.ok(!matchesProposalFilter(tracking('c', cop31, 'submitted'), 'todo', index))
})
