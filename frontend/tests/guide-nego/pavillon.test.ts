import { test } from 'node:test'
import assert from 'node:assert/strict'
import type { Registration, RegistrationFormField } from '../../app/types/programme/registration.ts'
import type { PublicScheduleRow } from '../../app/types/views.ts'
import type { Venue } from '../../app/types/event/venue.ts'
import {
  activitesDuJour,
  appliquerIntentionPavillon,
  avecLaFilePavillon,
  boutonDInscriptionPavillon,
  consentementRequis,
  etatDeLActivite,
  formulaireDUnGeste,
  inscriptionAAnnuler,
  inscriptionDeLaSeance,
  jourDuPavillonAOuvrir,
  joursDeLaBande,
  joursSuivants,
  lieuDuPavillon,
  lignePavillonDuJour,
  marqueDeLActivite,
  prochaineActivite,
  rediffusionsDeLaVeille,
  reponsesPreremplies,
  veille,
} from '../../app/utils/guide-nego/pavillon.ts'
import { BELEM } from './fausses-sessions.ts'

const a = (iso: string) => new Date(iso)

function activite(id: string, debut: string, fin: string, extra: Partial<PublicScheduleRow> = {}): PublicScheduleRow {
  return {
    id,
    event_id: 'cop31',
    event_day_id: null,
    proposal_id: null,
    slug: `activite-${id}`,
    title: { fr: `Activité ${id}` },
    summary: null,
    starts_at: debut,
    ends_at: fin,
    timezone: BELEM,
    format: 'in_person',
    status: 'scheduled',
    room_id: null,
    room_name: null,
    organization_id: null,
    organization_name: null,
    organization_acronym: null,
    organization_country_code: null,
    organization_country: null,
    is_streamed: false,
    broadcast_channel_id: null,
    capacity: null,
    tracks: [],
    cover: null,
    temporal_state: 'upcoming',
    registered_count: 0,
    theme_codes: [],
    themes: [],
    waitlist_enabled: false,
    registration_required: true,
    registration_opens_at: null,
    registration_closes_at: null,
    waitlisted_count: 0,
    listing_changed_at: null,
    language_codes: null,
    replay_url: null,
    replay_duration_seconds: null,
    ...extra,
  }
}

function inscription(id: string, session_id: string, extra: Partial<Registration> = {}): Registration {
  return {
    id,
    session_id,
    person_id: 'moi',
    organization_id: null,
    status: 'registered',
    answers: {},
    locale: 'fr',
    waitlist_position: null,
    joined_at: null,
    attendance_minutes: null,
    certificate_asset_id: null,
    source: 'web',
    cancelled_at: null,
    cancelled_reason: null,
    created_at: '2027-11-01T10:00:00Z',
    updated_at: '2027-11-01T10:00:00Z',
    ...extra,
  }
}

function champ(code: string, field_type: RegistrationFormField['field_type'], is_required: boolean, is_sensitive = false): RegistrationFormField {
  return {
    id: code,
    form_id: 'f',
    code,
    label: { fr: code },
    help_text: null,
    field_type,
    is_required,
    options: {},
    validation: {},
    is_sensitive,
    sort_order: 0,
    is_active: true,
  }
}

// Belém : UTC−3. Le 12 novembre à Belém court de 03:00Z le 12 à 03:00Z le 13.
const EDITION = [
  activite('hier', '2027-11-11T18:30:00Z', '2027-11-11T19:30:00Z', { replay_url: 'https://r/hier', replay_duration_seconds: 2820 }),
  activite('hier-sans', '2027-11-11T20:00:00Z', '2027-11-11T21:00:00Z'),
  activite('matin', '2027-11-12T12:30:00Z', '2027-11-12T13:30:00Z', { replay_url: 'https://r/matin', replay_duration_seconds: 3100 }),
  activite('midi', '2027-11-12T14:00:00Z', '2027-11-12T15:00:00Z'),
  activite('soir', '2027-11-13T01:30:00Z', '2027-11-13T02:30:00Z'),
  activite('demain', '2027-11-13T14:00:00Z', '2027-11-13T15:00:00Z'),
  activite('dernier', '2027-11-15T14:00:00Z', '2027-11-15T15:00:00Z'),
]
const MIDI = a('2027-11-12T14:30:00Z')

