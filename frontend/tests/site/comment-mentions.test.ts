import { test } from 'node:test'
import assert from 'node:assert/strict'
import { activeMention, insertMention, matchMentionCandidates } from '../../app/utils/comment-mentions.ts'
import type { CommentPerson } from '../../app/types/programme/proposal.ts'

const people: CommentPerson[] = [
  { person_id: 'p1', display_name: 'Awa Bakayoko' },
  { person_id: 'p2', display_name: 'Hélène Duchesne' },
  { person_id: 'p3', display_name: 'Aline Rasoanaivo' },
]

test('une arobase en début de mot ouvre une mention jusqu’au curseur', () => {
  assert.deepEqual(activeMention('Bonjour @Ba', 11), { start: 8, end: 11, query: 'Ba' })
  assert.deepEqual(activeMention('@', 1), { start: 0, end: 1, query: '' })
  assert.deepEqual(activeMention('Voir (@hél', 10), { start: 6, end: 10, query: 'hél' })
})

test('une adresse électronique n’est pas une mention', () => {
  assert.equal(activeMention('contact@ifdd', 12), null)
})

test('un nom en deux mots reste une mention, une double espace ou un retour la ferment', () => {
  assert.equal(activeMention('@Awa Bak', 8)?.query, 'Awa Bak')
  assert.equal(activeMention('@Awa  Bak', 9), null)
  assert.equal(activeMention('@Awa\nBak', 8), null)
  assert.equal(activeMention('@ Awa', 5), null)
})

test('seul ce qui précède le curseur compte', () => {
  assert.equal(activeMention('@Awa et la suite', 4)?.query, 'Awa')
  assert.equal(activeMention('texte sans arobase', 5), null)
})

test('la recherche ignore accents et casse, et vise le début de chaque mot', () => {
  assert.deepEqual(matchMentionCandidates(people, 'hel').map((p) => p.person_id), ['p2'])
  assert.deepEqual(matchMentionCandidates(people, 'a bak').map((p) => p.person_id), ['p1'])
  assert.deepEqual(matchMentionCandidates(people, 'oyo'), [])
  assert.equal(matchMentionCandidates(people, '').length, 3)
  assert.equal(matchMentionCandidates(people, '', 2).length, 2)
})

test('retenir une personne remplace la saisie par son nom et place le curseur après', () => {
  const text = 'Merci @bak pour la relecture'
  const mention = activeMention(text, 10)
  assert.ok(mention)
  const result = insertMention(text, mention, 'Awa Bakayoko')
  assert.equal(result.text, 'Merci @Awa Bakayoko pour la relecture')
  assert.equal(result.caret, 'Merci @Awa Bakayoko '.length)
})
