import { test } from 'node:test'
import assert from 'node:assert/strict'
import {
  DRAPEAU_APPLICATION as APP,
  DRAPEAU_ECHANGES as ECHANGES,
  echangesOuverts,
  extraireDrapeaux,
  fusionner,
  resoudreDrapeau,
} from '../../app/utils/guide-nego/drapeaux.ts'

for (const cle of [APP, ECHANGES] as const) {
  test(`${cle} — réponse « allumé » : ouvert, quelle que soit la garde`, () => {
    assert.equal(resoudreDrapeau({ [cle]: true }, { [cle]: false }, cle), true)
    assert.equal(resoudreDrapeau({ [cle]: true }, null, cle), true)
  })

  test(`${cle} — réponse « éteint » : fermé, même si la garde disait ouvert`, () => {
    assert.equal(resoudreDrapeau({ [cle]: false }, { [cle]: true }, cle), false)
  })

  test(`${cle} — drapeau absent de la réponse : la garde tient`, () => {
    assert.equal(resoudreDrapeau({}, { [cle]: true }, cle), true)
    assert.equal(resoudreDrapeau({}, { [cle]: false }, cle), false)
  })

  test(`${cle} — API muette : la garde tient`, () => {
    assert.equal(resoudreDrapeau(null, { [cle]: true }, cle), true)
    assert.equal(resoudreDrapeau(null, { [cle]: false }, cle), false)
  })

  test(`${cle} — jamais lu : fermé`, () => {
    assert.equal(resoudreDrapeau(null, null, cle), false)
    assert.equal(resoudreDrapeau({}, null, cle), false)
  })
}

test('une réponse illisible vaut une API muette', () => {
  assert.equal(extraireDrapeaux('<html>502</html>'), null)
  assert.equal(extraireDrapeaux({ erreur: true }), null)
  assert.deepEqual(extraireDrapeaux([{ key: APP, is_enabled: 'oui' }, null, 3]), {})
})

test('on ne garde que les deux drapeaux de Guide Négo', () => {
  const reponse = [
    { key: 'tools.enabled', is_enabled: true },
    { key: APP, is_enabled: true },
    { key: ECHANGES, is_enabled: false },
  ]
  assert.deepEqual(extraireDrapeaux(reponse), { [APP]: true, [ECHANGES]: false })
})

test('ce que la réponse ne dit pas reste gardé', () => {
  assert.deepEqual(fusionner({ [APP]: true }, { [APP]: false, [ECHANGES]: true }), {
    [APP]: true,
    [ECHANGES]: true,
  })
})

test('pas de cinquième onglet dans une application fermée', () => {
  assert.equal(echangesOuverts({ [APP]: false, [ECHANGES]: true }, null), false)
  assert.equal(echangesOuverts({ [APP]: true, [ECHANGES]: true }, null), true)
  assert.equal(echangesOuverts(null, { [APP]: true, [ECHANGES]: true }), true)
})
