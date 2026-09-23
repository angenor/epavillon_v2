import { test } from 'node:test'
import assert from 'node:assert/strict'
import type { LibraryDocument, LibraryVocabulary } from '../../app/types/negotiation-documents.ts'
import {
  aucunFiltre,
  compterAvec,
  filtrer,
  marquesDeLigne,
  libelleDuTerme,
  optionsDuFiltre,
  pourUneAutrePersonne,
  type Critere,
} from '../../app/utils/guide-nego/documents.ts'

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

const COP31 = 'cop31'
const COP30 = 'cop30'
const guide = doc('guide', {
  title: 'Guide des négociations — CdP31',
  publisher: 'IFDD',
  themes: ['adaptation', 'gender'],
  cop: COP31,
})
const bulletin = doc('bulletin', { type: 'bulletin', source: 'link', cop: COP31, publisher: 'IISD' })
const note = doc('note', {
  type: 'technical_note',
  title: 'Note technique — Bilan de la CdP30',
  summary: "Ce que la CdP30 a décidé sur l'adaptation.",
  themes: ['adaptation', 'finance'],
  cop: COP30,
  superseded_by: { id: 'guide', title: 'Guide des négociations — CdP31', published_at: '2026-11-02T08:00:00Z', page_count: 92 },
})
const tous = [guide, bulletin, note]

const vocabulaire: LibraryVocabulary = {
  types: [
    { code: 'bulletin', label: 'Bulletin', sort_order: 25 },
    { code: 'negotiation_guide', label: 'Guide', sort_order: 10 },
    { code: 'technical_note', label: 'Note technique', sort_order: 20 },
  ],
  themes: [
    { code: 'gender', label: 'Genre', sort_order: 70 },
    { code: 'adaptation', label: 'Adaptation', sort_order: 10 },
    { code: 'finance', label: 'Finance', sort_order: 30 },
  ],
  cops: [
    { id: COP31, label: 'COP31', city: 'Antalya' },
    { id: COP30, label: 'COP30', city: 'Belém' },
  ],
}

const critere = (champs: Partial<Critere> = {}): Critere => ({
  filtres: aucunFiltre(),
  recherche: '',
  dansLeTexte: null,
  ...champs,
})
const ids = (documents: LibraryDocument[]) => documents.map((d) => d.id)

test('la recherche ignore accents et casse, sur titre, résumé et éditeur', () => {
  assert.deepEqual(ids(filtrer(tous, critere({ recherche: 'NÉGOCIATIONS' }))), ['guide'])
  assert.deepEqual(ids(filtrer(tous, critere({ recherche: 'decide' }))), ['note'])
  assert.deepEqual(ids(filtrer(tous, critere({ recherche: 'CDP3' }))), ['guide', 'note'])
  assert.deepEqual(ids(filtrer(tous, critere({ recherche: 'iisd' }))), ['bulletin'])
  assert.deepEqual(ids(filtrer(tous, critere({ recherche: '  ' }))), ['guide', 'bulletin', 'note'])
})

test('un mot du texte seul suffit, quand la recherche en ligne l’a trouvé', () => {
  assert.deepEqual(ids(filtrer(tous, critere({ recherche: 'crochets' }))), [])
  assert.deepEqual(ids(filtrer(tous, critere({ recherche: 'crochets', dansLeTexte: new Set(['bulletin']) }))), ['bulletin'])
})

test('les valeurs d’un filtre s’additionnent, les filtres se combinent', () => {
  const filtres = { ...aucunFiltre(), types: ['bulletin', 'technical_note'] }
  assert.deepEqual(ids(filtrer(tous, critere({ filtres }))), ['bulletin', 'note'])
  const avecCop = { ...filtres, cops: [COP30] }
  assert.deepEqual(ids(filtrer(tous, critere({ filtres: avecCop }))), ['note'])
})

test('« Aucun bulletin sur le genre » : deux filtres ne laissent rien', () => {
  const filtres = { ...aucunFiltre(), types: ['bulletin'], themes: ['gender'] }
  assert.deepEqual(filtrer(tous, critere({ filtres })), [])
})

