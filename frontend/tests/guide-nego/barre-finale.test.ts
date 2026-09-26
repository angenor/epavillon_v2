import { test } from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { avecBarreFinale, sansBarreFinale } from '../../app/utils/guide-nego/barre-finale.ts'

const manifeste = JSON.parse(readFileSync(new URL('../../public/guide-nego/manifest.webmanifest', import.meta.url), 'utf8'))
const PORTEE = new URL(manifeste.scope, 'https://epavillon.test/guide-nego/manifest.webmanifest').pathname

test('seul l’accueil sans barre est redirigé, à la racine comme sous /v2/', () => {
  assert.equal(sansBarreFinale('/guide-nego'), true)
  assert.equal(sansBarreFinale('/v2/guide-nego', '/v2/'), true)
  for (const chemin of ['/guide-nego/', '/guide-nego/lexique', '/guide-negociation', '/v2/guide-nego']) {
    assert.equal(sansBarreFinale(chemin), false, chemin)
  }
})

test('la redirection tombe dans la portée du service worker et garde la recherche', () => {
  assert.equal(PORTEE, '/guide-nego/')
  const cible = avecBarreFinale('/guide-nego', '?q=contact')
  assert.equal(cible, '/guide-nego/?q=contact')
  assert.ok(new URL(cible, 'https://epavillon.test').pathname.startsWith(PORTEE))
  assert.equal(sansBarreFinale(new URL(cible, 'https://epavillon.test').pathname), false, 'pas de boucle')
})