test('la bande couvre toute l’édition, jours passés et jours creux compris', () => {
  assert.deepEqual(joursDeLaBande(EDITION, BELEM), ['2027-11-11', '2027-11-12', '2027-11-13', '2027-11-14', '2027-11-15'])
  assert.equal(jourDuPavillonAOuvrir(joursDeLaBande(EDITION, BELEM), MIDI, BELEM), '2027-11-12')
  assert.deepEqual(joursDeLaBande([], BELEM), [])
})

test('le jour se compte dans le fuseau de la COP : 22:30 à Belém reste le 12', () => {
  assert.deepEqual(
    activitesDuJour(EDITION, '2027-11-12', BELEM).map((x) => x.id),
    ['matin', 'midi', 'soir'],
  )
})

test('la veille ne rend que ses rediffusions, et les jours suivants se groupent par jour', () => {
  assert.equal(veille('2027-12-01'), '2027-11-30')
  assert.deepEqual(rediffusionsDeLaVeille(EDITION, '2027-11-12', BELEM).map((x) => x.id), ['hier'])
  assert.deepEqual(
    joursSuivants(EDITION, '2027-11-12', BELEM).map((j) => [j.jour, j.activites.map((x) => x.id)]),
    [['2027-11-13', ['demain']], ['2027-11-15', ['dernier']]],
  )
})

test('aucun filtre de thématique : toutes les activités sortent, quelles que soient mes thématiques', () => {
  const themees = [
    activite('t1', '2027-11-12T12:00:00Z', '2027-11-12T13:00:00Z', { theme_codes: ['finance'] }),
    activite('t2', '2027-11-12T13:00:00Z', '2027-11-12T14:00:00Z', { theme_codes: ['adaptation'] }),
    activite('t3', '2027-11-12T15:00:00Z', '2027-11-12T16:00:00Z', { theme_codes: [] }),
  ]
  assert.equal(activitesDuJour(themees, '2027-11-12', BELEM).length, 3)
  assert.equal(lignePavillonDuJour(themees, MIDI, BELEM).activites.length, 3)
  assert.equal(joursSuivants(themees, '2027-11-11', BELEM)[0]?.activites.length, 3)
})

test('état de l’activité : annulée et reportée d’abord, puis l’heure', () => {
  const x = activite('x', '2027-11-12T14:00:00Z', '2027-11-12T15:00:00Z')
  assert.equal(etatDeLActivite(x, a('2027-11-12T13:00:00Z')), 'prevue')
  assert.equal(etatDeLActivite(x, MIDI), 'en-cours')
  assert.equal(etatDeLActivite(x, a('2027-11-12T15:00:00Z')), 'terminee')
  assert.equal(etatDeLActivite({ ...x, status: 'cancelled' }, MIDI), 'annulee')
  assert.equal(etatDeLActivite({ ...x, temporal_state: 'postponed' }, a('2027-11-12T13:00:00Z')), 'reportee')
  assert.equal(etatDeLActivite({ ...x, status: 'completed' }, a('2027-11-12T13:00:00Z')), 'terminee')
})

