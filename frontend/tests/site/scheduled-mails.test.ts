import { test } from 'node:test'
import assert from 'node:assert/strict'
import type { ScheduledMailBatchRow } from '../../app/types/admin-scheduled-mails.ts'
import {
  clockOffsetMs,
  countdownParts,
  groupScheduledBatches,
  offsetUnit,
  scheduledTypeKey,
  shortDayTime,
} from '../../app/utils/scheduled-mails.ts'

test("l'écart d'horloge se mesure sur l'heure du serveur", () => {
  const recu = Date.parse('2026-11-04T08:00:00Z')
  assert.equal(clockOffsetMs('2026-11-04T08:00:05Z', recu), 5000)
  assert.equal(clockOffsetMs('2026-11-04T07:59:58Z', recu), -2000)
  assert.equal(clockOffsetMs('pas une date', recu), 0)
})

test('le décompte se lit en jours, heures, minutes et secondes', () => {
  const maintenant = Date.parse('2026-11-03T18:01:48Z')
  const parts = countdownParts('2026-11-06T06:00:00Z', maintenant)
  assert.deepEqual(parts, { days: 2, hours: 11, minutes: 58, seconds: 12, total: 215_892 })
})

test('un départ passé se décompte à zéro, jamais en négatif', () => {
  const parts = countdownParts('2026-11-03T10:00:00Z', Date.parse('2026-11-03T10:00:30Z'))
  assert.deepEqual(parts, { days: 0, hours: 0, minutes: 0, seconds: 0, total: 0 })
})

test("l'écart corrige un poste qui retarde", () => {
  const recuLocal = Date.parse('2026-11-04T07:58:00Z')
  const ecart = clockOffsetMs('2026-11-04T08:00:00Z', recuLocal)
  const parts = countdownParts('2026-11-04T08:01:00Z', recuLocal + ecart)
  assert.equal(parts.total, 60)
})

test('un décalage se dit dans son unité', () => {
  assert.deepEqual(offsetUnit(2880), { unit: 'day', count: 2 })
  assert.deepEqual(offsetUnit(60), { unit: 'hour', count: 1 })
  assert.deepEqual(offsetUnit(30), { unit: 'minute', count: 30 })
  assert.deepEqual(offsetUnit(90), { unit: 'minute', count: 90 })
})

function ligne(
  kind: 'day' | 'activity',
  offset: number,
  runAt: string,
  recipients: number,
  title = 'Atelier',
): ScheduledMailBatchRow {
  return {
    key: `${kind}.${title}.${offset}.${runAt}`,
    type_key: scheduledTypeKey(kind, offset),
    kind,
    offset_minutes: offset,
    target_id: '0198c1a0-0000-7097-8000-000000000001',
    target_starts_at: null,
    title,
    run_at: runAt,
    recipients,
    has_own_text: false,
  }
}

test('les envois se regroupent par type : la journée, puis les activités du plus lointain au plus proche', () => {
  const types = groupScheduledBatches(
    [
      ligne('activity', 60, '2026-11-09T09:00:00Z', 3),
      ligne('day', 1440, '2026-11-07T06:00:00Z', 4),
      ligne('activity', 2880, '2026-11-07T08:00:00Z', 2),
      ligne('day', 2880, '2026-11-06T06:00:00Z', 4),
      ligne('activity', 60, '2026-11-08T09:00:00Z', 1, 'Océans'),
    ],
    new Set(['activity.60']),
  )
  assert.deepEqual(
    types.map((t) => t.key),
    ['day', 'activity.2880', 'activity.60'],
  )
  const [jour, , heure] = types
  assert.equal(jour?.count, 8)
  assert.equal(jour?.offset_minutes, null)
  assert.equal(jour?.next_run_at, '2026-11-06T06:00:00Z')
  assert.equal(heure?.has_custom_text, true)
  assert.deepEqual(
    heure?.batches.map((b) => b.title),
    ['Océans', 'Atelier'],
  )
})

test('le départ court se lit dans le fuseau de l’édition', () => {
  assert.equal(shortDayTime('2026-11-06T06:00:00Z', 'Europe/Istanbul', 'fr-FR'), '06/11 09:00')
  assert.equal(shortDayTime('pas une date', 'Europe/Istanbul', 'fr-FR'), '')
})
