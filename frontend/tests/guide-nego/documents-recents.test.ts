import { test } from 'node:test'
import assert from 'node:assert/strict'
import type { LibraryDocument } from '../../app/types/negotiation-documents.ts'
import { documentsRecents } from '../../app/utils/guide-nego/journee.ts'
import {
  lireProgression,
  lireRecents,
  noterOuverture,
  noterProgression,
  type Stockage,
} from '../../app/utils/guide-nego/appareil-lecture.ts'

function doc(id: string, version = '1.0'): LibraryDocument {
  return {
    id,
    slug: id,
    version,
    title: id,
    summary: null,
    type: 'negotiation_guide',
    themes: [],
    themes_hidden: false,
    cop: null,
    issued_on: null,
    published_at: '2026-11-01T08:00:00Z',
    publisher: 'IFDD',
    locale: 'fr',
    source: 'file',
    external_url: null,
    link_host: null,
    restricted: false,
    accessible: true,
    page_count: 92,
    reading_bytes: 1000,
    mode: 'reflow',
    superseded_by: null,
    reading_etag: '"e"',
  }
}

function telephone(): Stockage {
  const cles = new Map<string, string>()
  return { lire: (cle) => cles.get(cle) ?? null, poser: (cle, valeur) => void cles.set(cle, valeur) }
}

const recentsDe = (stockage: Stockage, documents: LibraryDocument[]) =>
  documentsRecents(lireRecents(stockage), documents, (id, version) => lireProgression(stockage, id, version))

test('les récents gardent l’ordre du téléphone, le dernier ouvert d’abord', () => {
  const stockage = telephone()
  noterOuverture(stockage, 'guide', '2026-11-11T18:05:00Z')
  noterOuverture(stockage, 'bulletin', '2026-11-12T08:00:00Z')
  noterProgression(stockage, 'guide', '1.0', 14, '2026-11-11T18:30:00Z')

  const lignes = recentsDe(stockage, [doc('guide'), doc('bulletin')])

  assert.deepEqual(
    lignes.map((l) => [l.document.id, l.page, l.lu]),
    [
      ['bulletin', null, '2026-11-12T08:00:00Z'],
      ['guide', 14, '2026-11-11T18:30:00Z'],
    ],
  )
})

test('un document dépublié ne paraît plus', () => {
  const stockage = telephone()
  noterOuverture(stockage, 'retire', '2026-11-11T18:05:00Z')
  noterOuverture(stockage, 'guide', '2026-11-12T08:00:00Z')

  assert.deepEqual(recentsDe(stockage, [doc('guide')]).map((l) => l.document.id), ['guide'])
})

test('la page notée d’une autre version ne se reprend pas', () => {
  const stockage = telephone()
  noterOuverture(stockage, 'guide', '2026-11-11T18:05:00Z')
  noterProgression(stockage, 'guide', '1.0', 14, '2026-11-11T18:30:00Z')

  const [ligne] = recentsDe(stockage, [doc('guide', '2.0')])

  assert.equal(ligne?.page, null)
})

test('un téléphone neuf, ou une bibliothèque pas encore lue, ne rend rien', () => {
  assert.deepEqual(recentsDe(telephone(), [doc('guide')]), [])

  const stockage = telephone()
  noterOuverture(stockage, 'guide', '2026-11-11T18:05:00Z')
  assert.deepEqual(recentsDe(stockage, []), [])
})

test('un réservé fermé à la personne ne dit pas ce qu’une autre a lu sur ce téléphone', () => {
  const stockage = telephone()
  noterOuverture(stockage, 'resume', '2026-11-11T18:05:00Z')
  noterOuverture(stockage, 'guide', '2026-11-12T08:00:00Z')
  const resume = { ...doc('resume'), restricted: true, accessible: false }

  assert.deepEqual(recentsDe(stockage, [doc('guide'), resume]).map((l) => l.document.id), ['guide'])
  assert.equal(recentsDe(stockage, [{ ...resume, accessible: true }]).length, 1)
})
