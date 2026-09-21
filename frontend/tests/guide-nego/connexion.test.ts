import { test } from 'node:test'
import assert from 'node:assert/strict'
import {
  apresEchec,
  apresLectureGardee,
  apresReussite,
  connexionInitiale,
  momentDeLecture,
} from '../../app/utils/guide-nego/connexion.ts'

const le = (texte: string) => new Date(texte)

test('le bandeau se remontre à chaque nouvel épisode hors connexion, pas pendant', () => {
  let etat = connexionInitiale(true)
  etat = apresEchec(etat)
  assert.deepEqual([etat.enLigne, etat.bandeauVu], [false, false])

  etat = apresEchec({ ...etat, bandeauVu: true })
  assert.equal(etat.bandeauVu, true, 'un second échec du même épisode ne remontre pas le bandeau')

  etat = apresEchec(apresReussite(etat, '2026-11-12T10:00:00.000Z'))
  assert.equal(etat.bandeauVu, false, 'un nouvel épisode le remontre')
})

test('« lu à » garde la lecture la plus récente', () => {
  let etat = apresReussite(connexionInitiale(true), '2026-11-12T10:00:00.000Z')
  etat = apresLectureGardee(etat, '2026-11-11T08:00:00.000Z')
  assert.equal(etat.luA, '2026-11-12T10:00:00.000Z')
  assert.equal(connexionInitiale(false).luA, null, 'rien de lu : aucune heure inventée')
})

test('le jour même : l’heure seule', () => {
  assert.deepEqual(momentDeLecture('2026-11-12T14:05:00', le('2026-11-12T23:59:00')), {
    quand: 'aujourdhui',
    heure: '14:05',
  })
})

test('de part et d’autre de minuit : hier', () => {
  assert.deepEqual(momentDeLecture('2026-11-11T23:10:00', le('2026-11-12T00:05:00')), {
    quand: 'hier',
    heure: '23:10',
  })
  assert.equal(momentDeLecture('2026-11-11T00:01:00', le('2026-11-12T23:59:00')).quand, 'hier')
})

test('l’avant-veille et au-delà : le jour', () => {
  assert.deepEqual(momentDeLecture('2026-11-10T23:10:00', le('2026-11-12T08:00:00')), {
    quand: 'avant',
    heure: '23:10',
    jour: '10 nov.',
  })
})

test('d’une année sur l’autre : l’année s’écrit', () => {
  const moment = momentDeLecture('2026-12-30T09:00:00', le('2027-01-02T09:00:00'))
  assert.equal(moment.quand, 'avant')
  assert.match(moment.quand === 'avant' ? moment.jour : '', /2026/)
})

test('une horloge reculée ne donne pas un jour négatif', () => {
  assert.equal(momentDeLecture('2026-11-13T09:00:00', le('2026-11-12T09:00:00')).quand, 'aujourdhui')
})
