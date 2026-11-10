import { test } from 'node:test'
import assert from 'node:assert/strict'
import { resourceKindOf, youtubeIdOf } from '../../app/utils/track-resources.ts'

test('une adresse YouTube donne sa vidéo, sous toutes ses formes', () => {
  for (const address of [
    'https://www.youtube.com/watch?v=4vkLwfSDAM8',
    'https://youtu.be/4vkLwfSDAM8?t=12',
    'https://m.youtube.com/watch?v=4vkLwfSDAM8&list=x',
    'https://www.youtube.com/shorts/4vkLwfSDAM8',
    'https://www.youtube.com/embed/4vkLwfSDAM8',
  ]) {
    assert.equal(youtubeIdOf(address), '4vkLwfSDAM8', address)
  }
  assert.equal(youtubeIdOf('https://www.youtube.com/@ifdd'), null)
  assert.equal(youtubeIdOf('https://www.youtube.com/watch?v=court'), null)
})

test('le genre se lit dans l’adresse : vidéo, PDF, page', () => {
  assert.deepEqual(resourceKindOf('https://www.youtube.com/watch?v=4vkLwfSDAM8'), {
    kind: 'video',
    videoId: '4vkLwfSDAM8',
    site: 'youtube.com',
  })
  const pdf = resourceKindOf('https://www.ifdd.francophonie.org/wp-content/uploads/2026/11/1.-DOSSIER.PDF')
  assert.equal(pdf.kind, 'pdf')
  assert.equal(pdf.site, 'ifdd.francophonie.org')
  assert.equal(resourceKindOf('https://www.ifdd.francophonie.org/appel-a-manifestation').kind, 'page')
  assert.deepEqual(resourceKindOf('/guide-nego'), { kind: 'page', videoId: null, site: null })
  assert.equal(resourceKindOf('/docs/programme.pdf?v=2').kind, 'pdf')
})
