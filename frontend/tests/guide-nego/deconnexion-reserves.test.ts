import { test } from 'node:test'
import assert from 'node:assert/strict'
import { CACHE_PUBLICS, CACHE_RESERVES, effacerLesReserves, garderUneCopie } from '../../app/utils/guide-nego/copies.ts'
import { deconnecterDansLOrdre, relireEtEffacer } from '../../app/utils/guide-nego/effacements.ts'
import { copieDe, fauxDepots } from './faux-depots.ts'

async function telephoneAvecDeuxCopies() {
  const outils = fauxDepots()
  const publique = copieDe('guide')
  const reservee = copieDe('resume', { reserve: true })
  await garderUneCopie(outils.depots, publique.copie, publique.entrees)
  await garderUneCopie(outils.depots, reservee.copie, reservee.entrees)
  await outils.depots.aTelecharger.poser({ id: 'note-reservee', reserve: true, octets: null, demande_a: '2026-11-12T09:00:00Z' })
  await outils.depots.aTelecharger.poser({ id: 'bulletin', reserve: false, octets: null, demande_a: '2026-11-12T09:00:00Z' })
  return { ...outils, publique }
}

test('la déconnexion efface les réservés, et garde les publics', async () => {
  const { depots, noms, clesDe, publique } = await telephoneAvecDeuxCopies()
  await effacerLesReserves(depots)
  assert.equal(noms.has(CACHE_RESERVES), false, 'la lecture et le PDF du réservé partent ensemble')
  assert.deepEqual(clesDe(CACHE_PUBLICS), [...publique.copie.cles].sort())
  assert.deepEqual((await depots.copies.lire()).map((c) => c.id), ['guide'])
  assert.deepEqual((await depots.aTelecharger.lire()).map((d) => d.id), ['bulletin'])
})

test('les réservés s’effacent AVANT la file, et la file avant la session', async () => {
  const ordre: string[] = []
  await deconnecterDansLOrdre({
    effacerLesReserves: async () => void ordre.push('réservés'),
    viderLaFile: async () => void ordre.push('file'),
    fermerLaSession: async () => void ordre.push('session'),
  })
  assert.deepEqual(ordre, ['réservés', 'file', 'session'])
})

test('un effacement qui échoue n’empêche pas de se déconnecter', async () => {
  const ordre: string[] = []
  await deconnecterDansLOrdre({
    effacerLesReserves: async () => {
      throw new Error('Cache Storage indisponible')
    },
    viderLaFile: async () => void ordre.push('file'),
    fermerLaSession: async () => void ordre.push('session'),
  })
  assert.deepEqual(ordre, ['file', 'session'])
})

test('la perte d’accès, lue sur une réponse, efface les réservés', async () => {
  const { depots } = await telephoneAvecDeuxCopies()
  const lu = await relireEtEffacer(async () => ({ ouvert: false }), (a) => !a.ouvert, () => effacerLesReserves(depots))
  assert.deepEqual(lu, { ouvert: false })
  assert.deepEqual((await depots.copies.lire()).map((c) => c.id), ['guide'])
})

test('un accès toujours ouvert n’efface rien', async () => {
  const { depots } = await telephoneAvecDeuxCopies()
  await relireEtEffacer(async () => ({ ouvert: true }), (a) => !a.ouvert, () => effacerLesReserves(depots))
  assert.equal((await depots.copies.lire()).length, 2)
})

test('une API muette n’efface rien : la lecture lève avant', async () => {
  const { depots } = await telephoneAvecDeuxCopies()
  let efface = false
  await assert.rejects(
    relireEtEffacer(
      async () => {
        throw new Error('API injoignable (network)')
      },
      () => true,
      async () => {
        efface = true
        await effacerLesReserves(depots)
      },
    ),
  )
  assert.equal(efface, false)
  assert.equal((await depots.copies.lire()).length, 2)
})

test('un effacement qui échoue ne fait pas tomber la lecture du compte', async () => {
  const lu = await relireEtEffacer(
    async () => ({ connectee: false }),
    (c) => !c.connectee,
    async () => {
      throw new Error('Cache Storage indisponible')
    },
  )
  assert.deepEqual(lu, { connectee: false })
})
