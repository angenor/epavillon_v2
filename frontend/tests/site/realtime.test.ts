import { test } from 'node:test'
import assert from 'node:assert/strict'
import { backoffMs, changedTopics, isPublicTopic, isValidTopic, splitSse } from '../../app/utils/realtime.ts'

const ID = '0198a1b2-0000-7000-8000-000000000001'

test('un sujet suit la grammaire de platform.signal()', () => {
  assert.ok(isValidTopic(`proposal:${ID}`))
  assert.ok(isValidTopic(`event:${ID}:mailings`))
  assert.ok(!isValidTopic('proposal'))
  assert.ok(!isValidTopic(`Proposal:${ID}`))
  assert.ok(!isValidTopic(`proposal:${ID.toUpperCase()}`))
})

test("seuls les sujets publics d'une édition passent par le pouls public", () => {
  assert.ok(isPublicTopic(`edition:${ID}:public`))
  assert.ok(isPublicTopic(`edition:${ID}:public:live`))
  assert.ok(!isPublicTopic(`edition:${ID}:publication`))
  assert.ok(!isPublicTopic(`proposal:${ID}`))
})

test('le flux se découpe en événements, battements et retry ignorés, reste gardé', () => {
  const { events, rest } = splitSse(
    'retry: 3000\n\nevent: ready\ndata: {"boot":"b","versions":{},"denied":[]}\n\n: battement\n\nevent: signal\ndata: {"topics":["a:b"]}\n\nevent: sig',
  )
  assert.deepEqual(events, [
    { name: 'ready', data: '{"boot":"b","versions":{},"denied":[]}' },
    { name: 'signal', data: '{"topics":["a:b"]}' },
  ])
  assert.equal(rest, 'event: sig')
})

test("l'attente croît jusqu'à 30 s, gigue comprise", () => {
  assert.equal(backoffMs(1, () => 0), 250)
  assert.equal(backoffMs(1, () => 0.999), 250 + 1998)
  assert.equal(backoffMs(10, () => 0.999), 250 + 29_970)
})

test('le pouls relit ce qui a bougé, tout après un redémarrage, rien à la première lecture', () => {
  const topics = ['a:1', 'b:2', 'c:3']
  const known = { boot: 'b1', versions: new Map([['a:1', 1], ['b:2', 4]]) }

  assert.deepEqual(changedTopics(topics, known, { boot: 'b1', versions: { 'a:1': 2, 'b:2': 4, 'c:3': 0 } }), ['a:1'])
  assert.deepEqual(changedTopics(topics, known, { boot: 'b2', versions: { 'a:1': 0, 'b:2': 0, 'c:3': 0 } }), topics)
  assert.deepEqual(changedTopics(topics, { boot: null, versions: new Map() }, { boot: 'b1', versions: { 'a:1': 5 } }), [])
  assert.deepEqual(
    changedTopics(topics, known, { boot: 'b1', versions: { 'a:1': 1, 'b:2': 4, 'c:3': 0 } }, new Set(['c:3'])),
    ['c:3'],
  )
})
