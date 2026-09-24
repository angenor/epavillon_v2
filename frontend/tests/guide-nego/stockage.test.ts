import { test } from 'node:test'
import assert from 'node:assert/strict'
import { CLE_OUVERTURE_VUE, lireCle, poserCle } from '../../app/utils/guide-nego/stockage.ts'
import { magasinDesEcritures } from '../../app/utils/guide-nego/garde.ts'
import {
  annoncerLeTexteAgrandi,
  CLE_MODE,
  CLE_MODE_ANNONCE,
  lireMode,
  modeDuDocument,
  noterLAnnonceVue,
  poserMode,
} from '../../app/utils/guide-nego/appareil-lecture.ts'

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

// --- Le mode de lecture (FR-020 bis, FR-021) --------------------------------------

function memoire() {
  const cles = new Map<string, string>()
  return { lire: (cle: string) => cles.get(cle) ?? null, poser: (cle: string, valeur: string) => void cles.set(cle, valeur), cles }
}

test('le mode de lecture : « Pages » par défaut, puis celui qu’on a choisi', () => {
  const stockage = memoire()
  assert.equal(lireMode(stockage), 'pages')
  poserMode(stockage, 'texte')
  assert.equal(lireMode(stockage), 'texte')
  assert.equal(stockage.cles.get(CLE_MODE), 'texte')
  stockage.poser(CLE_MODE, 'autre chose')
  assert.equal(lireMode(stockage), 'pages', 'une valeur inconnue vaut l’absence')
})

test('un document sans « Texte agrandi » s’ouvre en pages, le dit, et ne change pas le mode gardé', () => {
  const stockage = memoire()
  poserMode(stockage, 'texte')
  assert.deepEqual(modeDuDocument(lireMode(stockage), false), { mode: 'pages', limite: true })
  assert.equal(lireMode(stockage), 'texte', 'le choix de la personne reste')
  assert.deepEqual(modeDuDocument('texte', true), { mode: 'texte', limite: false })
  assert.deepEqual(modeDuDocument('pages', false), { mode: 'pages', limite: false }, 'rien à dire à qui lit déjà en pages')
})

test('l’annonce de « Texte agrandi » : une fois par téléphone, sur un écran étroit, là où il est offert', () => {
  const stockage = memoire()
  const etroit = { largeur: 360, hauteur: 780 }
  assert.equal(annoncerLeTexteAgrandi(stockage, false, etroit), false, 'pas sur un document qui ne l’offre pas')
  assert.equal(annoncerLeTexteAgrandi(stockage, true, { largeur: 1024, hauteur: 768 }), false, 'pas sur un grand écran')
  assert.equal(annoncerLeTexteAgrandi(stockage, true, { largeur: 780, hauteur: 360 }), false, 'pas en paysage')
  assert.equal(annoncerLeTexteAgrandi(stockage, true, etroit), true)
  noterLAnnonceVue(stockage)
  assert.equal(stockage.cles.get(CLE_MODE_ANNONCE), '1')
  assert.equal(annoncerLeTexteAgrandi(stockage, true, etroit), false, 'une seule fois')
})
