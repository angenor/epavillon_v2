import { test } from 'node:test'
import assert from 'node:assert/strict'
import {
  bornerLEchelle,
  creerLeLecteurDeGestes,
  echelleApresDoubleToucher,
  type Geste,
} from '../../app/utils/guide-nego/pdf/gestes.ts'
import { fausseHorloge } from './fausse-horloge.ts'

function lecteur() {
  const horloge = fausseHorloge()
  const gestes: Geste[] = []
  const l = creerLeLecteurDeGestes({ minuterie: horloge.minuterie, surGeste: (g) => gestes.push(g) })
  const toucher = (x: number, y: number) => l.toucher({ x, y, t: horloge.maintenant() })
  return { horloge, gestes, l, toucher }
}

test('un toucher simple ne part qu’après 300 ms sans second toucher', () => {
  const { horloge, gestes, toucher } = lecteur()
  toucher(100, 200)
  horloge.avancer(299)
  assert.deepEqual(gestes, [])
  horloge.avancer(1)
  assert.deepEqual(gestes, [{ type: 'simple', x: 100, y: 200 }])
})

test('un double toucher part aussitôt, au point du second, et rien d’autre', () => {
  const { horloge, gestes, toucher } = lecteur()
  toucher(100, 200)
  horloge.avancer(150)
  toucher(110, 210)
  assert.deepEqual(gestes, [{ type: 'double', x: 110, y: 210 }])
  horloge.avancer(1000)
  assert.equal(gestes.length, 1)
  assert.equal(horloge.enCours(), 0)
})

test('deux touchers éloignés : le premier part en simple, le second attend son tour', () => {
  const { horloge, gestes, toucher } = lecteur()
  toucher(100, 200)
  horloge.avancer(100)
  toucher(100, 230)
  assert.deepEqual(gestes, [{ type: 'simple', x: 100, y: 200 }])
  horloge.avancer(300)
  assert.deepEqual(gestes, [{ type: 'simple', x: 100, y: 200 }, { type: 'simple', x: 100, y: 230 }])
})

test('un second toucher trop tard, même au même point, n’est pas un double', () => {
  const { horloge, gestes, l } = lecteur()
  l.toucher({ x: 10, y: 10, t: 0 })
  l.toucher({ x: 10, y: 10, t: 300 })
  assert.deepEqual(gestes, [{ type: 'simple', x: 10, y: 10 }])
  horloge.avancer(300)
  assert.equal(gestes.length, 2)
})

test('un toucher suivi d’un pincement n’émet aucun geste', () => {
  const { horloge, gestes, l, toucher } = lecteur()
  toucher(100, 200)
  horloge.avancer(100)
  l.annulerEnCours()
  horloge.avancer(1000)
  assert.deepEqual(gestes, [])
})

test('arrêter annule le toucher en attente', () => {
  const { horloge, gestes, l, toucher } = lecteur()
  toucher(1, 1)
  l.arreter()
  horloge.avancer(1000)
  assert.deepEqual(gestes, [])
  assert.equal(horloge.enCours(), 0)
})

test('le double toucher double l’échelle jusqu’au plafond, puis revient à la largeur', () => {
  assert.equal(echelleApresDoubleToucher(1, 1), 2)
  assert.equal(echelleApresDoubleToucher(1.005, 1), 2.01, 'à 1 % près, c’est encore la largeur')
  assert.equal(echelleApresDoubleToucher(2, 1), 1)
  assert.equal(echelleApresDoubleToucher(0.8, 0.5), 0.5)
  assert.equal(echelleApresDoubleToucher(0.5, 0.5), 1)
  assert.equal(echelleApresDoubleToucher(1.5, 1.5, 1.5), 2.25)
  assert.equal(echelleApresDoubleToucher(3, 3, 1.5), 4.5, 'le plafond borne le doublement')
})

test('l’échelle reste entre la largeur et quatre fois la largeur', () => {
  assert.equal(bornerLEchelle(0.3, 0.5), 0.5)
  assert.equal(bornerLEchelle(1.2, 0.5), 1.2)
  assert.equal(bornerLEchelle(3, 0.5), 2)
  assert.equal(bornerLEchelle(3, 0.5, 5), 2.5)
})
