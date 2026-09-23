import { test } from 'node:test'
import assert from 'node:assert/strict'
import type { DocumentReading, OutlineEntry, ReadingPage } from '../../app/types/negotiation-documents.ts'
import { blocsDeLaPage, estUneFormeLisible, pageDeReprise, partLue, sectionDeLaPage } from '../../app/utils/guide-nego/forme-lisible.ts'

const page = (blocks: unknown[], index = 59): ReadingPage => ({ index, label: String(index), blocks: blocks as ReadingPage['blocks'] })

test('un kind inconnu s’ignore, le reste de la page se rend', () => {
  const blocs = blocsDeLaPage(
    page([
      { kind: 'heading', level: 2, spans: [{ text: '3.6. Adaptation', bold: true }] },
      { kind: 'tableau_riche', html: '<table>' },
      { kind: 'paragraph', spans: [{ text: 'Le suivi des progrès collectifs.' }] },
    ]),
  )
  assert.deepEqual(
    blocs.map((b) => b.kind),
    ['heading', 'paragraph'],
  )
})

test('un bloc ou un segment mal formé ne casse rien', () => {
  const blocs = blocsDeLaPage(
    page([
      { kind: 'heading', level: 7, spans: [{ text: 'Titre' }] },
      { kind: 'paragraph', spans: [{ text: 'Bon' }, { text: 42 }, null, { italic: true }] },
      { kind: 'origin', reason: 'graphique' },
      null,
    ]),
  )
  assert.deepEqual(blocs, [{ kind: 'paragraph', spans: [{ text: 'Bon' }] }])
})

test('un terme anglais est repéré, et toujours en italique', () => {
  const [bloc] = blocsDeLaPage(page([{ kind: 'paragraph', spans: [{ text: 'global goal on adaptation', term: true }] }]))
  assert.deepEqual(bloc, { kind: 'paragraph', spans: [{ text: 'global goal on adaptation', italic: true, term: true }] })
})

test('les notes de bas de page passent en fin de page', () => {
  const blocs = blocsDeLaPage(
    page([
      { kind: 'note', mark: '1', spans: [{ text: 'Décision 2/CMA.5.' }] },
      { kind: 'paragraph', spans: [{ text: 'Texte' }] },
      { kind: 'origin', reason: 'table', text: [{ text: 'Colonne A' }] },
    ]),
  )
  assert.deepEqual(
    blocs.map((b) => b.kind),
    ['paragraph', 'origin', 'note'],
  )
  assert.deepEqual(blocs[1], { kind: 'origin', reason: 'table', text: [{ text: 'Colonne A' }] })
})

const entree = (level: 1 | 2 | 3, page_index: number, title: string, children: OutlineEntry[] = []): OutlineEntry => ({
  title,
  level,
  page_index,
  children,
})

const sommaire = [
  entree(1, 47, '3. Les enjeux', [
    entree(2, 57, '3.5. Atténuation'),
    entree(2, 59, '3.6. Adaptation', [entree(3, 59, '3.6.1. GGA'), entree(3, 60, '3.6.2. PNA')]),
    entree(2, 62, '3.7. Pertes et préjudices'),
  ]),
  entree(1, 65, 'Annexes'),
]

test('le pied dit la section de niveau 1 ou 2, pas la sous-partie', () => {
  assert.equal(sectionDeLaPage(sommaire, 59)?.title, '3.6. Adaptation')
  assert.equal(sectionDeLaPage(sommaire, 61)?.title, '3.6. Adaptation')
  assert.equal(sectionDeLaPage(sommaire, 47)?.title, '3. Les enjeux')
  assert.equal(sectionDeLaPage(sommaire, 70)?.title, 'Annexes')
  assert.equal(sectionDeLaPage(sommaire, 12), null)
  assert.equal(sectionDeLaPage([], 12), null)
})

test('la reprise : la page notée si elle existe encore, jamais la première', () => {
  const lecture = { pages: [page([], 1), page([], 2), page([], 59)] } as DocumentReading
  assert.equal(pageDeReprise(lecture, 59)?.index, 59)
  assert.equal(pageDeReprise(lecture, 1), null)
  assert.equal(pageDeReprise(lecture, null), null)
  assert.equal(pageDeReprise(lecture, 91), null, 'une version plus courte : on repart du début')
})

test('la part lue', () => {
  assert.equal(partLue(46, 92), 0.5)
  assert.equal(partLue(3, 0), 0)
})

test('une copie d’une autre forme ne s’ouvre pas : elle se retélécharge', () => {
  const bonne = { version: '1.0', mode: 'reflow', page_count: 1, outline: [], pages: [{ index: 1, label: '1', blocks: [] }] }
  assert.equal(estUneFormeLisible(bonne), true)
  assert.equal(estUneFormeLisible({ ...bonne, pages: [] }), false, 'sans page, ce serait une page blanche')
  assert.equal(estUneFormeLisible({ ...bonne, mode: 'pdf' }), false)
  assert.equal(estUneFormeLisible({ ...bonne, pages: [{ index: '1' }] }), false)
  assert.equal(estUneFormeLisible(null), false)
  assert.equal(estUneFormeLisible('{"version":"1.0"}'), false)
})