test('marque : rediffusion avec sa durée, inscription, liste d’attente, complet, sans inscription', () => {
  const matin = EDITION[2]!
  assert.deepEqual(marqueDeLActivite(matin, null, MIDI), { nom: 'rediffusion', url: 'https://r/matin', minutes: 52 })
  const midi = EDITION[3]!
  assert.deepEqual(marqueDeLActivite(midi, inscription('r', 'midi'), MIDI), { nom: 'inscrite' })
  const plus = activite('p', '2027-11-13T14:00:00Z', '2027-11-13T15:00:00Z', { capacity: 2, registered_count: 2 })
  assert.deepEqual(
    marqueDeLActivite(plus, inscription('r', 'p', { status: 'waitlisted', waitlist_position: 3 }), MIDI),
    { nom: 'liste-attente', position: 3 },
  )
  assert.deepEqual(marqueDeLActivite(plus, null, MIDI), { nom: 'complet' })
  assert.equal(marqueDeLActivite({ ...plus, waitlist_enabled: true }, null, MIDI), null)
  assert.deepEqual(marqueDeLActivite({ ...plus, registration_required: false }, null, MIDI), { nom: 'sans-inscription' })
  assert.equal(marqueDeLActivite({ ...plus, status: 'cancelled' }, inscription('r', 'p'), MIDI), null)
  assert.equal(marqueDeLActivite(EDITION[1]!, null, MIDI), null)
})

test('bouton : inscrire, rejoindre l’attente, complet, clos, pas encore ouvert, annuler', () => {
  const avant = a('2027-11-13T10:00:00Z')
  const x = activite('x', '2027-11-13T14:00:00Z', '2027-11-13T15:00:00Z')
  assert.deepEqual(boutonDInscriptionPavillon(x, null, avant), { libelle: 'inscrire', geste: 'inscrire', position: null, ouvreLe: null })
  assert.equal(boutonDInscriptionPavillon(x, inscription('r', 'x'), avant)?.geste, 'annuler')
  assert.deepEqual(
    boutonDInscriptionPavillon(x, inscription('r', 'x', { status: 'waitlisted', waitlist_position: 2 }), avant),
    { libelle: 'liste-attente', geste: 'annuler', position: 2, ouvreLe: null },
  )
  const pleine = { ...x, capacity: 1, registered_count: 1 }
  assert.equal(boutonDInscriptionPavillon({ ...pleine, waitlist_enabled: true }, null, avant)?.libelle, 'rejoindre-attente')
  assert.deepEqual(boutonDInscriptionPavillon(pleine, null, avant), { libelle: 'complet', geste: null, position: null, ouvreLe: null })
  assert.equal(boutonDInscriptionPavillon({ ...x, registration_closes_at: '2027-11-13T09:00:00Z' }, null, avant)?.libelle, 'closes')
  assert.deepEqual(boutonDInscriptionPavillon({ ...x, registration_opens_at: '2027-11-13T12:00:00Z' }, null, avant), {
    libelle: 'pas-encore',
    geste: null,
    position: null,
    ouvreLe: '2027-11-13T12:00:00Z',
  })
})

test('aucun geste sur une activité annulée, reportée, commencée ou sans inscription', () => {
  const avant = a('2027-11-13T10:00:00Z')
  const x = activite('x', '2027-11-13T14:00:00Z', '2027-11-13T15:00:00Z')
  assert.equal(boutonDInscriptionPavillon({ ...x, status: 'cancelled' }, inscription('r', 'x'), avant), null)
  assert.equal(boutonDInscriptionPavillon({ ...x, status: 'postponed' }, null, avant), null)
  assert.equal(boutonDInscriptionPavillon(x, null, a('2027-11-13T14:10:00Z')), null)
  assert.equal(boutonDInscriptionPavillon({ ...x, registration_required: false }, null, avant), null)
})

test('prochaine activité et ligne de « Ma journée »', () => {
  assert.equal(prochaineActivite(EDITION, MIDI)?.id, 'soir')
  assert.deepEqual(lignePavillonDuJour(EDITION, MIDI, BELEM).activites.map((x) => x.id), ['matin', 'midi', 'soir'])
  const creux = lignePavillonDuJour(EDITION, a('2027-11-14T15:00:00Z'), BELEM)
  assert.deepEqual([creux.activites.length, creux.prochaine?.id], [0, 'dernier'])
  const annulee = [activite('z', '2027-11-20T14:00:00Z', '2027-11-20T15:00:00Z', { status: 'cancelled' })]
  assert.equal(prochaineActivite(annulee, MIDI), null)
})

