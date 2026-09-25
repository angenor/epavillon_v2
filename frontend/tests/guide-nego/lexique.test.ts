import { test } from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import type { GlossaryEntry } from '../../app/types/negotiation-savoir.ts'
import { grouperParLettre, lettreDe, normaliserTerme, resoudreLeTerme, slugDe } from '../../app/utils/guide-nego/lexique.ts'

// Les mêmes chaînes passent à `platform.normalize_label` dans `savoir_normalisation.rs`.
const CAS = JSON.parse(
  readFileSync(
    new URL('../../../backend/crates/modules/negotiation/tests/fixtures/normalisation.json', import.meta.url),
    'utf8',
  ),
) as { texte: string; attendu: string }[]

test('normaliserTerme rend ce que rend platform.normalize_label, sur les vingt chaînes communes', () => {
  assert.equal(CAS.length, 20)
  for (const { texte, attendu } of CAS) assert.equal(normaliserTerme(texte), attendu, texte)
})

test('le slug suit platform.slugify', () => {
  assert.equal(slugDe('global goal on adaptation'), 'global-goal-on-adaptation')
  assert.equal(slugDe('ad ref.'), 'ad-ref')
  assert.equal(slugDe('PP / OP'), 'pp-op')
})

const entree = (id: string, term: string, extra: Partial<GlossaryEntry> = {}): GlossaryEntry => ({
  id,
  slug: slugDe(term),
  family_code: 'meetings',
  term,
  acronym: null,
  variants: [],
  translation: '',
  definition: '',
  heard_in_room: null,
  sources: [],
  related_ids: [],
  status: 'published',
  updated_at: '2026-09-24T08:00:00Z',
  ...extra,
})

const LEXIQUE = [
  entree('3', 'contact group', { variants: ['contact groups'] }),
  entree('2', 'global goal on adaptation', { acronym: 'GGA' }),
  entree('4', 'GGA', { variants: [] }),
  entree('5', 'loss and damage', { variants: ['L&D'] }),
  entree('6', 'L D'),
  entree('7', 'bracketed text', { slug: 'texte-entre-crochets' }),
]

test('résolution : le terme, puis le sigle, une variante, le slug', () => {
  assert.equal(resoudreLeTerme('Contact Group', LEXIQUE)?.id, '3')
  assert.equal(resoudreLeTerme('contact-groups', LEXIQUE)?.id, '3')
  // Un terme égal l'emporte sur un sigle égal.
  assert.equal(resoudreLeTerme('gga', LEXIQUE)?.id, '4')
  assert.equal(resoudreLeTerme('L D', LEXIQUE)?.id, '6')
  assert.equal(resoudreLeTerme('texte-entre-crochets', LEXIQUE)?.id, '7')
})

test('résolution : jamais d’approché', () => {
  assert.equal(resoudreLeTerme('contact grup', LEXIQUE), null)
  assert.equal(resoudreLeTerme('contact', LEXIQUE), null)
  assert.equal(resoudreLeTerme('  ', LEXIQUE), null)
})

test('regroupement par lettre, les chiffres et signes à la fin', () => {
  const groupes = grouperParLettre([...LEXIQUE, entree('9', '1/CP.21'), entree('8', 'Ægir')])
  assert.deepEqual(
    groupes.map((g) => g.lettre),
    ['A', 'B', 'C', 'G', 'L', '#'],
  )
  assert.deepEqual(
    groupes.find((g) => g.lettre === 'G')?.entrees.map((e) => e.id),
    ['4', '2'],
  )
  assert.equal(lettreDe('Écart'), 'E')
})
