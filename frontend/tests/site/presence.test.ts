import { test } from 'node:test'
import assert from 'node:assert/strict'
import {
  PRESENCE_TAB_KEY,
  isValidTabId,
  presenceBarWidth,
  presenceTabId,
  presenceZoneOf,
  secondsSince,
  sparklinePaths,
} from '../../app/utils/presence.ts'

test('chaque page du site tombe dans sa zone', () => {
  const cas: Array<[string, string]> = [
    ['activity-edition-session-room___fr', 'live'],
    ['track-edition-track-room___fr', 'live'],
    ['track-edition-track-live___en', 'live'],
    ['programme___fr', 'programme'],
    ['track-edition-track___fr', 'day'],
    ['track-edition-track-my-registration___fr', 'day'],
    ['activity-edition-session___fr', 'activity'],
    ['organization-workspace___fr', 'organization'],
    ['organization-proposal-id___fr', 'organization'],
    ['organization-proposal-cover-id___en', 'organization'],
    ['organization-join___fr', 'organization'],
    ['proposal-form___fr', 'organization'],
    ['admin___fr', 'admin'],
    ['admin-journal___fr', 'admin'],
    ['admin-evenements-id-journees-trackId___fr', 'admin'],
    ['guide-nego___fr', 'negotiation'],
    ['guide-nego-negociations-agenda___fr', 'negotiation'],
    ['negotiations___fr', 'negotiation'],
    ['index___fr', 'other'],
    ['event-slug___fr', 'other'],
    ['auth-login___fr', 'other'],
    ['administration-de-test___fr', 'other'],
    ['organizations___fr', 'other'],
  ]
  for (const [route, zone] of cas) assert.equal(presenceZoneOf(route), zone, route)
  assert.equal(presenceZoneOf(null), 'other')
})

test("l'identifiant d'onglet est gardé, et refait s'il manque ou ne convient pas", () => {
  const store = new Map<string, string>()
  const storage = {
    getItem: (k: string) => store.get(k) ?? null,
    setItem: (k: string, v: string) => void store.set(k, v),
  }
  let n = 0
  const make = () => `onglet-${++n}-abcdef`
  const premier = presenceTabId(storage, make)
  assert.ok(isValidTabId(premier))
  assert.equal(presenceTabId(storage, make), premier)
  store.set(PRESENCE_TAB_KEY, 'trafiqué <script>')
  assert.notEqual(presenceTabId(storage, make), premier)
  assert.equal(presenceTabId(null, make), 'onglet-3-abcdef')
  const refuse = {
    getItem: () => {
      throw new Error('refusé')
    },
    setItem: () => {
      throw new Error('refusé')
    },
  }
  assert.equal(presenceTabId(refuse, make), 'onglet-4-abcdef')
})

test("l'identifiant suit la règle de l'API", () => {
  assert.ok(isValidTabId('a1B2-c3_d4'))
  assert.ok(!isValidTabId('court'))
  assert.ok(!isValidTabId('x'.repeat(65)))
  assert.ok(!isValidTabId(null))
})

test('barres et âge du relevé', () => {
  assert.equal(presenceBarWidth(187, 187), 100)
  assert.equal(presenceBarWidth(97, 187), 52)
  assert.equal(presenceBarWidth(1, 1000), 2)
  assert.equal(presenceBarWidth(0, 10), 0)
  assert.equal(presenceBarWidth(3, 0), 0)
  const at = '2026-11-03T10:00:00.000Z'
  assert.equal(secondsSince(at, Date.parse(at) + 12_400), 12)
  assert.equal(secondsSince(at, Date.parse(at) - 5_000), 0)
  assert.equal(secondsSince('pas une date', 0), 0)
})

test('tracé de la dernière heure', () => {
  assert.equal(sparklinePaths([], 600, 56), null)
  assert.equal(sparklinePaths([{ total: 4 }], 600, 56), null)
  const tracé = sparklinePaths([{ total: 0 }, { total: 10 }, { total: 5 }], 600, 56, 6)
  assert.ok(tracé)
  assert.equal(tracé.line, 'M0 56 L300 6 L600 31')
  assert.equal(tracé.area, 'M0 56 L300 6 L600 31 L600 56 L0 56 Z')
  // Que des zéros : une ligne au ras du bas, pas une division par zéro.
  assert.equal(sparklinePaths([{ total: 0 }, { total: 0 }], 100, 20)?.line, 'M0 20 L100 20')
})