test('le formulaire d’un geste : aucun champ obligatoire, ou le seul pays connu du profil', () => {
  const simple = [champ('country', 'country', true), champ('job', 'text', false)]
  assert.equal(formulaireDUnGeste(simple, 'SN'), true)
  assert.equal(formulaireDUnGeste(simple, null), false)
  assert.equal(formulaireDUnGeste([champ('job', 'text', false)], null), true)
  assert.equal(formulaireDUnGeste([...simple, champ('badge', 'boolean', true)], 'SN'), false)
  assert.equal(formulaireDUnGeste([champ('country', 'country', true, true)], 'SN'), false)
  assert.deepEqual(reponsesPreremplies(simple, 'SN'), { country: 'SN' })
  assert.deepEqual(reponsesPreremplies(simple, null), {})
  assert.equal(consentementRequis([champ('sante', 'long_text', false, true)], { sante: '' }), false)
  assert.equal(consentementRequis([champ('sante', 'long_text', false, true)], { sante: 'fauteuil' }), true)
})

test('l’annulation vise la ligne vivante que le serveur connaît', () => {
  const mes = [
    inscription('ancienne', 's', { status: 'cancelled', created_at: '2027-11-02T10:00:00Z' }),
    inscription('vivante', 's', { created_at: '2027-11-01T10:00:00Z' }),
  ]
  assert.equal(inscriptionDeLaSeance(mes, 's')?.id, 'vivante')
  assert.equal(inscriptionAAnnuler(mes, 's')?.id, 'vivante')
  const locale = appliquerIntentionPavillon([], 's', null, { etat: 'inscrite', reponses: {}, consentement: false }, 'moi', MIDI)
  assert.equal(inscriptionDeLaSeance(locale, 's')?.status, 'registered')
  assert.equal(inscriptionAAnnuler(locale, 's'), null)
})

test('l’intention s’affiche aussitôt ; la dernière gagne ; inscrire puis annuler ne laisse rien', () => {
  const pleine = activite('p', '2027-11-13T14:00:00Z', '2027-11-13T15:00:00Z', { capacity: 1, registered_count: 1, waitlist_enabled: true })
  const vouloir = { etat: 'inscrite' as const, reponses: { country: 'SN' }, consentement: false }
  const apres = appliquerIntentionPavillon([], 'p', pleine, vouloir, 'moi', MIDI)
  assert.equal(apres[0]?.status, 'waitlisted')
  assert.deepEqual(apres[0]?.answers, { country: 'SN' })
  assert.deepEqual(appliquerIntentionPavillon(apres, 'p', pleine, { etat: 'annulee' }, 'moi', MIDI), [])
  const serveur = [inscription('r1', 'p')]
  const annulee = avecLaFilePavillon(serveur, { p: { etat: 'annulee' } }, [pleine], 'moi', MIDI)
  assert.equal(annulee[0]?.status, 'cancelled')
  assert.equal(inscriptionDeLaSeance(annulee, 'p'), null)
})

test('le lieu : le stand de l’édition, sinon le premier lieu', () => {
  const lieu = (id: string, kind: Venue['kind']): Venue => ({
    id,
    event_id: 'cop31',
    name: { fr: id },
    kind,
    address: null,
    map_url: null,
    created_at: '2027-01-01T00:00:00Z',
  })
  assert.equal(lieuDuPavillon([lieu('ligne', 'virtual'), lieu('stand', 'pavilion')])?.id, 'stand')
  assert.equal(lieuDuPavillon([lieu('ligne', 'virtual')])?.id, 'ligne')
  assert.equal(lieuDuPavillon([]), null)
})
