import { test } from 'node:test'
import assert from 'node:assert/strict'
import { organizationShortLabel } from '../../app/utils/organization-label.ts'

const OIF = { name: 'Organisation internationale de la Francophonie', acronym: 'OIF' }
const RF = { name: 'Réseau francophone des entreprises et des femmes en habitat', acronym: 'RF-EFH' }

test('un sigle bien connu suffit là où la place manque', () => {
  assert.equal(organizationShortLabel({ ...OIF, acronymWellKnown: true }), 'OIF')
})

test("un sigle que le public ne connaît pas cède la place au nom complet", () => {
  assert.equal(organizationShortLabel({ ...RF, acronymWellKnown: false }), RF.name)
  assert.equal(organizationShortLabel(RF), RF.name)
})

test('sans sigle, le nom ; sans nom, le sigle ; sans rien, une chaîne vide', () => {
  assert.equal(organizationShortLabel({ name: 'Business Maurice', acronym: null, acronymWellKnown: true }), 'Business Maurice')
  assert.equal(organizationShortLabel({ name: null, acronym: 'GCF' }), 'GCF')
  assert.equal(organizationShortLabel({ name: '  ', acronym: ' ' }), '')
})