test('un réservé sans accès ne répond à aucun filtre de thématique', () => {
  const cache = doc('reserve', { restricted: true, accessible: false, themes: [], themes_hidden: true })
  const filtres = { ...aucunFiltre(), themes: ['adaptation'] }
  assert.deepEqual(ids(filtrer([cache, guide], critere({ filtres }))), ['guide'])
})

test('les comptes par valeur gardent les autres filtres et ignorent le leur', () => {
  const c = critere({ filtres: { ...aucunFiltre(), types: ['negotiation_guide'], cops: [COP31] } })
  const types = optionsDuFiltre(tous, vocabulaire, c, 'types')
  assert.deepEqual(
    types.map((o) => [o.libelle, o.compte]),
    [
      ['Guide', 1],
      ['Note technique', 0],
      ['Bulletin', 1],
    ],
  )
  const themes = optionsDuFiltre(tous, vocabulaire, c, 'themes')
  assert.deepEqual(
    themes.map((o) => [o.valeur, o.compte]),
    [
      ['adaptation', 1],
      ['finance', 0],
      ['gender', 1],
    ],
  )
  assert.deepEqual(
    optionsDuFiltre(tous, vocabulaire, critere(), 'cops').map((o) => [o.libelle, o.compte]),
    [
      ['COP31 — Antalya', 2],
      ['COP30 — Belém', 1],
    ],
  )
})

test('« Afficher n documents » compte le brouillon avant de l’appliquer', () => {
  const c = critere({ recherche: 'cdp' })
  assert.equal(compterAvec(tous, c, 'types', []), 2)
  assert.equal(compterAvec(tous, c, 'types', ['technical_note']), 1)
  assert.equal(compterAvec(tous, c, 'types', ['bulletin']), 0)
})

test('les marques d’une ligne, en ligne', () => {
  const enLigne = { telecharge: false, nouveau: false, enLigne: true }
  assert.deepEqual(marquesDeLigne(guide, { ...enLigne, telecharge: true }), ['telecharge', 'a-jour'])
  assert.deepEqual(marquesDeLigne(doc('r', { restricted: true }), { ...enLigne, nouveau: true }), ['nouveau', 'reserve'])
  assert.deepEqual(marquesDeLigne(bulletin, { ...enLigne, nouveau: true }), ['nouveau', 'lien-externe'])
  assert.deepEqual(marquesDeLigne(note, enLigne), ['remplace'])
  assert.deepEqual(marquesDeLigne(doc('seul'), enLigne), ['a-jour'])
})

test('hors connexion, les marques disent ce qui s’ouvre', () => {
  const horsLigne = { telecharge: false, nouveau: true, enLigne: false }
  assert.deepEqual(marquesDeLigne(guide, { ...horsLigne, telecharge: true }), ['telecharge', 'a-jour'])
  assert.deepEqual(marquesDeLigne(doc('r', { restricted: true }), horsLigne), ['reserve', 'non-telecharge'])
  assert.deepEqual(marquesDeLigne(bulletin, horsLigne), ['lien-reseau'])
  assert.deepEqual(marquesDeLigne(note, horsLigne), ['remplace', 'non-telecharge'])
  assert.deepEqual(marquesDeLigne(doc('seul'), horsLigne), ['non-telecharge'])
})

test('une liste gardée pour une autre personne ne montre pas ses réservés', () => {
  const ouvert = doc('reserve', {
    restricted: true,
    accessible: true,
    summary: 'Les lignes rouges',
    themes: ['finance'],
    source: 'link',
    external_url: 'https://exemple.org/reserve',
  })
  const [masque, public_] = pourUneAutrePersonne({
    documents: [ouvert, guide],
    vocabulary: vocabulaire,
    served_at: '2026-11-12T08:00:00Z',
  }).documents
  assert.deepEqual(
    [masque?.summary, masque?.themes, masque?.themes_hidden, masque?.external_url, masque?.accessible],
    [null, [], true, null, false],
  )
  assert.equal(masque?.title, 'reserve', 'le titre reste, comme pour qui n’a pas l’accès')
  assert.equal(public_, guide)
})

test('un terme désactivé n’a pas de libellé, jamais son code', () => {
  assert.equal(libelleDuTerme(vocabulaire.types, 'bulletin'), 'Bulletin')
  assert.equal(libelleDuTerme(vocabulaire.types, 'report'), null)
})
