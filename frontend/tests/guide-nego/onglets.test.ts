import { test } from 'node:test'
import assert from 'node:assert/strict'
import { ongletActif, onglets } from '../../app/utils/guide-nego/onglets.ts'

test('Échanges fermés : quatre onglets, Ressources en dernier', () => {
  const liste = onglets(false)
  assert.deepEqual(
    liste.map((o) => o.cle),
    ['accueil', 'negociations', 'francophonie', 'ressources'],
  )
})

test('Échanges ouverts : cinq onglets, Échanges en quatrième', () => {
  const liste = onglets(true)
  assert.deepEqual(
    liste.map((o) => o.cle),
    ['accueil', 'negociations', 'francophonie', 'echanges', 'ressources'],
  )
})

test('chaque onglet a son adresse et son pictogramme', () => {
  for (const onglet of onglets(true)) {
    assert.match(onglet.route, /^\/guide-nego/)
    assert.ok(onglet.picto.length > 0)
  }
})

test('onglet actif : la route la plus longue gagne, pas l’accueil', () => {
  const liste = onglets(true)
  assert.equal(ongletActif('/guide-nego', liste)?.cle, 'accueil')
  assert.equal(ongletActif('/guide-nego/', liste)?.cle, 'accueil')
  assert.equal(ongletActif('/guide-nego/negociations', liste)?.cle, 'negociations')
  assert.equal(ongletActif('/guide-nego/ressources/reglages', liste)?.cle, 'ressources')
})

test('un écran secondaire n’allume aucun onglet', () => {
  assert.equal(ongletActif('/guide-nego/lexique', onglets(true)), null)
})

test('Échanges fermés : leur adresse n’allume aucun onglet', () => {
  assert.equal(ongletActif('/guide-nego/echanges', onglets(false)), null)
})
