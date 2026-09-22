import { test } from 'node:test'
import assert from 'node:assert/strict'
import { CLE_OUVERTURE_VUE, lireCle, poserCle } from '../../app/utils/guide-nego/stockage.ts'
import { magasinDesEcritures } from '../../app/utils/guide-nego/garde.ts'

/** Navigation privée : `localStorage` lève. L'ouverture vue doit tenir le temps de la visite. */
test('stockage refusé, une clé posée se relit pendant la visite', () => {
  Object.defineProperty(globalThis, 'localStorage', {
    get: () => {
      throw new DOMException('Stockage refusé', 'SecurityError')
    },
    configurable: true,
  })
  try {
    assert.equal(lireCle(CLE_OUVERTURE_VUE), null)
    poserCle(CLE_OUVERTURE_VUE)
    assert.equal(lireCle(CLE_OUVERTURE_VUE), '1')
  } finally {
    delete (globalThis as { localStorage?: unknown }).localStorage
  }
})

/** IndexedDB refusé : un choix fait en ligne ne se perd pas en silence, il attend en mémoire. */
test('IndexedDB refusé, la file garde ses intentions le temps de la visite', async () => {
  const intention = {
    cle: 'mes-thematiques',
    corps: { codes: ['gender'] },
    empreinte: null,
    personne: '01a0cab7-7b51-73a9-a811-962e78664dba',
    prise_a: '2026-09-22T20:30:00.000Z',
  }
  await magasinDesEcritures.poser(intention)
  assert.deepEqual(await magasinDesEcritures.lire(), [intention])
  await magasinDesEcritures.retirer(intention.cle)
  assert.deepEqual(await magasinDesEcritures.lire(), [])
})
