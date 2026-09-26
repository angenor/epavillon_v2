import { test } from 'node:test'
import assert from 'node:assert/strict'
import type { LibraryDocument } from '../../app/types/negotiation-documents.ts'
import type { Passage } from '../../app/utils/guide-nego/lecteur.ts'
import {
  documentsTrouves,
  PASSAGES_PAR_DOCUMENT,
  rechercheLancee,
  sessionsTrouvees,
} from '../../app/utils/guide-nego/recherche-globale.ts'
import { BELEM, session } from './fausses-sessions.ts'

function doc(id: string, champs: Partial<LibraryDocument> = {}): LibraryDocument {
  return {
    id,
    slug: id,
    version: '1.0',
    title: id,
    summary: null,
    type: 'negotiation_guide',
    themes: [],
    themes_hidden: false,
    cop: null,
    issued_on: null,
    published_at: '2026-11-01T08:00:00Z',
    publisher: null,
    locale: 'fr',
    source: 'file',
    external_url: null,
    link_host: null,
    restricted: false,
    accessible: true,
    page_count: 10,
    reading_bytes: 1000,
    mode: 'reflow',
    superseded_by: null,
    reading_etag: '"e"',
    ...champs,
  }
}

const passage = (page: number, trouve = 'contact'): Passage => ({
  page,
  bloc: 0,
  champ: 'spans',
  debut: 0,
  fin: trouve.length,
  etiquette: String(page + 10),
  section: null,
  extrait: { avant: 'le groupe de ', trouve, apres: ' est ouvert' },
})

test('une lettre seule ne cherche rien, accents et blancs ne comptent pas', () => {
  assert.equal(rechercheLancee('c'), false)
  assert.equal(rechercheLancee('  é '), false)
  assert.equal(rechercheLancee('co'), true)
  assert.deepEqual(documentsTrouves([doc('a', { title: 'contact' })], 'c', null, new Map()), [])
})

test('sans réseau, les documents se trouvent par leur fiche, puis par le texte des copies gardées, cinq passages au plus', () => {
  const guide = doc('guide', { title: 'Guide des négociations' })
  const iisd = doc('iisd', { title: 'Au nom de ma délégation', publisher: 'IISD' })
  const garde = doc('garde', { title: 'Contact et consultations' })
  const passages = Array.from({ length: 8 }, (_, i) => passage(i + 1))
  const trouves = documentsTrouves([guide, iisd, garde], 'CONTACT', null, new Map([['guide', passages]]))
  assert.deepEqual(trouves.map((t) => t.document.id), ['garde', 'guide'], 'la fiche d’abord, le texte ensuite')
  assert.equal(trouves[0]?.passages.length, 0)
  assert.equal(trouves[1]?.passages.length, PASSAGES_PAR_DOCUMENT)
  assert.deepEqual(trouves[1]?.passages[0], { page: 1, etiquette: '11', extrait: 'le groupe de contact est ouvert' })
  assert.equal(documentsTrouves([iisd], 'iisd', null, new Map()).length, 1, 'l’éditeur répond aussi')
})

test('en ligne, les pages servies font foi et les copies gardées ne comptent plus', () => {
  const guide = doc('guide', { title: 'Guide des négociations' })
  const reserve = doc('reserve', { title: 'Note réservée', restricted: true, accessible: false })
  const trouves = documentsTrouves(
    [guide, reserve],
    'contact',
    [
      { document_id: 'reserve', pages: [] },
      { document_id: 'guide', pages: [{ index: 14, label: '14', excerpt: 'Le groupe de contact' }] },
      { document_id: 'inconnu', pages: [{ index: 1, label: '1', excerpt: 'contact' }] },
    ],
    new Map([['guide', [passage(3)]]]),
  )
  assert.deepEqual(trouves.map((t) => t.document.id), ['reserve', 'guide'], 'un document hors de la liste ne s’invente pas')
  assert.deepEqual(trouves[0]?.passages, [], 'un réservé sans accès se nomme, sans passage')
  assert.deepEqual(trouves[1]?.passages, [{ page: 14, etiquette: '14', extrait: 'Le groupe de contact' }])
})

test('les sessions trouvées sont celles du jour, par titre anglais, traduction ou salle', () => {
  const sessions = [
    session('b', '2027-11-12T18:00:00Z', null, { title_en: 'Contact group on gender' }),
    session('a', '2027-11-12T13:00:00Z', null, { title_en: 'Informal consultations', title_fr: 'Groupe de contact informel' }),
    session('c', '2027-11-12T15:00:00Z', null, { title_en: 'Plenary', venue: 'Salle Contact' }),
    session('d', '2027-11-13T13:00:00Z', null, { title_en: 'Contact group, next day' }),
    session('e', '2027-11-12T12:00:00Z', null, { title_en: 'Plenary' }),
  ]
  assert.deepEqual(sessionsTrouvees(sessions, 'contact', '2027-11-12', BELEM).map((s) => s.id), ['a', 'c', 'b'])
  assert.deepEqual(sessionsTrouvees(sessions, 'contact', null, BELEM), [])
  assert.deepEqual(sessionsTrouvees(sessions, 'c', '2027-11-12', BELEM), [])
})
