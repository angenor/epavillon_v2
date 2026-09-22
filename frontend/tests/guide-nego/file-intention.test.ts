import { test } from 'node:test'
import assert from 'node:assert/strict'
import {
  creerFile,
  fusionnerLesIntentions,
  magasinEnMemoire,
  type Intention,
} from '../../app/utils/guide-nego/file.ts'

const intention = (partiel: Partial<Intention>): Intention => ({
  cle: 'mes-thematiques',
  corps: { codes: ['adaptation'] },
  empreinte: '"a1"',
  personne: 'awa',
  prise_a: '2026-11-12T10:00:00.000Z',
  ...partiel,
})

test('deux intentions sur la même clé : une seule entrée, et l’empreinte de la première', async () => {
  const magasin = magasinEnMemoire()
  const file = creerFile({ magasin, expediteurs: new Map(), personne: () => 'awa' })

  await file.poser(intention({ corps: { codes: ['adaptation'] }, empreinte: '"a1"' }))
  await file.poser(
    intention({ corps: { codes: ['adaptation', 'gender'] }, empreinte: '"b2"', prise_a: '2026-11-12T10:05:00.000Z' }),
  )

  const entrees = await magasin.lire()
  assert.equal(entrees.length, 1)
  assert.deepEqual(entrees[0]?.corps, { codes: ['adaptation', 'gender'] }, 'le dernier état voulu')
  assert.equal(entrees[0]?.empreinte, '"a1"', 'l’état qu’a vu la personne avant de changer d’avis')
  assert.equal(entrees[0]?.prise_a, '2026-11-12T10:00:00.000Z')
})

test('une autre personne sur la même clé ne reprend pas l’empreinte de la première', () => {
  const fusion = fusionnerLesIntentions(
    intention({ personne: 'awa', empreinte: '"a1"' }),
    intention({ personne: 'fatou', empreinte: '"f9"' }),
  )
  assert.equal(fusion.empreinte, '"f9"')
  assert.equal(fusion.personne, 'fatou')
})

test('un magasin qui lève ne fait pas lever la file', async () => {
  const casse = magasinEnMemoire()
  casse.poser = async () => {
    throw new Error('stockage refusé')
  }
  casse.lire = async () => {
    throw new Error('stockage refusé')
  }
  const file = creerFile({ magasin: casse, expediteurs: new Map(), personne: () => 'awa' })
  await assert.doesNotReject(file.poser(intention({})))
  assert.deepEqual(await file.partir(), [])
  await assert.doesNotReject(file.vider())
})
