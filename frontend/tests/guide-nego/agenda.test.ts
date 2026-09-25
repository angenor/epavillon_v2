import { test } from 'node:test'
import assert from 'node:assert/strict'
import {
  appliquerIntention,
  avecLaFile,
  chevauchements,
  prochaineSession,
  rappelsDus,
  serecouvrent,
} from '../../app/utils/guide-nego/agenda.ts'
import { BELEM, session } from './fausses-sessions.ts'

const a = (iso: string) => new Date(iso)
const annulee = { status: 'cancelled' as const, cancelled: { at: '2027-11-10T08:00:00Z', reason: 'source' as const } }

test('chevauchements (FR-033) : chacune porte l’autre', () => {
  const x = session('x', '2027-11-10T13:00:00Z', '2027-11-10T15:00:00Z')
  const y = session('y', '2027-11-10T14:00:00Z', '2027-11-10T16:00:00Z')
  const z = session('z', '2027-11-10T15:00:00Z', '2027-11-10T16:00:00Z')
  const liste = chevauchements([x, y, z])
  assert.deepEqual(liste.get('x')?.map((s) => s.id), ['y'])
  assert.deepEqual(liste.get('y')?.map((s) => s.id), ['x', 'z'])
  assert.deepEqual(liste.get('z')?.map((s) => s.id), ['y'], 'bout à bout ne chevauche pas x')
})

test('une annulée n’en cause ni n’en porte', () => {
  const x = session('x', '2027-11-10T13:00:00Z', '2027-11-10T15:00:00Z')
  const y = session('y', '2027-11-10T14:00:00Z', '2027-11-10T16:00:00Z', annulee)
  const liste = chevauchements([x, y])
  assert.equal(liste.size, 0)
})

test('fin absente : la session vaut son début, sans durée inventée', () => {
  const longue = session('l', '2027-11-10T13:00:00Z', '2027-11-10T15:00:00Z')
  assert.equal(serecouvrent(session('p', '2027-11-10T14:00:00Z', null), longue), true, 'commence pendant l’autre')
  assert.equal(serecouvrent(longue, session('p', '2027-11-10T14:00:00Z', null)), true, 'symétrique')
  assert.equal(serecouvrent(session('p', '2027-11-10T12:00:00Z', null), longue), false, 'avant l’autre')
  assert.equal(serecouvrent(session('p', '2027-11-10T15:00:00Z', null), longue), false, 'à sa fin')
  assert.equal(serecouvrent(session('p', '2027-11-10T13:00:00Z', null), session('q', '2027-11-10T13:00:00Z', null)), true)
  assert.equal(serecouvrent(session('p', '2027-11-10T13:00:00Z', null), session('q', '2027-11-10T13:30:00Z', null)), false)
})

test('prochaine session (FR-036) : l’agenda, ni terminée ni annulée ; une en cours compte', () => {
  const maintenant = a('2027-11-10T14:00:00Z')
  const passee = session('passee', '2027-11-10T10:00:00Z', '2027-11-10T11:00:00Z')
  const enCours = session('en-cours', '2027-11-10T13:30:00Z', '2027-11-10T15:00:00Z')
  const plusTard = session('plus-tard', '2027-11-10T16:00:00Z', null)
  const annuleeTot = session('annulee', '2027-11-10T13:00:00Z', null, annulee)
  const suivie = session('suivie', '2027-11-10T14:30:00Z', null, { theme: 'gender' })

  assert.deepEqual(
    prochaineSession([passee, annuleeTot, plusTard, enCours], [suivie], ['gender'], maintenant, BELEM),
    { session: enCours, source: 'agenda' },
  )
})

test('sans agenda utile : la prochaine des thématiques suivies, sinon rien', () => {
  const maintenant = a('2027-11-10T14:00:00Z')
  const autre = session('autre', '2027-11-10T14:10:00Z', null, { theme: 'finance' })
  const sansTheme = session('sans', '2027-11-10T14:15:00Z', null)
  const suivie = session('suivie', '2027-11-10T14:30:00Z', null, { theme: 'gender' })
  const passee = session('passee', '2027-11-10T10:00:00Z', '2027-11-10T11:00:00Z')
  assert.deepEqual(prochaineSession([passee], [autre, sansTheme, suivie], ['gender'], maintenant, BELEM), {
    session: suivie,
    source: 'thematiques',
  })
  assert.equal(prochaineSession([], [autre], ['gender'], maintenant, BELEM), null)
})

test('rappel dû (FR-034) : dans les quinze minutes avant le début, jamais sur une annulée', () => {
  const s = session('s', '2027-11-10T14:00:00Z', null)
  const agenda = { entries: [{ session_id: 's', remind: true, added_at: '2027-11-09T10:00:00Z' }] }
  assert.deepEqual(rappelsDus(agenda, [s], a('2027-11-10T13:44:59Z')), [])
  assert.deepEqual(rappelsDus(agenda, [s], a('2027-11-10T13:45:00Z')).map((x) => x.id), ['s'])
  assert.deepEqual(rappelsDus(agenda, [s], a('2027-11-10T13:59:59Z')).map((x) => x.id), ['s'])
  assert.deepEqual(rappelsDus(agenda, [s], a('2027-11-10T14:00:00Z')), [], 'au début, plus de rappel')
  assert.deepEqual(rappelsDus(agenda, [{ ...s, ...annulee }], a('2027-11-10T13:50:00Z')), [])
  const desarme = { entries: [{ session_id: 's', remind: false, added_at: '2027-11-09T10:00:00Z' }] }
  assert.deepEqual(rappelsDus(desarme, [s], a('2027-11-10T13:50:00Z')), [])
})

test('une intention s’affiche aussitôt ; la dernière sur une session gagne', () => {
  const maintenant = a('2027-11-10T10:00:00Z')
  const lu = { entries: [{ session_id: 'a', remind: false, added_at: '2027-11-09T10:00:00Z' }] }

  const rappel = appliquerIntention(lu, 'a', { garder: true, remind: true }, maintenant)
  assert.deepEqual(rappel.entries, [{ session_id: 'a', remind: true, added_at: '2027-11-09T10:00:00Z' }], 'l’heure d’ajout ne bouge pas')

  assert.deepEqual(appliquerIntention(lu, 'a', { garder: false, remind: false }, maintenant).entries, [])

  const vue = avecLaFile(lu, { b: { garder: true, remind: false }, a: { garder: false, remind: false } }, maintenant)
  assert.deepEqual(vue.entries.map((e) => e.session_id), ['b'])
})
