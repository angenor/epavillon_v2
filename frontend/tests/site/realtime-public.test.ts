import { test } from 'node:test'
import assert from 'node:assert/strict'
import { isPublicTopic, isValidTopic } from '../../app/utils/realtime.ts'
import { PROGRAMME_CHANNELS, deskTopic, publicEditionTopic, publicEditionTopics } from '../../app/utils/realtime-public.ts'

const A = '0198a1b2-0000-7000-8000-000000000001'
const B = '0198a1b2-0000-7000-8000-000000000002'

test('les sujets publics d’une édition passent par le pouls, le poste non', () => {
  for (const channel of ['live', 'schedule', 'incidents', 'questions'] as const) {
    const topic = publicEditionTopic(A, channel)
    assert.ok(isValidTopic(topic), topic)
    assert.ok(isPublicTopic(topic), topic)
  }
  assert.ok(isValidTopic(deskTopic(A)))
  assert.ok(!isPublicTopic(deskTopic(A)))
})

test('plusieurs éditions : dédoublonnées, absentes ignorées', () => {
  assert.deepEqual(publicEditionTopics([A, null, A, undefined, B], PROGRAMME_CHANNELS), [
    `edition:${A}:public:live`,
    `edition:${A}:public:schedule`,
    `edition:${B}:public:live`,
    `edition:${B}:public:schedule`,
  ])
  assert.deepEqual(publicEditionTopics([null], ['incidents']), [])
})
