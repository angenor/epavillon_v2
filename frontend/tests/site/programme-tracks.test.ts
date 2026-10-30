import { test } from 'node:test'
import assert from 'node:assert/strict'
import { composeFrame, indexTracks, specialOf } from '../../app/utils/programme-tracks.ts'
import type { PublicScheduleTrack } from '../../app/types/programme/track-page.ts'
import type { PublicScheduleRow } from '../../app/types/views.ts'

const at = (h: string) => `2026-11-13T${h}:00+03:00`
const zone = 'Europe/Istanbul'
const moment = (h: string, label: string) => ({ starts_at: at(h), ends_at: null, label: { fr: label } })
const track = {
  slug: 'journee-finance',
  kind: 'special_day',
  title: { fr: 'Journée finance' },
  moments: [moment('08:00', 'Accueil'), moment('09:00', 'Ouverture'), moment('12:30', 'Déjeuner'), moment('18:00', 'Clôture')],
} as PublicScheduleTrack
const seance = (id: string, debut: string, fin: string) =>
  ({ id, starts_at: at(debut), ends_at: at(fin), tracks: [{ slug: 'journee-finance', kind: 'special_day' }] }) as PublicScheduleRow
const seances = [seance('a', '09:15', '10:00'), seance('b', '10:15', '11:00'), seance('c', '14:00', '15:00')]

test('les moments consécutifs forment une ligne, à leur place dans l’heure', () => {
  const frame = composeFrame(track, seances, new Set(['a', 'b', 'c']), '2026-11-13', zone, false, (s) => s.id)
  assert.ok(frame)
  const lignes = frame.items.map((item) => (item.kind === 'activity' ? item.row : item.moments.map((m) => m.label.fr).join('+')))
  assert.deepEqual(lignes, ['Accueil+Ouverture', 'a', 'b', 'Déjeuner', 'c', 'Clôture'])
  assert.equal(frame.start, at('08:00'), 'la journée part du premier moment')
  assert.equal(frame.end, at('18:00'))
})

test('en recherche, seules les activités retenues, sans moments ; aucune : pas de cadre', () => {
  const frame = composeFrame(track, seances, new Set(['b']), '2026-11-13', zone, true, (s) => s.id)
  assert.deepEqual(frame?.items.map((item) => item.kind), ['activity'])
  assert.equal(composeFrame(track, seances, new Set(), '2026-11-13', zone, true, (s) => s.id), null)
})

test('un fil non publié ne range rien', () => {
  assert.equal(specialOf(seances[0]!, indexTracks([])), null)
  assert.equal(specialOf(seances[0]!, indexTracks([track]))?.slug, 'journee-finance')
})
