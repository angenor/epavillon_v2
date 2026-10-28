import { test } from 'node:test'
import assert from 'node:assert/strict'
import { groupThreads, summarizeReceipts, unreadCommentMap } from '../../app/utils/comment-reads.ts'
import type { CommentReader } from '../../app/types/programme/proposal.ts'

function reader(person: string, at: string, side: CommentReader['side'] = 'committee'): CommentReader {
  return { person_id: person, display_name: person, read_at: at, side }
}

test('« lu par N sur M » compte une fois chaque membre du comité, du premier au dernier', () => {
  const summary = summarizeReceipts({
    audience_count: 3,
    readers: [
      reader('b', '2026-08-07T10:00:00Z'),
      reader('a', '2026-08-07T09:00:00Z'),
      reader('b', '2026-08-06T08:00:00Z'),
    ],
  })
  assert.ok(summary)
  assert.equal(summary.audience, 3)
  assert.deepEqual(summary.committee.map((r) => [r.person_id, r.read_at]), [
    ['b', '2026-08-06T08:00:00Z'],
    ['a', '2026-08-07T09:00:00Z'],
  ])
  assert.equal(summary.organisation, null)
})

test('la lecture de l’organisation se compte à part, à sa première heure', () => {
  const summary = summarizeReceipts({
    audience_count: 2,
    readers: [
      reader('org2', '2026-08-08T10:00:00Z', 'organisation'),
      reader('org1', '2026-08-07T10:40:00Z', 'organisation'),
      reader('a', '2026-08-07T09:00:00Z'),
    ],
  })
  assert.equal(summary?.committee.length, 1)
  assert.equal(summary?.organisation?.person_id, 'org1')
})

test('le dénominateur ne descend jamais sous le nombre de lecteurs', () => {
  const summary = summarizeReceipts({
    audience_count: 1,
    readers: [reader('a', '2026-08-07T09:00:00Z'), reader('b', '2026-08-07T10:00:00Z')],
  })
  assert.equal(summary?.audience, 2)
  assert.equal(summarizeReceipts(null), null)
})

test('les compteurs de la liste écartent les zéros et additionnent les doublons', () => {
  const map = unreadCommentMap([
    { proposal_id: 'x', count: 2 },
    { proposal_id: 'y', count: 0 },
    { proposal_id: 'x', count: 1 },
  ])
  assert.deepEqual([...map], [['x', 3]])
  assert.equal(unreadCommentMap(undefined).size, 0)
})

test('une réponse à une réponse reste dans le fil de sa racine, dans l’ordre d’écriture', () => {
  const threads = groupThreads([
    { id: 'r2', parent_id: 'r1', created_at: '2026-08-03T00:00:00Z' },
    { id: 'root', parent_id: null, created_at: '2026-08-01T00:00:00Z' },
    { id: 'r1', parent_id: 'root', created_at: '2026-08-02T00:00:00Z' },
    { id: 'other', parent_id: null, created_at: '2026-08-04T00:00:00Z' },
  ])
  assert.deepEqual(
    threads.map((thread) => [thread.root.id, thread.replies.map((reply) => reply.id)]),
    [
      ['root', ['r1', 'r2']],
      ['other', []],
    ],
  )
})
