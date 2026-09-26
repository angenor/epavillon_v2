import { test } from 'node:test'
import assert from 'node:assert/strict'
import type { Notification } from '../../app/types/engagement.ts'
import type { MyReport, ReportQueueItem } from '../../app/types/negotiation-reports.ts'
import type { NetworkMeeting, NetworkReport } from '../../app/types/negotiation-sessions.ts'
import {
  apresDecision,
  corpsDeLaReunion,
  compteurDeCloche,
  corpsDuChangement,
  dessinDeNotification,
  avecLesLectures,
  debutDeTri,
  encartsAffiches,
  etatDuTraite,
  finDuJour,
  idsNonLues,
  joursAvecReunions,
  lignesDesSignalements,
  lignesDuJour,
  notificationsParJour,
  referenceClient,
  repereSignale,
  reunionPasseLeFiltre,
  reunionsDeLAgenda,
  reunionsDuJour,
  reunionsParJour,
  signalementEnAttente,
  signalementEnCours,
  texteDeLEtat,
} from '../../app/utils/guide-nego/signalements.ts'
import { BELEM, session } from './fausses-sessions.ts'

const a = (iso: string) => new Date(iso)

const encart = (validated_at: string, reason: NetworkReport['reason'] = 'venue'): NetworkReport => ({
  reason,
  proposed_start: null,
  proposed_venue: 'Room Negro',
  detail: null,
  validated_at,
})

const reunion = (id: string, day: string, start_at: string | null): NetworkMeeting => ({
  id,
  title: `Réunion ${id}`,
  venue: null,
  start_at,
  day,
  theme: null,
  validated_at: '2027-11-10T12:00:00Z',
})

const rapport = (client_ref: string, submitted_at: string, extra: Partial<MyReport> = {}): MyReport => ({
  id: `id-${client_ref}`,
  client_ref,
  reason: 'time',
  session: { id: 's1', title_en: 'S1', title_fr: null, start_at: '2027-11-10T13:00:00Z', venue: null },
  what: null,
  proposed_start: null,
  proposed_venue: null,
  day: null,
  theme: null,
  network_meeting_id: null,
  detail: null,
  status: 'submitted',
  submitted_at,
  decided_at: null,
  reject_reason: null,
  reject_detail: null,
  ...extra,
})

test('encart : affiché tant que la session n’est pas terminée, le plus récent d’abord', () => {
  const s = session('s', '2027-11-10T13:00:00Z', '2027-11-10T15:00:00Z', {
    network_reports: [encart('2027-11-10T10:00:00Z'), encart('2027-11-10T11:00:00Z', 'other')],
  })
  const avant = encartsAffiches(s, a('2027-11-10T14:59:00Z'), BELEM)
  assert.deepEqual(avant.map((e) => e.reason), ['other', 'venue'])
  assert.deepEqual(encartsAffiches(s, a('2027-11-10T15:00:00Z'), BELEM), [], '« Autre chose » compris (FR-020)')
})

test('encart : sans fin, il tombe à minuit du jour de la COP', () => {
  const s = session('s', '2027-11-10T23:00:00Z', null, { network_reports: [encart('2027-11-10T10:00:00Z')] })
  // 23:00 UTC = 20:00 à Belém ; minuit à Belém = 03:00 UTC le lendemain.
  assert.equal(encartsAffiches(s, a('2027-11-11T02:59:00Z'), BELEM).length, 1)
  assert.equal(encartsAffiches(s, a('2027-11-11T03:00:00Z'), BELEM).length, 0)
})

test('repère : l’heure de validation dans le fuseau de la COP, nul sans encart', () => {
  const s = session('s', '2027-11-10T13:00:00Z', '2027-11-10T15:00:00Z', {
    network_reports: [encart('2027-11-10T12:05:00Z')],
  })
  assert.deepEqual(repereSignale(s, a('2027-11-10T12:30:00Z'), BELEM), { validation: '2027-11-10T12:05:00Z', heure: '09:05' })
  assert.equal(repereSignale(session('t', '2027-11-10T13:00:00Z', null), a('2027-11-10T12:30:00Z'), BELEM), null)
  assert.equal(repereSignale(s, a('2027-11-10T16:00:00Z'), BELEM), null, 'terminée : plus de repère')
})

