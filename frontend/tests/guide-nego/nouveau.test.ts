import { test } from 'node:test'
import assert from 'node:assert/strict'
import {
  CLE_OUVERTS,
  CLE_PROGRESSION,
  CLE_TAILLE,
  estNouveau,
  lireProgression,
  lireRecents,
  lireTaille,
  noterOuverture,
  noterProgression,
  oublierLesVersionsDisparues,
} from '../../app/utils/guide-nego/appareil-lecture.ts'
import { fauxStockage } from './faux-depots.ts'

const MAINTENANT = new Date('2026-11-12T12:00:00Z')
const guide = { id: 'guide', published_at: '2026-11-09T08:00:00Z' }

test('« Nouveau » : moins de sept jours et jamais ouvert', () => {
  const stockage = fauxStockage()
  assert.equal(estNouveau(stockage, guide, MAINTENANT), true)
  assert.equal(estNouveau(stockage, { id: 'vieux', published_at: '2026-11-01T08:00:00Z' }, MAINTENANT), false)
})

test('ouvert une fois, il ne l’est plus — sur ce téléphone', () => {
  const stockage = fauxStockage()
  noterOuverture(stockage, 'guide', '2026-11-12T11:00:00Z')
  assert.equal(estNouveau(stockage, guide, MAINTENANT), false)
  assert.equal(estNouveau(fauxStockage(), guide, MAINTENANT), true, 'un autre téléphone le voit encore')
})

test('une date illisible n’est pas « Nouveau »', () => {
  assert.equal(estNouveau(fauxStockage(), { id: 'x', published_at: 'hier' }, MAINTENANT), false)
})

test('les derniers ouverts : le plus récent en tête, sans doublon, cinq au plus', () => {
  const stockage = fauxStockage()
  for (const [i, id] of ['a', 'b', 'c', 'd', 'e', 'f', 'b'].entries()) noterOuverture(stockage, id, `2026-11-12T0${i}:00:00Z`)
  assert.deepEqual(lireRecents(stockage).map((r) => r.id), ['b', 'f', 'e', 'd', 'c'])
})

test('la progression suit la version : une autre version s’ouvre au début', () => {
  const stockage = fauxStockage()
  noterProgression(stockage, 'guide', '2025', 59, '2026-11-12T10:00:00Z')
  assert.equal(lireProgression(stockage, 'guide', '2025')?.page, 59)
  assert.equal(lireProgression(stockage, 'guide', '2026'), null)
})

test('la progression d’une version qui n’est plus servie s’oublie à la relecture', () => {
  const stockage = fauxStockage()
  noterProgression(stockage, 'guide', '2025', 59, '2026-11-12T10:00:00Z')
  noterProgression(stockage, 'note', '1', 3, '2026-11-12T10:00:00Z')
  oublierLesVersionsDisparues(stockage, [{ id: 'guide', version: '2026' }, { id: 'note', version: '1' }])
  assert.equal(lireProgression(stockage, 'guide', '2025'), null)
  assert.equal(lireProgression(stockage, 'note', '1')?.page, 3)
})

test('la taille du texte : 17 par défaut, et rien d’autre que 17, 20 ou 24', () => {
  assert.equal(lireTaille(fauxStockage()), 17)
  assert.equal(lireTaille(fauxStockage({ [CLE_TAILLE]: '24' })), 24)
  assert.equal(lireTaille(fauxStockage({ [CLE_TAILLE]: '99' })), 17)
})

test('un stockage abîmé ne fait rien tomber', () => {
  const stockage = fauxStockage({ [CLE_OUVERTS]: '{pas du json', [CLE_PROGRESSION]: 'null' })
  assert.equal(estNouveau(stockage, guide, MAINTENANT), true)
  assert.equal(lireProgression(stockage, 'guide', '2025'), null)
})
