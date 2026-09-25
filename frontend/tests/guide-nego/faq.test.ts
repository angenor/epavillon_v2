import { test } from 'node:test'
import assert from 'node:assert/strict'
import type { FaqEntry } from '../../app/types/negotiation-savoir.ts'
import { destinationDe, jourDeVerification, lectureACompter, liees } from '../../app/utils/guide-nego/faq.ts'

test('une source de document ouvre le lecteur à la page citée', () => {
  assert.deepEqual(destinationDe({ document_id: 'd1', page_from: 74 }), {
    interne: '/guide-nego/ressources/documents/d1/lire?page=74',
  })
  assert.deepEqual(destinationDe({ document_id: 'd1' }), { interne: '/guide-nego/ressources/documents/d1/lire' })
  assert.deepEqual(destinationDe({ external_title: 'IISD', external_url: 'https://iisd.org' }), {
    externe: 'https://iisd.org',
  })
  assert.equal(destinationDe({ external_title: 'IISD', page_from: 38 }), null)
})

test('le jour de vérification ne glisse pas selon le fuseau', () => {
  assert.equal(jourDeVerification('2026-11-12', 'fr'), '12 novembre 2026')
  assert.equal(jourDeVerification('2026-11-12', 'en'), 'November 12, 2026')
})

test('les questions liées absentes du téléphone, ou la question elle-même, sont écartées', () => {
  const e = (id: string, related_ids: string[] = []) => ({ id, related_ids }) as unknown as FaqEntry
  const toutes = [e('a', ['b', 'x', 'a']), e('b')]
  assert.deepEqual(liees(toutes[0]!, toutes).map((l) => l.id), ['b'])
})

test('une lecture se compte une fois par entrée et par jour', () => {
  const premiere = lectureACompter(null, 'a', '2026-11-12')
  assert.equal(premiere.compter, true)
  const seconde = lectureACompter(premiere.garder, 'a', '2026-11-12')
  assert.equal(seconde.compter, false)
  assert.equal(lectureACompter(seconde.garder, 'b', '2026-11-12').compter, true)
  assert.equal(lectureACompter(seconde.garder, 'a', '2026-11-13').compter, true)
  assert.deepEqual(JSON.parse(lectureACompter(seconde.garder, 'a', '2026-11-13').garder), { jour: '2026-11-13', ids: ['a'] })
  assert.equal(lectureACompter('{illisible', 'a', '2026-11-12').compter, true)
})
