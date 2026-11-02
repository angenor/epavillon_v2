import { test } from 'node:test'
import assert from 'node:assert/strict'
import { dayPickGroups, dayTrackOf, keptIds, pickableIds } from '../../app/utils/day-registration.ts'
import type { PublicTrackEntry, PublicTrackPage } from '../../app/types/programme/track-page.ts'
import type { ScheduleTrackBadge } from '../../app/types/views.ts'

const at = (h: string) => `2026-11-13T${h}:00+03:00`
const activite = (id: string, h: string, part: string | null, status = 'scheduled') =>
  ({
    kind: 'activity',
    starts_at: at(h),
    ends_at: at(h),
    part_id: part,
    activity: { session: { id, status, title: { fr: id } }, organizations: [], speakers: [], is_highlight: false },
  }) as unknown as PublicTrackEntry
const moment = (label: string, h: string, part: string | null) =>
  ({ kind: 'moment', starts_at: at(h), ends_at: null, part_id: part, moment: { id: label, label: { fr: label }, description: null } }) as PublicTrackEntry

const page = {
  parts: [
    { id: 'p1', number: 1 },
    { id: 'p2', number: 2 },
    { id: 'p3', number: 3 },
  ],
  agenda: [
    moment('Accueil', '08:00', null),
    activite('a', '09:00', 'p1'),
    activite('b', '10:15', 'p2'),
    moment('Déjeuner', '12:30', 'p2'),
    activite('c', '13:00', 'p2', 'cancelled'),
    moment('Clôture', '18:00', 'p3'),
  ],
} as unknown as PublicTrackPage

test('les activités annulées ne se cochent pas ; les moments restent, sans case', () => {
  assert.deepEqual(pickableIds(page), ['a', 'b'])
  const groups = dayPickGroups(page)
  assert.deepEqual(
    groups.map((g) => [g.part?.id ?? null, g.entries.map((e) => (e.kind === 'activity' ? e.activity.session.id : e.entry.moment.id))]),
    [
      [null, ['Accueil']],
      ['p1', ['a']],
      ['p2', ['b', 'Déjeuner']],
    ],
    'une partie sans activité à cocher (la clôture seule) ne fait pas de groupe',
  )
})

test('les activités gardées suivent l’ordre du déroulé et oublient les annulées', () => {
  assert.deepEqual(keptIds(['c', 'b', 'a', 'inconnue'], page), ['a', 'b'])
})

test('une activité relève de sa journée spéciale avant l’atelier, et d’aucune hors fil de journée', () => {
  const badge = (slug: string, kind: ScheduleTrackBadge['kind']) => ({ slug, kind }) as ScheduleTrackBadge
  assert.equal(dayTrackOf([badge('atelier', 'side_programme'), badge('finance', 'special_day')])?.slug, 'finance')
  assert.equal(dayTrackOf([badge('atelier', 'side_programme')])?.slug, 'atelier')
  assert.equal(dayTrackOf([badge('climat', 'thematic_track')]), null)
})
