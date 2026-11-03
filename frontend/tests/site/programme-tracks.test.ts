import { test } from 'node:test'
import assert from 'node:assert/strict'
import { composeDay, indexTracks, trackOf } from '../../app/utils/programme-tracks.ts'
import type { PublicScheduleTrack } from '../../app/types/programme/track-page.ts'
import type { PublicScheduleRow } from '../../app/types/views.ts'

const at = (h: string) => `2026-11-13T${h}:00+03:00`
const day = '2026-11-13'
const zone = 'Europe/Istanbul'
const moment = (h: string, label: string, extra: Partial<PublicScheduleTrack['moments'][number]> = {}) => ({
  starts_at: at(h),
  ends_at: null,
  label: { fr: label },
  is_highlight: false,
  session_ids: [],
  ...extra,
})
const track = {
  slug: 'journee-finance',
  kind: 'special_day',
  title: { fr: 'Journée finance' },
  starts_at: at('08:00'),
  starts_on: day,
  parts: [{ id: 'p1', number: 1, label: null, starts_at: at('08:00'), ends_at: at('10:00') }],
  moments: [
    moment('08:00', 'Accueil'),
    moment('09:20', 'Les 10 minutes', { session_ids: ['g1', 'g2'] }),
    moment('18:00', 'Clôture', { is_highlight: true }),
  ],
} as PublicScheduleTrack
const seance = (id: string, debut: string, fin: string, slug = 'journee-finance', title = id) =>
  ({
    id,
    title: { fr: title },
    status: 'scheduled',
    starts_at: at(debut),
    ends_at: at(fin),
    tracks: slug ? [{ slug, kind: 'special_day', is_highlight: id === 'b' }] : [],
  }) as PublicScheduleRow
const seances = [
  seance('o', '07:00', '07:45', ''),
  seance('a', '09:00', '09:20'),
  seance('g1', '09:20', '09:30'),
  seance('g2', '09:30', '09:40'),
  seance('b', '10:15', '11:00'),
  seance('z', '19:00', '20:00', '', 'Dialogue des bailleurs'),
]
const all = () => true

test('un bloc par journée, ses activités absorbées, les autres à leur heure', () => {
  const composed = composeDay(day, seances, indexTracks([track]), zone, all, all)
  assert.deepEqual(
    composed.items.map((item) => (item.kind === 'block' ? item.block.track.slug : item.session.id)),
    ['o', 'journee-finance', 'z'],
  )
  const block = composed.blocks[0]!
  assert.equal(block.activities, 2, 'les activités regroupées sous un moment ne comptent pas')
  assert.equal(block.moments, 3)
  assert.equal(block.start, at('08:00'))
  assert.equal(block.end, at('18:00'))
  assert.equal(block.highlight?.title.fr, 'b', 'le premier temps fort du jour')
  assert.equal(composed.total, 3)
})

test('en recherche, le bloc reste l’entrée d’une activité retrouvée ; sinon il se tait', () => {
  const index = indexTracks([track])
  const seek = (needle: string) => (s: PublicScheduleRow) => s.title.fr.includes(needle)
  const none = () => false
  const found = composeDay(day, seances, index, zone, seek('g2'), none)
  assert.deepEqual(found.items.map((item) => item.kind), ['block'])
  const elsewhere = composeDay(day, seances, index, zone, seek('bailleurs'), none)
  assert.deepEqual(elsewhere.items.map((item) => (item.kind === 'session' ? item.session.id : 'bloc')), ['z'])
})

test('un fil non publié ne range rien', () => {
  assert.equal(trackOf(seances[1]!, indexTracks([])), null)
  assert.equal(trackOf(seances[1]!, indexTracks([track]))?.slug, 'journee-finance')
})
