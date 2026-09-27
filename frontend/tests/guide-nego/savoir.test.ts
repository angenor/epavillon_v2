import { test } from 'node:test'
import assert from 'node:assert/strict'
import type { FaqEntry, GlossaryEntry, KnowledgeBundle } from '../../app/types/negotiation-savoir.ts'
import { fusionner, lisible } from '../../app/utils/guide-nego/savoir.ts'

const QUAND = '2026-09-24T08:00:00Z'

const question = (id: string, section: string, related: string[] = []): FaqEntry => ({
  id,
  section_code: section,
  question: `Question ${id}`,
  answer: `Réponse ${id}`,
  status: 'published',
  verified_on: '2026-09-24',
  sources: [],
  related_ids: related,
  updated_at: QUAND,
})

const terme = (id: string, term: string, related: string[] = []): GlossaryEntry => ({
  id,
  slug: term.replace(/ /g, '-'),
  family_code: 'meetings',
  term,
  acronym: null,
  variants: [],
  translation: '',
  definition: `Définition de ${term}`,
  heard_in_room: null,
  sources: [],
  related_ids: related,
  status: 'published',
  updated_at: QUAND,
})

const paquet = (partiel: Partial<KnowledgeBundle>): KnowledgeBundle => ({
  served_at: QUAND,
  complete: true,
  faq_sections: [
    { code: 'first_cop', label: 'Ma première COP', icon: 'star', sort_order: 10 },
    { code: 'process', label: 'Le processus', icon: 'toc', sort_order: 20 },
  ],
  glossary_families: [{ code: 'meetings', label: 'Réunions', sort_order: 10 }],
  faq: [],
  glossary: [],
  pathway: { groups: [] },
  most_read: [],
  removed: { faq: [], glossary: [] },
  ...partiel,
})

const GARDE = paquet({
  faq: [question('f1', 'first_cop'), question('f2', 'process', ['f1', 'f9'])],
  glossary: [terme('a1', 'contact group', ['a2', 'a3']), terme('a2', 'informal consultations')],
  most_read: ['f2'],
})

test('une lecture entière remplace la garde', () => {
  const entiere = paquet({ served_at: '2026-09-25T08:00:00Z', faq: [question('f3', 'process')] })
  const fusionne = fusionner(GARDE, entiere)
  assert.deepEqual(
    fusionne.faq.map((e) => e.id),
    ['f3'],
  )
  assert.equal(fusionne.served_at, '2026-09-25T08:00:00Z')
})

test('une différence remplace, ajoute, retire, et remplace le reste en entier', () => {
  const difference = paquet({
    served_at: '2026-09-25T08:00:00Z',
    complete: false,
    faq: [{ ...question('f1', 'first_cop'), answer: 'Corrigée' }, question('f4', 'first_cop')],
    glossary: [terme('a0', 'ad ref.')],
    most_read: ['f4'],
    pathway: { groups: [{ id: 'g1', label: 'Avant de partir', sort_order: 10, steps: [] }] },
    removed: { faq: ['f2'], glossary: ['a2'] },
  })
  const fusionne = fusionner(GARDE, difference)
  assert.equal(fusionne.complete, true)
  assert.equal(fusionne.served_at, '2026-09-25T08:00:00Z')
  assert.deepEqual(
    fusionne.faq.map((e) => [e.id, e.answer]),
    [
      ['f1', 'Corrigée'],
      ['f4', 'Réponse f4'],
    ],
  )
  // Par terme normalisé, comme le serveur.
  assert.deepEqual(
    fusionne.glossary.map((e) => e.id),
    ['a0', 'a1'],
  )
  assert.deepEqual(fusionne.most_read, ['f4'])
  assert.equal(fusionne.pathway.groups.length, 1)
  assert.deepEqual(fusionne.removed, { faq: [], glossary: [] })
})

test('fusionner deux fois la même différence ne change rien : le chevauchement ne coûte rien', () => {
  const difference = paquet({ complete: false, faq: [question('f4', 'first_cop')] })
  const une = fusionner(GARDE, difference)
  assert.deepEqual(fusionner(une, difference), une)
})

test('la FAQ suit l’ordre des rubriques', () => {
  const difference = paquet({ complete: false, faq: [question('f5', 'first_cop')] })
  assert.deepEqual(
    fusionner(GARDE, difference).faq.map((e) => e.id),
    ['f1', 'f5', 'f2'],
  )
})

test('les liens vers une entrée absente tombent à la lecture, et reviennent quand elle est publiée', () => {
  const lu = lisible(GARDE)
  assert.deepEqual(lu.faq.find((e) => e.id === 'f2')?.related_ids, ['f1'])
  assert.deepEqual(lu.glossary.find((e) => e.id === 'a1')?.related_ids, ['a2'])

  // a3 est publiée : la différence ne renvoie pas a1, dont le lien était gardé entier.
  const difference = paquet({ complete: false, glossary: [terme('a3', 'informal informals')] })
  const apres = lisible(fusionner(GARDE, difference))
  assert.deepEqual(apres.glossary.find((e) => e.id === 'a1')?.related_ids, ['a2', 'a3'])
})

test('« les plus lues » ne citent que des entrées présentes', () => {
  assert.deepEqual(lisible(paquet({ most_read: ['f1', 'f2'], faq: [question('f2', 'process')] })).most_read, ['f2'])
})
