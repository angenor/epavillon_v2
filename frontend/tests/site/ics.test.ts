import { test } from 'node:test'
import assert from 'node:assert/strict'
import { icsCalendar } from '../../app/utils/ics.ts'

const fichier = icsCalendar(
  {
    uid: 'seance-1@epavillon',
    title: 'Taxonomie verte, marchés; capitaux',
    startsAt: '2026-11-11T10:30:00Z',
    endsAt: '2026-11-11T12:00:00Z',
    location: 'Pavillon de la Francophonie, Antalya',
  },
  Date.parse('2026-09-30T08:00:00Z'),
)

test('les heures partent en UTC', () => {
  assert.match(fichier, /\r\nDTSTART:20261111T103000Z\r\n/)
  assert.match(fichier, /\r\nDTEND:20261111T120000Z\r\n/)
})

test('virgules et points-virgules sont échappés', () => {
  assert.match(fichier, /SUMMARY:Taxonomie verte\\, marchés\\; capitaux/)
})

test('aucune ligne ne dépasse 75 octets', () => {
  const long = icsCalendar({ uid: 'x', title: 'é'.repeat(80), startsAt: '2026-11-11T10:30:00Z', endsAt: '2026-11-11T12:00:00Z' })
  for (const line of long.split('\r\n')) assert.ok(new TextEncoder().encode(line).length <= 75)
})
