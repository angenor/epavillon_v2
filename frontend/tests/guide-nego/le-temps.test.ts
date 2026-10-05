import { test } from 'node:test'
import assert from 'node:assert/strict'
import {
  changementsDuJour,
  compteARebours,
  filDuJour,
  jourDeLaCop,
  momentDeLaCop,
  pisteJusquALaCop,
} from '../../app/utils/guide-nego/journee.ts'
import { BELEM, session } from './fausses-sessions.ts'

// Belém : UTC−3. 10:48 à Belém = 13:48Z.
const A_10H48 = new Date('2027-11-10T13:48:00Z')
const COP = { debut: '2027-11-09T12:00:00Z', fin: '2027-11-20T21:00:00Z' }

test('le compte à rebours : minutes sous l’heure, heures dans la journée, puis le jour', () => {
  assert.deepEqual(compteARebours('2027-11-10T14:30:00Z', A_10H48, BELEM), { unite: 'minutes', minutes: 42 })
  assert.deepEqual(compteARebours('2027-11-10T17:08:00Z', A_10H48, BELEM), { unite: 'heures', heures: 3, minutes: 20 })
  assert.deepEqual(compteARebours('2027-11-11T12:00:00Z', A_10H48, BELEM), { unite: 'demain' })
  assert.deepEqual(compteARebours(COP.debut, new Date('2027-10-05T15:00:00Z'), BELEM), { unite: 'jours', jours: 35 })
})

test('le jour se compte à Belém : 23:30 heure de Belém est encore aujourd’hui', () => {
  assert.deepEqual(compteARebours('2027-11-11T02:30:00Z', A_10H48, BELEM), { unite: 'heures', heures: 12, minutes: 42 })
})

test('avant la COP seulement si son ouverture est connue', () => {
  assert.equal(momentDeLaCop(COP, new Date('2027-10-05T15:00:00Z')), 'avant')
  assert.equal(momentDeLaCop(COP, A_10H48), 'pendant')
  assert.equal(momentDeLaCop({}, new Date('2027-10-05T15:00:00Z')), 'pendant')
  assert.equal(momentDeLaCop(null, A_10H48), 'pendant')
})

test('« jour 2 sur 12 », et rien hors de la COP', () => {
  assert.deepEqual(jourDeLaCop(COP, A_10H48, BELEM), { jour: 2, total: 12 })
  assert.equal(jourDeLaCop(COP, new Date('2027-10-05T15:00:00Z'), BELEM), null)
  assert.equal(jourDeLaCop({ debut: COP.debut }, A_10H48, BELEM), null)
})

test('le fil : 08 h à 20 h, maintenant placé, un créneau sans fin en point', () => {
  const fil = filDuJour(
    [[{ debut: '2027-11-10T13:00:00Z', fin: '2027-11-10T14:30:00Z', marque: true }], [{ debut: '2027-11-10T17:00:00Z', fin: null }], []],
    A_10H48,
    BELEM,
  )
  assert.equal(fil.debut, 8)
  assert.equal(fil.fin, 20)
  assert.deepEqual(fil.graduations, [8, 12, 16, 20])
  assert.deepEqual(fil.pistes[0], [{ gauche: 2 / 12, largeur: 1.5 / 12, marque: true }])
  assert.deepEqual(fil.pistes[1], [{ gauche: 6 / 12, largeur: null, marque: false }])
  assert.ok(Math.abs((fil.maintenant ?? 0) - 2.8 / 12) < 1e-9)
})

test('le fil s’élargit quand un créneau sort de la plage, graduations sur des heures', () => {
  const fil = filDuJour([[{ debut: '2027-11-10T10:00:00Z', fin: '2027-11-10T11:00:00Z' }]], A_10H48, BELEM)
  assert.equal(fil.debut, 7)
  assert.equal(fil.fin, 22)
  assert.deepEqual(fil.graduations, [7, 12, 17, 22])
})

test('changements du jour : annulée, salle changée, signalement validé par-dessus la source', () => {
  const sessions = [
    session('annulee', '2027-11-10T19:30:00Z', null, { status: 'cancelled' }),
    session('salle', '2027-11-10T18:00:00Z', '2027-11-10T19:00:00Z', {
      venue: 'Salle 7',
      previous: { start_at: '2027-11-10T18:00:00Z', end_at: null, venue: 'Salle 2', changed_at: '2027-11-10T09:00:00Z' },
    }),
    session('reseau', '2027-11-10T17:00:00Z', '2027-11-10T18:00:00Z', {
      venue: 'Salle 2',
      network_reports: [{ reason: 'venue', proposed_start: null, proposed_venue: 'Salle 9', detail: null, validated_at: '2027-11-10T12:00:00Z' }],
    }),
    session('terminee', '2027-11-10T11:00:00Z', '2027-11-10T12:00:00Z', {
      previous: { start_at: '2027-11-10T10:00:00Z', end_at: null, venue: null, changed_at: '2027-11-10T09:00:00Z' },
    }),
    session('demain', '2027-11-11T14:00:00Z', null, { status: 'cancelled' }),
    session('calme', '2027-11-10T16:00:00Z', null),
  ]
  const changements = changementsDuJour(sessions, A_10H48, BELEM)
  assert.deepEqual(
    changements.map((c) => [c.session.id, c.genre, c.origine, c.salle]),
    [
      ['reseau', 'salle', 'reseau', 'Salle 9'],
      ['salle', 'salle', 'source', 'Salle 7'],
      ['annulee', 'annulee', 'source', null],
    ],
  )
})

test('la piste jusqu’à la COP : l’ouverture placée, un jalon passé ou d’après l’ouverture écarté', () => {
  const piste = pisteJusquALaCop(new Date('2027-10-05T15:00:00Z'), COP.debut, COP.fin, [
    { cle: 'passe', date: '2027-10-01T12:00:00Z' },
    { cle: 'atelier', date: '2027-10-26T12:00:00Z' },
    { cle: 'pendant', date: '2027-11-12T12:00:00Z' },
  ])
  assert.ok(piste.ouverture > 0.7 && piste.ouverture < 0.8)
  assert.deepEqual(piste.jalons.map((j) => j.cle), ['atelier'])
})
