import { test } from 'node:test'
import assert from 'node:assert/strict'
import type { PathwayGroup, PathwayLink } from '../../app/types/negotiation-savoir.ts'
import { avancement, changeesAilleurs, destinationDeLEtape } from '../../app/utils/guide-nego/ma-premiere-cop.ts'
import { pageDeLaSection } from '../../app/utils/guide-nego/forme-lisible.ts'

const etape = (id: string) => ({ id, label: id, detail: null, origin_label: null, link: null, sort_order: 0 })
const GROUPES: PathwayGroup[] = [
  { id: 'avant', label: 'Avant de partir', sort_order: 1, steps: [etape('a1'), etape('a2')] },
  { id: 'jour', label: 'Le premier jour', sort_order: 2, steps: [etape('j1'), etape('j2'), etape('j3')] },
]

test("l'avancement compte les étapes publiées et désigne la première non cochée", () => {
  const a = avancement(GROUPES, new Set(['a1', 'a2', 'j2', 'retiree']))
  assert.equal(a.faites, 3, "une coche d'étape retirée ne compte pas")
  assert.equal(a.total, 5)
  assert.deepEqual(a.parGroupe, { avant: { faites: 2, total: 2 }, jour: { faites: 1, total: 3 } })
  assert.equal(a.prochaine?.etape.id, 'j1')
  assert.equal(a.prochaine?.groupe.id, 'jour')
  assert.equal(avancement(GROUPES, new Set(['a1', 'a2', 'j1', 'j2', 'j3'])).prochaine, null)
})

test('un lien « Lire : … » mène au document, à la FAQ ou au terme', () => {
  const lien = (l: Partial<PathwayLink>): PathwayLink => ({
    kind: 'document', target_id: 'd1', page: null, section: null, label: null, ...l,
  })
  assert.equal(destinationDeLEtape(lien({ page: 12 }), () => null), '/guide-nego/ressources/documents/d1/lire?page=12')
  assert.equal(
    destinationDeLEtape(lien({ section: 'chapitre 3' }), () => null),
    '/guide-nego/ressources/documents/d1/lire?section=chapitre%203',
  )
  assert.equal(destinationDeLEtape(lien({}), () => null), '/guide-nego/ressources/documents/d1')
  assert.equal(destinationDeLEtape(lien({ kind: 'faq', target_id: 'f1' }), () => null), '/guide-nego/ressources/faq/f1')
  const slug = (id: string) => (id === 'g1' ? 'bracketed-text' : null)
  assert.equal(destinationDeLEtape(lien({ kind: 'glossary', target_id: 'g1' }), slug), '/guide-nego/lexique/bracketed-text')
  assert.equal(destinationDeLEtape(lien({ kind: 'glossary', target_id: 'g2' }), slug), null, 'terme non servi : pas de lien')
})

test("une relecture qui change l'écran se dit, sauf pour ce que la file porte encore", () => {
  assert.equal(changeesAilleurs(['a1', 'j1'], ['j1', 'a1'], [], null), false, "l'ordre ne compte pas")
  assert.equal(changeesAilleurs(['a1', 'j1'], ['a1'], [], null), true, 'décochée ailleurs')
  assert.equal(changeesAilleurs(['a1'], ['a1', 'j1'], [], null), true, 'cochée ailleurs')
  assert.equal(changeesAilleurs(['a1'], ['a1', 'j1'], ['j1'], null), false, 'le geste de ce téléphone')
  assert.equal(changeesAilleurs(['a1'], ['a1', 'x'], [], new Set(['a1'])), false, 'une étape non publiée ne compte pas')
})

test("une section se trouve dans le sommaire, sans accents ni casse", () => {
  const sommaire = [
    { title: 'Chapitre 1 — Le cadre', level: 1 as const, page_index: 3, children: [] },
    {
      title: 'Chapitre 3 — Les thématiques', level: 1 as const, page_index: 40,
      children: [{ title: 'Annexe A.5 Coordonnées', level: 2 as const, page_index: 74, children: [] }],
    },
  ]
  assert.equal(pageDeLaSection(sommaire, 'chapitre 3'), 40)
  assert.equal(pageDeLaSection(sommaire, 'annexe A.5'), 74)
  assert.equal(pageDeLaSection(sommaire, 'chapitre 9'), null)
  const numerote = [
    { title: '2. État des négociations', level: 1 as const, page_index: 26, children: [] },
    { title: '3. LES ENJEUX', level: 1 as const, page_index: 47, children: [{ title: '3.1. L’ordre du jour', level: 2 as const, page_index: 47, children: [] }] },
  ]
  assert.equal(pageDeLaSection(numerote, 'chapitre 3'), 47, 'à défaut du nom, le numéro')
  assert.equal(pageDeLaSection(numerote, 'chapitre deux'), null)
})
