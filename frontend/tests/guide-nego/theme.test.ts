import { test } from 'node:test'
import assert from 'node:assert/strict'
import { lireChoix, themeAffiche } from '../../app/utils/guide-nego/theme.ts'

test('les trois choix se relisent tels quels', () => {
  assert.equal(lireChoix('clair'), 'clair')
  assert.equal(lireChoix('sombre'), 'sombre')
  assert.equal(lireChoix('systeme'), 'systeme')
})

test('rien de gardé, ou une valeur abîmée : le réglage du téléphone', () => {
  assert.equal(lireChoix(null), 'systeme')
  assert.equal(lireChoix(''), 'systeme')
  assert.equal(lireChoix('dark'), 'systeme')
  assert.equal(lireChoix('{"theme":"sombre"}'), 'systeme')
})

test('un choix explicite ne dépend pas du téléphone', () => {
  assert.equal(themeAffiche('clair', true), 'clair')
  assert.equal(themeAffiche('sombre', false), 'sombre')
})

test('« Système » suit le téléphone', () => {
  assert.equal(themeAffiche('systeme', true), 'sombre')
  assert.equal(themeAffiche('systeme', false), 'clair')
})