test('réunion non annoncée : fin du jour dans le fuseau de la COP, pas du téléphone', () => {
  assert.equal(finDuJour('2027-11-10', BELEM), Date.parse('2027-11-11T03:00:00Z'))
  const r = reunion('r', '2027-11-10', null)
  assert.deepEqual(reunionsDuJour([r], '2027-11-10', a('2027-11-11T02:59:00Z'), BELEM), [r])
  assert.deepEqual(reunionsDuJour([r], '2027-11-10', a('2027-11-11T03:00:00Z'), BELEM), [], 'jour passé : retirée d’office')
})

test('réunion sans heure : rangée en fin de journée', () => {
  const sansHeure = reunion('a', '2027-11-10', null)
  const tardive = reunion('b', '2027-11-10', '2027-11-11T02:30:00Z')
  assert.ok(debutDeTri(sansHeure, BELEM) > debutDeTri(tardive, BELEM))
  const lignes = lignesDuJour(
    [session('s', '2027-11-10T13:00:00Z', null)],
    [sansHeure, tardive, reunion('c', '2027-11-10', '2027-11-10T12:00:00Z')],
    '2027-11-10',
    a('2027-11-10T08:00:00Z'),
    BELEM,
  )
  assert.deepEqual(
    lignes.map((l) => (l.genre === 'session' ? l.session.id : l.reunion.id)),
    ['c', 's', 'b', 'a'],
  )
})

test('la bande gagne le jour d’une réunion non annoncée, pas celui d’une réunion passée', () => {
  const jours = joursAvecReunions(
    ['2027-11-10'],
    [reunion('a', '2027-11-12', null), reunion('b', '2027-11-09', null)],
    a('2027-11-10T12:00:00Z'),
    BELEM,
  )
  assert.deepEqual(jours, ['2027-11-10', '2027-11-12'])
})

test('mes signalements : l’envoi en file paraît aussitôt, une seule fois', () => {
  const enFile = signalementEnAttente(
    { corps: { client_ref: 'c2', edition: 'cop', reason: 'cancelled', session_id: 's1' }, prise_a: '2027-11-10T12:00:00Z' },
    null,
  )
  const lus = [rapport('c1', '2027-11-10T10:00:00Z', { status: 'rejected', reject_reason: 'already_known' })]
  const lignes = lignesDesSignalements(lus, [enFile])
  assert.deepEqual(lignes.map((l) => l.etat), ['en-attente', 'non-retenu'])

  const recu = rapport('c2', '2027-11-10T12:00:00Z')
  assert.deepEqual(
    lignesDesSignalements([recu, ...lus], [enFile]).map((l) => l.etat),
    ['envoye', 'non-retenu'],
    'lu par le réseau : l’intention ne double pas la ligne',
  )
})

test('texte de l’état : envoyé, validé, non retenu avec son motif', () => {
  const [attente, envoye, valide, refuse] = lignesDesSignalements(
    [
      rapport('b', '2027-11-10T11:00:00Z'),
      rapport('c', '2027-11-10T10:00:00Z', { status: 'validated', decided_at: '2027-11-10T10:30:00Z' }),
      rapport('d', '2027-11-10T09:00:00Z', {
        status: 'rejected',
        decided_at: '2027-11-10T09:30:00Z',
        reject_reason: 'source_maintains',
        reject_detail: 'La salle est confirmée.',
      }),
    ],
    [rapport('a', '2027-11-10T12:00:00Z')],
  )
  assert.deepEqual(texteDeLEtat(attente!), { cle: 'en-attente', a: '2027-11-10T12:00:00Z', motif: null, precision: null })
  assert.equal(texteDeLEtat(envoye!).cle, 'envoye')
  assert.deepEqual(texteDeLEtat(valide!), { cle: 'valide', a: '2027-11-10T10:30:00Z', motif: null, precision: null })
  assert.deepEqual(texteDeLEtat(refuse!), {
    cle: 'non-retenu',
    a: '2027-11-10T09:30:00Z',
    motif: 'source_maintains',
    precision: 'La salle est confirmée.',
  })
})

