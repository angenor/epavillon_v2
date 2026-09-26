import { test } from 'node:test'
import assert from 'node:assert/strict'
import type { FrancophoneMeeting, MyMeetingRegistrations } from '../../app/types/negotiation-meetings.ts'
import type { MyAgenda } from '../../app/types/negotiation-sessions.ts'
import {
  appliquerIntentionInscription,
  avecLaFile,
  boutonDInscription,
  etatDeLaReunion,
  inscriptionOuverte,
  ligneReunionsDuJour,
  ligneSessionsDuJour,
  prochaineReunion,
  reunionsDuJour,
  trierLesReunions,
} from '../../app/utils/guide-nego/reunions.ts'
import { BELEM, session } from './fausses-sessions.ts'

const a = (iso: string) => new Date(iso)

function reunion(id: string, start: string, end: string, extra: Partial<FrancophoneMeeting> = {}): FrancophoneMeeting {
  return {
    id,
    type: { code: 'preparatory_workshop', label: { fr: 'Atelier préparatoire', en: 'Preparatory workshop' } },
    title: { fr: `Réunion ${id}` },
    description: null,
    start_at: start,
    end_at: end,
    format: 'onsite',
    venue: 'Salle 3',
    has_video: false,
    organizer: 'IFDD',
    open_access: true,
    access_audience: null,
    requires_registration: true,
    capacity: null,
    registered_count: 0,
    waitlist_enabled: true,
    registration_opens_at: null,
    registration_closes_at: null,
    status: 'scheduled',
    cancellation_reason: null,
    pavilion_session_id: null,
    ...extra,
  }
}

const AVANT = a('2027-11-10T10:00:00Z')
const r = reunion('r', '2027-11-10T13:00:00Z', '2027-11-10T15:00:00Z')
const pleine = { capacity: 1, registered_count: 1 }
const inscrite = { status: 'registered' as const, waitlist_position: null }
const attente = { status: 'waitlisted' as const, waitlist_position: 2 }

test('une marque, par priorité : Annulée, Terminée, Inscrite, Liste d’attente, Complet, Prévue', () => {
  const annulee = reunion('x', r.start_at, r.end_at, { status: 'cancelled', ...pleine })
  assert.equal(etatDeLaReunion(annulee, inscrite, AVANT), 'annulee')
  assert.equal(etatDeLaReunion(r, inscrite, a('2027-11-10T15:00:00Z')), 'terminee')
  assert.equal(etatDeLaReunion(reunion('p', r.start_at, r.end_at, pleine), inscrite, AVANT), 'inscrite')
  assert.equal(etatDeLaReunion(reunion('p', r.start_at, r.end_at, pleine), attente, AVANT), 'liste-attente')
  assert.equal(etatDeLaReunion(reunion('p', r.start_at, r.end_at, pleine), null, AVANT), 'complet')
  assert.equal(etatDeLaReunion(r, null, AVANT), 'prevue')
})

test('pas d’« En cours » : commencée, elle reste Prévue jusqu’à sa fin', () => {
  assert.equal(etatDeLaReunion(r, null, a('2027-11-10T14:00:00Z')), 'prevue')
})

test('sans capacité, jamais Complet ; capacité abaissée sous les inscrites : Complet', () => {
  assert.equal(etatDeLaReunion(reunion('s', r.start_at, r.end_at, { registered_count: 400 }), null, AVANT), 'prevue')
  assert.equal(etatDeLaReunion(reunion('b', r.start_at, r.end_at, { capacity: 2, registered_count: 3 }), null, AVANT), 'complet')
})

test('tri par début, puis fin', () => {
  const x = reunion('x', '2027-11-11T13:00:00Z', '2027-11-11T14:00:00Z')
  const y = reunion('y', '2027-11-10T13:00:00Z', '2027-11-10T16:00:00Z')
  assert.deepEqual(trierLesReunions([x, y, r]).map((m) => m.id), ['r', 'y', 'x'])
})

test('le jour se compte dans le fuseau de la COP', () => {
  const tard = reunion('tard', '2027-11-11T01:30:00Z', '2027-11-11T02:30:00Z')
  assert.deepEqual(reunionsDuJour([r, tard], '2027-11-10', BELEM).map((m) => m.id), ['r', 'tard'])
  assert.deepEqual(reunionsDuJour([r, tard], '2027-11-11', BELEM), [])
})

test('la prochaine : à venir, jamais annulée', () => {
  const annulee = reunion('a', '2027-11-11T12:00:00Z', '2027-11-11T13:00:00Z', { status: 'cancelled' })
  const suite = reunion('s', '2027-11-12T12:00:00Z', '2027-11-12T13:00:00Z')
  assert.equal(prochaineReunion([suite, annulee, r], a('2027-11-10T16:00:00Z'))?.id, 's')
  assert.equal(prochaineReunion([r], a('2027-11-10T16:00:00Z')), null)
})

test('inscription ouverte : publiée, non commencée, demandée, dans la fenêtre', () => {
  assert.equal(inscriptionOuverte(r, AVANT), true)
  assert.equal(inscriptionOuverte(r, a('2027-11-10T13:00:00Z')), false, 'commencée')
  assert.equal(inscriptionOuverte(reunion('c', r.start_at, r.end_at, { status: 'cancelled' }), AVANT), false)
  assert.equal(inscriptionOuverte(reunion('n', r.start_at, r.end_at, { requires_registration: false }), AVANT), false)
  const fenetre = reunion('f', r.start_at, r.end_at, {
    registration_opens_at: '2027-11-10T09:00:00Z',
    registration_closes_at: '2027-11-10T11:00:00Z',
  })
  assert.equal(inscriptionOuverte(fenetre, a('2027-11-10T08:59:00Z')), false)
  assert.equal(inscriptionOuverte(fenetre, AVANT), true)
  assert.equal(inscriptionOuverte(fenetre, a('2027-11-10T11:00:00Z')), false, 'borne de fin exclue, comme la base')
})

