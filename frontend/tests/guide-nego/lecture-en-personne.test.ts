import { test } from 'node:test'
import assert from 'node:assert/strict'
import { lireEnPersonne } from '../../app/utils/guide-nego/effacements.ts'

const fermee = { accessible: false }
const ouverte = { accessible: true }

function lectures(...reponses: Array<{ accessible: boolean }>) {
  let n = 0
  return { lire: async () => reponses[Math.min(n++, reponses.length - 1)]!, lues: () => n }
}
const semble = (lu: { accessible: boolean }) => !lu.accessible

test('un jeton expiré : la rotation, puis la liste de la personne', async () => {
  const l = lectures(fermee, ouverte)
  assert.deepEqual(await lireEnPersonne(l.lire, semble, async () => 'renouvelee'), ouverte)
  assert.equal(l.lues(), 2)
})

test('une session vraiment finie : la liste anonyme fait foi', async () => {
  const l = lectures(fermee)
  assert.deepEqual(await lireEnPersonne(l.lire, semble, async () => 'finie'), fermee)
  assert.equal(l.lues(), 1)
})

test('une rotation sans réponse ne conclut rien : rien ne s’applique', async () => {
  const l = lectures(fermee)
  await assert.rejects(lireEnPersonne(l.lire, semble, async () => 'injoignable'))
})

test('une liste qui n’a pas l’air anonyme ne fait rien tourner', async () => {
  let tours = 0
  const l = lectures(ouverte)
  await lireEnPersonne(l.lire, semble, async () => {
    tours += 1
    return 'renouvelee'
  })
  assert.equal(tours, 0)
})
