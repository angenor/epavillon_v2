import { test } from 'node:test'
import assert from 'node:assert/strict'
import { creerLaSurveillance, type CauseDeBascule } from '../../app/utils/guide-nego/pdf/bascule.ts'
import {
  CLE_BASCULES,
  compterUneBascule,
  lireLesBascules,
} from '../../app/utils/guide-nego/appareil-lecture.ts'
import { fauxStockage } from './faux-depots.ts'
import { fausseHorloge } from './fausse-horloge.ts'

function surveillance() {
  const horloge = fausseHorloge()
  const bascules: Array<{ cause: CauseDeBascule; a: number }> = []
  const s = creerLaSurveillance({
    minuterie: horloge.minuterie,
    maintenant: horloge.maintenant,
    surBascule: (cause) => bascules.push({ cause, a: horloge.maintenant() }),
  })
  return { horloge, bascules, s }
}

test('une erreur de pdf.js bascule aussitôt, une seule fois', () => {
  const { horloge, bascules, s } = surveillance()
  s.demarrer()
  horloge.avancer(1000)
  s.erreur()
  s.erreur()
  horloge.avancer(60_000)
  assert.deepEqual(bascules, [{ cause: 'erreur', a: 1000 }])
})

test('une copie gardée : 8 s sans première page, et le lecteur bascule', () => {
  const { horloge, bascules, s } = surveillance()
  s.demarrer()
  horloge.avancer(7999)
  assert.deepEqual(bascules, [])
  horloge.avancer(1)
  assert.deepEqual(bascules, [{ cause: 'delai', a: 8000 }])
  s.erreur()
  horloge.avancer(60_000)
  assert.equal(bascules.length, 1)
})

test('la première page dessinée à temps : plus aucune bascule', () => {
  const { horloge, bascules, s } = surveillance()
  s.demarrer()
  horloge.avancer(7000)
  s.rendue()
  horloge.avancer(60_000)
  s.erreur()
  assert.deepEqual(bascules, [])
  assert.equal(horloge.enCours(), 0)
})

test('30 s d’attente du réseau puis la page en 2 s : pas de bascule', () => {
  const { horloge, bascules, s } = surveillance()
  s.demarrer()
  s.reseau(true)
  horloge.avancer(30_000)
  s.reseau(false)
  horloge.avancer(2000)
  s.rendue()
  horloge.avancer(60_000)
  assert.deepEqual(bascules, [])
})

test('une plage qui part suspend le délai, qui reprend où il en était', () => {
  const { horloge, bascules, s } = surveillance()
  s.demarrer()
  horloge.avancer(5000)
  s.reseau(true)
  s.reseau(true)
  horloge.avancer(15_000)
  s.reseau(false)
  s.reseau(false)
  horloge.avancer(2999)
  assert.deepEqual(bascules, [])
  horloge.avancer(1)
  assert.deepEqual(bascules, [{ cause: 'delai', a: 23_000 }])
})

test('une plage en route avant le départ : le délai n’attend que le réseau', () => {
  const { horloge, bascules, s } = surveillance()
  s.reseau(true)
  s.demarrer()
  horloge.avancer(20_000)
  assert.deepEqual(bascules, [])
  s.reseau(false)
  horloge.avancer(8000)
  assert.deepEqual(bascules, [{ cause: 'delai', a: 28_000 }])
})

test('arrêter la surveillance n’émet rien', () => {
  const { horloge, bascules, s } = surveillance()
  s.demarrer()
  s.arreter()
  s.erreur()
  horloge.avancer(60_000)
  assert.deepEqual(bascules, [])
})

test('rien n’est lu du stockage pour décider : un téléphone déjà basculé réessaie', () => {
  const stockage = fauxStockage({ [CLE_BASCULES]: JSON.stringify({ nombre: 9, cause: 'erreur', a: '2026-11-11T08:00:00Z' }) })
  const { horloge, bascules, s } = surveillance()
  s.demarrer()
  horloge.avancer(1000)
  s.rendue()
  horloge.avancer(60_000)
  assert.deepEqual(bascules, [])
  assert.equal(lireLesBascules(stockage)?.nombre, 9)
})

test('le compteur de bascules augmente et garde la dernière cause', () => {
  const stockage = fauxStockage()
  assert.equal(lireLesBascules(stockage), null)
  assert.deepEqual(compterUneBascule(stockage, 'delai', '2026-11-12T08:00:00Z'), { nombre: 1, cause: 'delai', a: '2026-11-12T08:00:00Z' })
  compterUneBascule(stockage, 'erreur', '2026-11-12T09:00:00Z')
  assert.deepEqual(lireLesBascules(stockage), { nombre: 2, cause: 'erreur', a: '2026-11-12T09:00:00Z' })
})

test('un compte illisible se lit absent, et le suivant repart de 1', () => {
  for (const brut of ['pas du json', 'null', '[]', '{"nombre":0,"cause":"delai","a":"x"}', '{"nombre":2,"cause":"panne","a":"x"}', '{"nombre":2,"cause":"delai"}']) {
    const stockage = fauxStockage({ [CLE_BASCULES]: brut })
    assert.equal(lireLesBascules(stockage), null, brut)
    assert.equal(compterUneBascule(stockage, 'erreur', '2026-11-12T08:00:00Z').nombre, 1, brut)
  }
})
