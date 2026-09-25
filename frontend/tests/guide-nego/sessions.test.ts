import { test } from 'node:test'
import assert from 'node:assert/strict'
import type { OfficialSession, OfficialSessions } from '../../app/types/negotiation-sessions.ts'
import {
  affichageDesSessions,
  apresLecture,
  etatAffiche,
  etatVide,
  filtrer,
  jourAOuvrir,
  joursDeLaBande,
  passeLeFiltre,
  sessionsDuJour,
  trierParDebut,
} from '../../app/utils/guide-nego/sessions.ts'
import { BELEM, session } from './fausses-sessions.ts'

const coordination = (id: string, group: string | null) =>
  session(id, '2027-11-10T12:00:00Z', '2027-11-10T13:00:00Z', {
    type: { code: 'group_coordination', label: { fr: 'Coordination' }, term_en: 'Coordination meeting' },
    group: group ? { code: group, label: { fr: group } } : null,
  })

const a = (iso: string) => new Date(iso)

test('état affiché : prévue, en cours, terminée, déplacée, annulée', () => {
  const s = session('a', '2027-11-10T13:00:00Z', '2027-11-10T15:00:00Z')
  assert.equal(etatAffiche(s, a('2027-11-10T12:59:00Z'), BELEM), 'prevue')
  assert.equal(etatAffiche(s, a('2027-11-10T13:00:00Z'), BELEM), 'en-cours')
  assert.equal(etatAffiche(s, a('2027-11-10T15:00:00Z'), BELEM), 'terminee')

  const deplacee = { ...s, previous: { start_at: '2027-11-10T12:00:00Z', end_at: null, venue: null, changed_at: '2027-11-10T08:00:00Z' } }
  assert.equal(etatAffiche(deplacee, a('2027-11-10T12:00:00Z'), BELEM), 'deplacee')
  assert.equal(etatAffiche(deplacee, a('2027-11-10T14:00:00Z'), BELEM), 'en-cours', 'en cours, l’heure prime')

  const annulee = { ...s, status: 'cancelled' as const, cancelled: { at: '2027-11-10T08:00:00Z', reason: 'source' as const } }
  assert.equal(etatAffiche(annulee, a('2027-11-10T14:00:00Z'), BELEM), 'annulee')
})

test('fin absente : terminée à minuit du jour de la COP, pas du téléphone', () => {
  // 22:00 à Belém le 10 = 01:00 UTC le 11 ; minuit à Belém = 03:00 UTC le 11.
  const s = session('b', '2027-11-11T01:00:00Z', null)
  assert.equal(etatAffiche(s, a('2027-11-11T02:59:00Z'), BELEM), 'en-cours')
  assert.equal(etatAffiche(s, a('2027-11-11T03:00:00Z'), BELEM), 'terminee')
})

test('les jours de la bande se comptent dans le fuseau de la COP', () => {
  const tardive = session('t', '2027-11-11T01:00:00Z', null) // 22:00 le 10 à Belém
  const matin = session('m', '2027-11-11T12:00:00Z', null)
  assert.deepEqual(joursDeLaBande([matin, tardive], BELEM), ['2027-11-10', '2027-11-11'])
  assert.deepEqual(sessionsDuJour([matin, tardive], '2027-11-10', BELEM).map((s) => s.id), ['t'])
})

test('jour ouvert (FR-003) : aujourd’hui, sinon le prochain, sinon le dernier', () => {
  const jours = ['2027-11-10', '2027-11-12']
  assert.equal(jourAOuvrir(jours, a('2027-11-10T20:00:00Z'), BELEM), '2027-11-10')
  assert.equal(jourAOuvrir(jours, a('2027-11-11T12:00:00Z'), BELEM), '2027-11-12')
  assert.equal(jourAOuvrir(jours, a('2027-11-20T12:00:00Z'), BELEM), '2027-11-12')
  assert.equal(jourAOuvrir(jours, a('2027-11-01T12:00:00Z'), BELEM), '2027-11-10')
  // 23:30 le 11 à Belém = 02:30 UTC le 12 : c'est encore le 11 à la COP.
  assert.equal(jourAOuvrir(['2027-11-11', '2027-11-12'], a('2027-11-12T02:30:00Z'), BELEM), '2027-11-11')
  assert.equal(jourAOuvrir([], a('2027-11-10T12:00:00Z'), BELEM), null)
})

test('tri par début actuel (FR-005), l’annulée garde sa place', () => {
  const annulee = session('b', '2027-11-10T13:00:00Z', null, { status: 'cancelled' })
  const tri = trierParDebut([
    session('c', '2027-11-10T15:00:00Z', null),
    annulee,
    session('a', '2027-11-10T12:00:00Z', null),
  ])
  assert.deepEqual(tri.map((s) => s.id), ['a', 'b', 'c'])
})

