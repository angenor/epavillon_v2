import { test } from 'node:test'
import assert from 'node:assert/strict'
import { isEmailAddress, parseAddressList, renderMailingTemplate } from '../../app/utils/mailing.ts'

test('une saisie libre se coupe, passe en minuscules et se dédoublonne', () => {
  assert.deepEqual(parseAddressList('Awa@Example.org, b@example.org;\n<awa@example.org>  c@example.org'), [
    'awa@example.org',
    'b@example.org',
    'c@example.org',
  ])
  assert.deepEqual(parseAddressList('  '), [])
})

test("le contrôle d'adresse est celui du domaine platform.email", () => {
  assert.equal(isEmailAddress('a@b.co'), true)
  assert.equal(isEmailAddress('a@b.c'), false)
  assert.equal(isEmailAddress('pas une adresse'), false)
})

test("l'aperçu échappe les valeurs dans le HTML, pas dans l'objet, et garde une variable inconnue", () => {
  const values = { prenom: 'A<b>', nom: "O'Neil" }
  assert.equal(renderMailingTemplate('<p>Bonjour {{prenom}} {{ nom }}</p>', values, true), '<p>Bonjour A&lt;b&gt; O&#39;Neil</p>')
  assert.equal(renderMailingTemplate('Pour {{prenom}}', values, false), 'Pour A<b>')
  assert.equal(renderMailingTemplate('{{inconnue}}', values, true), '{{inconnue}}')
})
