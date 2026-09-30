import { test } from 'node:test'
import assert from 'node:assert/strict'
import { addDays, hourRange, layoutDay, spanOf, weekStart, type SessionSpan } from '../../app/utils/programme-week.ts'
import type { PublicScheduleRow } from '../../app/types/views.ts'

const seance = (starts_at: string, ends_at: string) => ({ starts_at, ends_at }) as PublicScheduleRow
const plage = (start: number, end: number): SessionSpan => ({ session: seance('', ''), start, end })

test('la semaine commence le lundi', () => {
  assert.equal(weekStart('2026-11-11'), '2026-11-09')
  assert.equal(weekStart('2026-11-09'), '2026-11-09')
  assert.equal(weekStart('2026-11-15'), '2026-11-09')
  assert.equal(addDays('2026-11-30', 1), '2026-12-01')
})

test('les heures murales suivent le fuseau de l’édition', () => {
  const span = spanOf(seance('2026-11-11T10:30:00Z', '2026-11-11T12:00:00Z'), 'Europe/Istanbul')
  assert.deepEqual([span.start, span.end], [13 * 60 + 30, 15 * 60])
})

test('une séance qui passe minuit s’arrête à minuit dans sa colonne', () => {
  const span = spanOf(seance('2026-11-11T19:00:00Z', '2026-11-11T22:30:00Z'), 'Europe/Istanbul')
  assert.equal(span.end, 24 * 60)
})

test('des séances qui se chevauchent sont mises côte à côte, jamais cachées', () => {
  const placed = layoutDay([plage(540, 600), plage(570, 660), plage(600, 630), plage(700, 760)])
  assert.deepEqual(
    placed.map((item) => [item.start, item.lane, item.lanes]),
    [[540, 0, 2], [570, 1, 2], [600, 0, 2], [700, 0, 1]],
  )
})

test('la grille se cale sur les heures pleines du programme', () => {
  assert.deepEqual(hourRange([plage(9 * 60 + 30, 11 * 60), plage(17 * 60, 18 * 60 + 45)]), [9, 19])
  assert.deepEqual(hourRange([]), [9, 18])
})
