import { test } from 'node:test'
import assert from 'node:assert/strict'
import { saisieAGarder, saisieALire } from '../../app/utils/guide-nego/proposition.ts'

const saisie = { terme: 'placeholder text', contexte: 'Entendu au SBSTA' }

test('la saisie revient sur son écran, même si la recherche a changé', () => {
  const brut = saisieAGarder('/guide-nego/lexique?q=placeholder', saisie, 1000)
  assert.deepEqual(saisieALire(brut, '/guide-nego/lexique?q=placeholder%20text', 2000), saisie)
})

test('ailleurs, trop tard ou illisible, rien ne revient', () => {
  const brut = saisieAGarder('/guide-nego/lexique', saisie, 1000)
  assert.equal(saisieALire(brut, '/guide-nego/recherche', 2000), null)
  assert.equal(saisieALire(brut, '/guide-nego/lexique', 1000 + 60 * 60 * 1000), null)
  assert.equal(saisieALire('{', '/guide-nego/lexique', 2000), null)
  assert.equal(saisieALire('', '/guide-nego/lexique', 2000), null)
})
