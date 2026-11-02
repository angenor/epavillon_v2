import { test } from 'node:test'
import assert from 'node:assert/strict'
import { editionMilestones } from '../../app/utils/edition-milestones.ts'

const call = {
  status: 'open' as const,
  opens_at: '2026-08-21T09:00:00Z',
  closes_at: '2026-10-26T20:59:00Z',
  extended_until: null,
  results_expected_at: null,
}
const edition = {
  starts_at: '2026-11-08T21:00:00Z',
  ends_at: '2026-11-20T20:59:00Z',
  programme_published_at: null,
}
const states = (steps: ReturnType<typeof editionMilestones>) => steps.map((step) => `${step.key}:${step.state}`)

test("pendant l'appel, les dépôts sont en cours et le reste à venir", () => {
  assert.deepEqual(states(editionMilestones(edition, call, Date.parse('2026-09-01T00:00:00Z'))), [
    'callOpens:done',
    'callCloses:current',
    'results:upcoming',
    'event:upcoming',
  ])
})

test("un programme publié vaut annonce des résultats, même sans date d'annonce", () => {
  const steps = editionMilestones(
    { ...edition, programme_published_at: '2026-11-02T02:14:29Z' },
    { ...call, status: 'under_review' },
    Date.parse('2026-11-02T12:00:00Z'),
  )
  assert.deepEqual(states(steps), ['callOpens:done', 'callCloses:done', 'results:done', 'event:upcoming'])
  assert.equal(steps[2]?.at, null)
})

test("l'événement est en cours entre ses dates, et coche tout ce qui le précède", () => {
  const steps = editionMilestones(
    edition,
    { ...call, closes_at: '2026-11-25T00:00:00Z' },
    Date.parse('2026-11-10T00:00:00Z'),
  )
  assert.deepEqual(states(steps), ['callOpens:done', 'callCloses:done', 'results:done', 'event:current'])
})

test('une édition terminée est entièrement franchie (cas de la CdP30)', () => {
  const steps = editionMilestones(
    { starts_at: '2025-11-10T03:00:00Z', ends_at: '2025-11-22T02:59:00Z', programme_published_at: '2025-11-20T13:05:57Z' },
    { ...call, status: 'published', opens_at: '2025-09-15T16:44:16Z', closes_at: '2025-11-21T23:59:00Z' },
    Date.parse('2026-11-02T00:00:00Z'),
  )
  assert.ok(steps.every((step) => step.state === 'done'))
})

test("après la clôture, l'examen est en cours tant que rien n'est annoncé", () => {
  const steps = editionMilestones(edition, { ...call, status: 'under_review' }, Date.parse('2026-10-28T00:00:00Z'))
  assert.deepEqual(states(steps), ['callOpens:done', 'callCloses:done', 'results:current', 'event:upcoming'])
})

test("un appel annulé n'a ni clôture ni résultats", () => {
  const steps = editionMilestones(edition, { ...call, status: 'cancelled' }, Date.parse('2026-11-10T00:00:00Z'))
  assert.deepEqual(states(steps), ['callOpens:done', 'callCloses:skipped', 'results:skipped', 'event:current'])
})

test("sans appel, seule la tenue de l'événement figure", () => {
  assert.deepEqual(states(editionMilestones(edition, null, Date.parse('2026-01-01T00:00:00Z'))), ['event:upcoming'])
})
