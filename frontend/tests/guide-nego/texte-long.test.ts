import { test } from 'node:test'
import assert from 'node:assert/strict'
import { blocsDe, segmentsDe } from '../../app/utils/guide-nego/texte-long.ts'

test('titres, paragraphes et listes deviennent des blocs', () => {
  const blocs = blocsDe('## Vos données\n\nUne ligne\nqui continue.\n\n### Durée\n- un an\n- deux ans\n\n1. d’abord\n2. ensuite')
  assert.deepEqual(
    blocs.map((b) => b.type),
    ['titre2', 'paragraphe', 'titre3', 'liste', 'liste-numerotee'],
  )
  const paragraphe = blocs[1]!
  assert.ok(paragraphe.type === 'paragraphe')
  assert.deepEqual(paragraphe.segments, [{ type: 'texte', texte: 'Une ligne qui continue.' }])
  const liste = blocs[3]!
  assert.ok(liste.type === 'liste')
  assert.equal(liste.elements.length, 2)
})

test('emphase et liens sûrs se reconnaissent', () => {
  assert.deepEqual(segmentsDe('Lire **tout** le *texte* sur [le site](https://ifdd.francophonie.org).'), [
    { type: 'texte', texte: 'Lire ' },
    { type: 'gras', texte: 'tout' },
    { type: 'texte', texte: ' le ' },
    { type: 'italique', texte: 'texte' },
    { type: 'texte', texte: ' sur ' },
    { type: 'lien', texte: 'le site', adresse: 'https://ifdd.francophonie.org' },
    { type: 'texte', texte: '.' },
  ])
})

/** Rien ne devient du HTML : une adresse qui n'est ni web ni courriel reste son texte. */
test('une adresse douteuse ne devient jamais un lien', () => {
  assert.deepEqual(segmentsDe('[cliquer](javascript:alert(1))'), [
    { type: 'texte', texte: 'cliquer' },
    { type: 'texte', texte: ')' },
  ])
  assert.deepEqual(segmentsDe('<b>gras</b>'), [{ type: 'texte', texte: '<b>gras</b>' }])
})

test('un texte vide ne rend rien', () => {
  assert.deepEqual(blocsDe(''), [])
})
