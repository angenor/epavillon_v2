import { test } from 'node:test'
import assert from 'node:assert/strict'
import {
  awaitZoomLink,
  blankOnlineSpeaker,
  cleanOnlineSpeaker,
  onlineSpeakerProblems,
  speakerLinkHeaders,
} from '../../app/utils/speaker-links.ts'

const notReady = Object.assign(new Error('pas prête'), { code: 'LIVE_MEETING_NOT_READY', status: 409 })
const unavailable = Object.assign(new Error('Zoom muet'), { code: 'LIVE_ZOOM_UNAVAILABLE', status: 503 })
const refused = Object.assign(new Error('refus'), { code: 'NOT_FOUND', status: 404 })

const flush = () => new Promise((resolve) => setImmediate(resolve))

test("le lien arrive au troisième essai, trois secondes d'écart", async (t) => {
  t.mock.timers.enable({ apis: ['setTimeout', 'Date'] })
  let calls = 0
  const pending = awaitZoomLink(async () => {
    calls += 1
    if (calls < 3) throw notReady
    return { join_url: 'https://zoom.example/w/1' }
  })
  await flush()
  assert.equal(calls, 1)
  t.mock.timers.tick(2_999)
  await flush()
  assert.equal(calls, 1)
  t.mock.timers.tick(1)
  await flush()
  assert.equal(calls, 2)
  t.mock.timers.tick(3_000)
  assert.deepEqual(await pending, { join_url: 'https://zoom.example/w/1' })
  assert.equal(calls, 3)
})

test("au-delà d'une minute, l'attente rend nul sans erreur", async (t) => {
  t.mock.timers.enable({ apis: ['setTimeout', 'Date'] })
  let calls = 0
  const pending = awaitZoomLink(async () => {
    calls += 1
    throw notReady
  })
  for (let i = 0; i < 25; i += 1) {
    await flush()
    t.mock.timers.tick(3_000)
  }
  assert.equal(await pending, null)
  assert.equal(calls, 21)
})

test('Zoom injoignable : nul aussitôt, sans réessai', async () => {
  let calls = 0
  const result = await awaitZoomLink(async () => {
    calls += 1
    throw unavailable
  })
  assert.equal(result, null)
  assert.equal(calls, 1)
})

test('un autre refus remonte tel quel', async () => {
  await assert.rejects(awaitZoomLink(async () => Promise.reject(refused)), refused)
})

test("une fenêtre fermée arrête l'attente", async (t) => {
  t.mock.timers.enable({ apis: ['setTimeout', 'Date'] })
  const controller = new AbortController()
  let calls = 0
  const pending = awaitZoomLink(
    async () => {
      calls += 1
      throw notReady
    },
    { signal: controller.signal },
  )
  await flush()
  controller.abort()
  t.mock.timers.tick(3_000)
  assert.equal(await pending, null)
  assert.equal(calls, 1)
})

test('le formulaire exige nom, fonction, organisation et une adresse bien formée', () => {
  const blank = blankOnlineSpeaker()
  assert.deepEqual(Object.keys(onlineSpeakerProblems(blank)).sort(), ['email', 'first_name', 'job_title', 'last_name', 'organization'])
  const filled = { ...blank, first_name: 'Fatou', last_name: 'Ndiaye', job_title: 'Cheffe', organization: 'Exemple', email: 'fatou@exemple' }
  assert.deepEqual(onlineSpeakerProblems(filled), { email: 'email' })
  assert.deepEqual(onlineSpeakerProblems({ ...filled, email: ' fatou@exemple.org ' }), {})
})

test('la saisie part nettoyée, présentation vide comprise', () => {
  const input = { ...blankOnlineSpeaker(), first_name: ' Fatou ', email: ' f@e.org ', bio: '   ' }
  const clean = cleanOnlineSpeaker(input)
  assert.equal(clean.first_name, 'Fatou')
  assert.equal(clean.email, 'f@e.org')
  assert.equal(clean.bio, null)
})

test("le jeton part en en-tête, et rien sans jeton", () => {
  assert.deepEqual(speakerLinkHeaders('abc'), { 'X-Speaker-Link': 'abc' })
  assert.equal(speakerLinkHeaders(null), undefined)
})