test('« Votre signalement » : seulement tant qu’il n’est pas tranché', () => {
  const tranche = lignesDesSignalements([rapport('a', '2027-11-10T09:00:00Z', { status: 'validated' })], [])
  assert.equal(signalementEnCours('s1', tranche), null)
  const enCours = lignesDesSignalements([], [rapport('b', '2027-11-10T10:00:00Z')])
  assert.equal(signalementEnCours('s1', enCours)?.signalement.client_ref, 'b')
  assert.equal(signalementEnCours('s2', enCours), null)
})

test('référence client : un UUID, différent à chaque appel', () => {
  const x = referenceClient()
  assert.match(x, /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/)
  assert.notEqual(x, referenceClient())
})

test('file de validation : validé passe aux traités, annulé reprend sa place', () => {
  const item = (id: string, submitted_at: string, status: MyReport['status'] = 'submitted') =>
    ({
      ...rapport(id, submitted_at, { id, status }),
      author: { name: 'A', country: null },
      source_now: null,
      decided_by: null,
      published_at: null,
      withdrawn_at: null,
    }) satisfies ReportQueueItem
  const file = { pending: [item('a', '2027-11-10T09:00:00Z'), item('b', '2027-11-10T10:00:00Z')], decided_today: [] }
  const apres = apresDecision(file, item('a', '2027-11-10T09:00:00Z', 'validated'))
  assert.deepEqual(apres.pending.map((r) => r.id), ['b'])
  assert.deepEqual(apres.decided_today.map((r) => r.id), ['a'])
  const annule = apresDecision(apres, item('a', '2027-11-10T09:00:00Z'))
  assert.deepEqual(annule.pending.map((r) => r.id), ['a', 'b'])
  assert.deepEqual(annule.decided_today, [])
})

test('traités : non retenu, en publication, validé, retiré', () => {
  const item = (champs: Partial<ReportQueueItem>): ReportQueueItem => ({
    ...rapport('x', '2027-11-10T09:00:00Z', { status: 'validated' }),
    author: { name: 'A', country: null },
    source_now: null,
    decided_by: 'IFDD',
    published_at: null,
    withdrawn_at: null,
    ...champs,
  })
  assert.equal(etatDuTraite(item({ status: 'rejected' })), 'non-retenu')
  assert.equal(etatDuTraite(item({})), 'en-publication')
  assert.equal(etatDuTraite(item({ published_at: '2027-11-10T09:01:00Z' })), 'valide')
  assert.equal(etatDuTraite(item({ published_at: '2027-11-10T09:01:00Z', withdrawn_at: '2027-11-10T10:00:00Z' })), 'retire')
})

test('réunions : filtre des thématiques, agenda, et par jour quand la source est coupée', () => {
  const r1 = { ...reunion('r1', '2027-11-11', null), theme: 'finance' }
  const r2 = reunion('r2', '2027-11-10', '2027-11-10T15:00:00Z')
  const passee = reunion('r0', '2027-11-09', null)
  assert.equal(reunionPasseLeFiltre(r1, ['adaptation']), false)
  assert.equal(reunionPasseLeFiltre(r1, ['finance']), true)
  assert.equal(reunionPasseLeFiltre(r2, []), true, 'sans thématique, elle passe')
  const agenda = { entries: [], network_entries: [{ network_meeting_id: 'r1', remind: false, added_at: '2027-11-10T09:00:00Z' }] }
  assert.deepEqual(reunionsDeLAgenda(agenda, [r1, r2]).map((r) => r.id), ['r1'])
  const jours = reunionsParJour([r1, r2, passee], a('2027-11-10T13:00:00Z'), BELEM)
  assert.deepEqual(jours.map((j) => j.jour), ['2027-11-10', '2027-11-11'])
})

const avis = (id: string, created_at: string, read_at: string | null = null): Notification => ({
  id,
  type_code: 'negotiation.meeting.changed',
  title: { fr: id },
  body: null,
  variables: {},
  link_path: null,
  subject_schema: null,
  subject_table: null,
  subject_id: null,
  group_count: 1,
  read_at,
  created_at,
})