test('« Mes thématiques » (FR-006) : thèmes suivis, sans thème, coordinations des groupes cochés', () => {
  const suivis = { thematiques: ['adaptation'], groupes: ['african_group'] }
  assert.equal(passeLeFiltre(session('1', '2027-11-10T12:00:00Z', null, { theme: 'adaptation' }), suivis), true)
  assert.equal(passeLeFiltre(session('2', '2027-11-10T12:00:00Z', null, { theme: 'finance' }), suivis), false)
  assert.equal(passeLeFiltre(session('3', '2027-11-10T12:00:00Z', null), suivis), true, 'thématique non précisée')
  assert.equal(passeLeFiltre(coordination('4', 'african_group'), suivis), true)
  assert.equal(passeLeFiltre(coordination('5', 'aosis'), suivis), false)
  assert.equal(passeLeFiltre(coordination('6', null), suivis), false, 'sans groupe : seulement sous « Toutes »')
})

test('aucun groupe coché : toutes les coordinations rattachées, jamais celle sans groupe', () => {
  const suivis = { thematiques: [], groupes: [] }
  const liste = [coordination('a', 'aosis'), coordination('b', 'lmdc'), coordination('c', null)]
  assert.deepEqual(filtrer(liste, 'miennes', suivis).map((s) => s.id), ['a', 'b'])
  assert.deepEqual(filtrer(liste, 'toutes', suivis).map((s) => s.id), ['a', 'b', 'c'])
})

test('état vide (FR-011) : le prochain créneau suivi et le nombre du jour', () => {
  const suivis = { thematiques: ['gender'], groupes: [] }
  const sessions = [
    session('f1', '2027-11-10T12:00:00Z', null, { theme: 'finance' }),
    session('f2', '2027-11-10T14:00:00Z', null, { theme: 'finance' }),
    session('g0', '2027-11-11T12:00:00Z', null, { theme: 'gender', status: 'cancelled' }),
    session('g1', '2027-11-12T12:00:00Z', null, { theme: 'gender' }),
  ]
  const vide = etatVide(sessions, '2027-11-10', suivis, a('2027-11-10T10:00:00Z'), BELEM)
  assert.equal(vide.prochain?.id, 'g1', 'l’annulée n’est pas un créneau')
  assert.equal(vide.autresDuJour, 2)
  assert.equal(etatVide(sessions, '2027-11-10', suivis, a('2027-11-13T10:00:00Z'), BELEM).prochain, null)
})

const lues = (state: 'serving' | 'cut', sessions: OfficialSession[]): OfficialSessions => ({
  edition: { slug: 'cop31', timezone: BELEM, city: 'Belém' },
  official_programme_url: 'https://unfccc.int/cop31/programme',
  state,
  cut_reason: state === 'cut' ? 'unreachable' : null,
  failing_since: state === 'cut' ? '2027-11-10T09:40:00Z' : null,
  read_at: '2027-11-10T09:00:00Z',
  server_time: '2027-11-10T12:00:00Z',
  sessions,
})

test('coupé (FR-039) : la réponse coupée remplace la copie, jamais une session à côté', () => {
  const avant = apresLecture(null, 'cop31', { lues: lues('serving', [session('a', '2027-11-10T12:00:00Z', null)]), empreinte: '"e1"' })
  assert.equal(affichageDesSessions(avant, 'cop31').etat, 'sert')

  // Même un corps coupé qui porterait des lignes ne les rend pas.
  const coupee = apresLecture(avant, 'cop31', { lues: lues('cut', [session('a', '2027-11-10T12:00:00Z', null)]), empreinte: '"e2"' })
  assert.deepEqual(coupee.lues?.sessions, [])
  const affiche = affichageDesSessions(coupee, 'cop31')
  assert.equal(affiche.etat, 'coupe')
  assert.ok(!('sessions' in affiche))
  if (affiche.etat === 'coupe') {
    assert.equal(affiche.raison, 'unreachable')
    assert.equal(affiche.depuis, '2027-11-10T09:40:00Z')
    assert.equal(affiche.programme, 'https://unfccc.int/cop31/programme')
  }

  // Un 304 sur une coupure reste coupé.
  assert.equal(affichageDesSessions(apresLecture(coupee, 'cop31', { inchange: true, empreinte: '"e2"' }), 'cop31').etat, 'coupe')
})

test('édition inconnue (404) : coupé, désactivé, sans lien', () => {
  const garde = apresLecture(null, 'cop31', { lues: null, empreinte: null })
  assert.deepEqual(affichageDesSessions(garde, 'cop31'), { etat: 'coupe', raison: 'disabled', depuis: null, programme: null, luA: null })
})

test('une garde d’une autre édition ne s’affiche pas ; un 304 sans garde lève', () => {
  const garde = apresLecture(null, 'cop30', { lues: lues('serving', []), empreinte: '"e"' })
  assert.equal(affichageDesSessions(garde, 'cop31').etat, 'inconnu')
  assert.throws(() => apresLecture(garde, 'cop31', { inchange: true, empreinte: '"e"' }))
})