test('le bouton : libellé et geste', () => {
  assert.deepEqual(boutonDInscription(r, null, AVANT), { libelle: 'inscrire', geste: 'inscrire', position: null })
  assert.deepEqual(boutonDInscription(r, inscrite, AVANT), { libelle: 'inscrite', geste: 'desinscrire', position: null })
  assert.deepEqual(boutonDInscription(r, attente, AVANT), { libelle: 'liste-attente', geste: 'desinscrire', position: 2 })
  assert.deepEqual(boutonDInscription(reunion('p', r.start_at, r.end_at, pleine), null, AVANT), {
    libelle: 'rejoindre-attente',
    geste: 'inscrire',
    position: null,
  })
  assert.deepEqual(boutonDInscription(reunion('p', r.start_at, r.end_at, { ...pleine, waitlist_enabled: false }), null, AVANT), {
    libelle: 'complet',
    geste: null,
    position: null,
  })
  assert.deepEqual(boutonDInscription(r, null, a('2027-11-10T14:00:00Z')), { libelle: 'closes', geste: null, position: null })
  assert.deepEqual(boutonDInscription(r, inscrite, a('2027-11-10T14:00:00Z')), { libelle: 'inscrite', geste: null, position: null })
})

test('rien pour une annulée, une terminée, une réunion sans inscription', () => {
  assert.equal(boutonDInscription(reunion('c', r.start_at, r.end_at, { status: 'cancelled' }), inscrite, AVANT), null)
  assert.equal(boutonDInscription(r, inscrite, a('2027-11-10T15:00:00Z')), null)
  assert.equal(boutonDInscription(reunion('n', r.start_at, r.end_at, { requires_registration: false }), null, AVANT), null)
})

const MES: MyMeetingRegistrations = {
  registrations: [{ meeting_id: 'r', status: 'registered', waitlist_position: null, client_ref: 'c0', registered_at: '2027-11-09T10:00:00Z' }],
  video: [{ meeting_id: 'r', url: 'https://visio.exemple/r' }],
}

test('le lien de visio part dès l’intention de désinscription (R6)', () => {
  const apres = appliquerIntentionInscription(MES, 'r', r, { inscrire: false, client_ref: null }, AVANT)
  assert.deepEqual(apres, { registrations: [], video: [] })
})

test('l’inscription se voit aussitôt ; pleine avec attente, elle se montre en attente', () => {
  const vide: MyMeetingRegistrations = { registrations: [], video: [] }
  const libre = appliquerIntentionInscription(vide, 'r', r, { inscrire: true, client_ref: 'c1' }, AVANT)
  assert.equal(libre.registrations[0]?.status, 'registered')
  assert.equal(libre.registrations[0]?.client_ref, 'c1')
  const p = reunion('p', r.start_at, r.end_at, pleine)
  assert.equal(appliquerIntentionInscription(vide, 'p', p, { inscrire: true, client_ref: 'c2' }, AVANT).registrations[0]?.status, 'waitlisted')
  assert.equal(appliquerIntentionInscription(MES, 'r', r, { inscrire: true, client_ref: 'c3' }, AVANT), MES, 'déjà inscrite : inchangée')
})

test('la file s’applique par-dessus la lecture', () => {
  const mes = avecLaFile(MES, { r: { inscrire: false, client_ref: null }, q: { inscrire: true, client_ref: 'c4' } }, [r], AVANT)
  assert.deepEqual(mes.registrations.map((i) => i.meeting_id), ['q'])
  assert.deepEqual(mes.video, [])
})

test('« Ma journée » — Sessions : mon agenda d’abord, sinon mes thématiques', () => {
  const s1 = session('s1', '2027-11-10T13:00:00Z', null, { theme: 'finance' })
  const s2 = session('s2', '2027-11-10T12:00:00Z', null, { theme: 'adaptation' })
  const demain = session('s3', '2027-11-11T12:00:00Z', null)
  const agenda: MyAgenda = { entries: [{ session_id: 's1', remind: false, added_at: '2027-11-01T00:00:00Z' }], network_entries: [] }
  const vide: MyAgenda = { entries: [], network_entries: [] }
  assert.deepEqual(ligneSessionsDuJour(agenda, [s1, s2, demain], [], AVANT, BELEM), { source: 'agenda', sessions: [s1] })
  assert.deepEqual(ligneSessionsDuJour(vide, [s1, s2, demain], ['adaptation'], AVANT, BELEM), { source: 'thematiques', sessions: [s2] })
  assert.deepEqual(ligneSessionsDuJour(vide, [s1, s2, demain], [], AVANT, BELEM), { source: 'aucune', sessions: [] })
})

test('« Ma journée » — Réunions : celles du jour, sinon la prochaine', () => {
  const demain = reunion('d', '2027-11-11T13:00:00Z', '2027-11-11T14:00:00Z')
  assert.deepEqual(ligneReunionsDuJour([demain, r], AVANT, BELEM), { reunions: [r], prochaine: null })
  assert.deepEqual(ligneReunionsDuJour([demain], AVANT, BELEM), { reunions: [], prochaine: demain })
  assert.deepEqual(ligneReunionsDuJour([], AVANT, BELEM), { reunions: [], prochaine: null })
})