test('notifications : les lectures en file comptent aussitôt', () => {
  const feed = { items: [avis('a', '2027-11-10T12:00:00Z'), avis('b', '2027-11-10T11:00:00Z', '2027-11-10T11:30:00Z')], unread_count: 4 }
  const lu = avecLesLectures(feed, new Set(['a', 'b']), a('2027-11-10T13:00:00Z'))
  assert.equal(lu.unread_count, 3, 'b était déjà lue : elle ne se décompte pas deux fois')
  assert.equal(lu.items[0]!.read_at, '2027-11-10T13:00:00.000Z')
  assert.deepEqual(idsNonLues(feed.items), ['a'])
})

test('notifications : groupées par jour dans le fuseau de la COP', () => {
  // 02:00 UTC le 11 = 23:00 le 10 à Belém.
  const jours = notificationsParJour(
    [avis('a', '2027-11-11T02:00:00Z'), avis('b', '2027-11-11T04:00:00Z'), avis('c', '2027-11-10T15:00:00Z')],
    BELEM,
  )
  assert.deepEqual(
    jours.map((j) => [j.jour, j.notifications.map((n) => n.id)]),
    [
      ['2027-11-11', ['b']],
      ['2027-11-10', ['a', 'c']],
    ],
  )
})

test('le corps d’un changement ne garde que la valeur de son motif, dans le fuseau de la COP', () => {
  const s = { id: 's1', start_at: '2026-11-12T13:00:00Z' }
  const saisie = { heure: '15:30', salle: '  Salle 9 ', precision: ' ' }
  assert.deepEqual(corpsDuChangement('venue', saisie, s, 'Europe/Istanbul'), {
    reason: 'venue',
    session_id: 's1',
    proposed_venue: 'Salle 9',
  })
  assert.deepEqual(corpsDuChangement('time', saisie, s, 'Europe/Istanbul'), {
    reason: 'time',
    session_id: 's1',
    proposed_start: '2026-11-12T12:30:00.000Z',
  })
  assert.deepEqual(corpsDuChangement('cancelled', { heure: '', salle: '', precision: 'Écran éteint' }, s, 'UTC'), {
    reason: 'cancelled',
    session_id: 's1',
    detail: 'Écran éteint',
  })
})

test('une réunion non annoncée exige « Quoi », le reste est facultatif', () => {
  const vide = { quoi: '  ', ou: 'Couloir', quand: '16:30', thematique: 'adaptation' }
  assert.equal(corpsDeLaReunion(vide, '2026-11-12', 'UTC'), null)
  assert.deepEqual(corpsDeLaReunion({ ...vide, quoi: 'Aparté' }, '2026-11-12', 'UTC'), {
    reason: 'unannounced',
    what: 'Aparté',
    day: '2026-11-12',
    theme: 'adaptation',
    proposed_start: '2026-11-12T16:30:00.000Z',
    proposed_venue: 'Couloir',
  })
  assert.deepEqual(corpsDeLaReunion({ quoi: 'Aparté', ou: '', quand: '', thematique: null }, '2026-11-12', 'UTC'), {
    reason: 'unannounced',
    what: 'Aparté',
    day: '2026-11-12',
    theme: null,
  })
})

test('notifications : le pictogramme dit le motif, ou la décision', () => {
  const de = (type_code: string, variables: Record<string, unknown>) => dessinDeNotification({ ...avis('x', '2027-11-10T12:00:00Z'), type_code, variables })
  assert.equal(de('negotiation.meeting.changed', { change: 'annulee' }).teinte, 'annulee')
  assert.equal(de('negotiation.meeting.changed', { change: 'sallechangee' }).picto, 'pin')
  assert.equal(de('negotiation.report.published', { reason: 'time' }).picto, 'moved')
  assert.equal(de('negotiation.network_meeting.published', {}).picto, 'diamond')
  assert.equal(de('negotiation.report.decided', { status: 'rejected' }).teinte, 'non-retenu')
  assert.equal(de('negotiation.report.decided', { status: 'published' }).teinte, 'valide')
  assert.equal(de('autre.chose', {}).picto, 'bell')
})

test('cloche : neuf au plus, puis « 9+ »', () => {
  assert.equal(compteurDeCloche(3), '3')
  assert.equal(compteurDeCloche(9), '9')
  assert.equal(compteurDeCloche(12), '9+')
})
