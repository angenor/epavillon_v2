import { test } from 'node:test'
import assert from 'node:assert/strict'
import {
  countdownTo,
  defaultRegieDay,
  documentKind,
  focusSession,
  journalUndoMode,
  nextNoticeDue,
  nextSession,
  regieAlerts,
  regieOrganizationsLabel,
  regieTopics,
  sessionDisplayState,
  slotProgress,
  startLiveBlock,
  undoDeadline,
} from '../../app/utils/regie.ts'
import { isValidTopic } from '../../app/utils/realtime.ts'
import type { RegieGesture, RegieJournalEntry, RegieSession } from '../../app/types/admin-regie.ts'

const NOW = Date.parse('2026-11-13T10:41:00+03:00')
const at = (hhmm: string): string => `2026-11-13T${hhmm}:00+03:00`

function seance(id: string, debut: string, fin: string, extra: Partial<RegieSession> = {}): RegieSession {
  return {
    session_id: id,
    title: { fr: id },
    starts_at: at(debut),
    ends_at: at(fin),
    status: 'scheduled',
    is_published: true,
    is_streamed: true,
    is_joint: false,
    room_name: null,
    cover: null,
    organizations: [],
    registrants: 0,
    presence: { state: 'confirmed', noted_at: null, noted_by: null },
    meeting: { status: 'scheduled', sync_status: 'synced', ready: true, present_count: null },
    documents: [
      {
        id: `${id}-d`,
        proposal_id: 'p',
        title: { fr: 'Support' },
        filename: 'support.pdf',
        mime_type: 'application/pdf',
        byte_size: 1000,
        url: null,
        uploaded_at: at('08:00'),
      },
    ],
    live_started_at: null,
    live_ended_at: null,
    resumable_until: null,
    cancellation: null,
    last_action_id: null,
    ...extra,
  }
}

function entree(kind: RegieJournalEntry['kind'], extra: Partial<RegieJournalEntry> = {}): RegieJournalEntry {
  return {
    id: kind,
    kind,
    session_id: null,
    session_title: null,
    session_starts_at: null,
    incident_message: null,
    incident_kind_code: null,
    acted_at: at('09:00'),
    actor_name: null,
    actor_is_me: true,
    undone_at: null,
    undone_by: null,
    can_undo: true,
    notice: null,
    ...extra,
  }
}

test('jour par défaut : aujourd’hui, sinon le premier à venir, sinon le premier', () => {
  const jours = ['2026-11-12', '2026-11-13', '2026-11-14']
  assert.equal(defaultRegieDay(jours, '2026-11-13'), '2026-11-13')
  assert.equal(defaultRegieDay(jours, '2026-11-01'), '2026-11-12')
  assert.equal(defaultRegieDay(['2026-11-14', '2026-11-12'], '2026-11-13'), '2026-11-14')
  assert.equal(defaultRegieDay(jours, '2026-12-01'), '2026-11-12')
  assert.equal(defaultRegieDay([], '2026-11-13'), null)
})

test('le bloc « maintenant » : le direct, sinon la prochaine séance non annulée', () => {
  const passee = seance('passee', '09:15', '10:00', { status: 'completed' })
  const annulee = seance('annulee', '11:00', '11:20', { status: 'cancelled' })
  const suivante = seance('suivante', '11:30', '12:30')
  assert.equal(nextSession([passee, annulee, suivante], NOW)?.session_id, 'suivante')
  const direct = seance('direct', '10:15', '11:15', { status: 'live' })
  assert.equal(focusSession([passee, direct, suivante], NOW)?.session_id, 'direct')
  assert.equal(focusSession([passee], NOW), null)
})

test('écoulé depuis le début du direct, restant jusqu’à la fin du créneau', () => {
  const direct = seance('direct', '10:15', '11:15', { status: 'live', live_started_at: at('10:17') })
  const p = slotProgress(direct, NOW)
  assert.equal(p.elapsedMinutes, 24)
  assert.equal(p.remainingMinutes, 34)
  assert.equal(p.percent, 43)
  assert.equal(p.overrun, false)
  const deborde = slotProgress(direct, Date.parse(at('11:20')))
  assert.equal(deborde.percent, 100)
  assert.equal(deborde.remainingMinutes, 0)
  assert.ok(deborde.overrun)
})

test('état affiché d’une séance', () => {
  assert.equal(sessionDisplayState(seance('a', '09:00', '10:00'), NOW), 'past')
  assert.equal(sessionDisplayState(seance('a', '10:30', '11:30'), NOW), 'ongoing')
  assert.equal(sessionDisplayState(seance('a', '12:00', '13:00'), NOW), 'upcoming')
  assert.equal(sessionDisplayState(seance('a', '10:30', '11:30', { status: 'live' }), NOW), 'live')
  assert.equal(sessionDisplayState(seance('a', '12:00', '13:00', { status: 'cancelled' }), NOW), 'cancelled')
})

test('à surveiller : présence à moins d’une heure, présentations, réunion Zoom', () => {
  const proche = seance('proche', '11:30', '12:30', { presence: { state: 'unknown', noted_at: null, noted_by: null } })
  const lointaine = seance('lointaine', '14:15', '15:15', {
    presence: { state: 'unknown', noted_at: null, noted_by: null },
    documents: [],
    meeting: null,
  })
  const pasPrete = seance('pasPrete', '16:00', '17:00', {
    meeting: { status: 'scheduled', sync_status: 'pending', ready: false, present_count: null },
  })
  const nonDiffusee = seance('nonDiffusee', '17:00', '18:00', { is_streamed: false, meeting: null })
  const annulee = seance('annulee', '12:00', '13:00', { status: 'cancelled', documents: [] })
  const alertes = regieAlerts([proche, annulee, lointaine, pasPrete, nonDiffusee], NOW)
  assert.deepEqual(
    alertes.map((a) => `${a.kind}:${a.session.session_id}`),
    ['presence:proche', 'documents:lointaine', 'no_meeting:lointaine', 'meeting_not_ready:pasPrete'],
  )
  assert.equal(alertes[0]!.minutesUntil, 49)
})

test('démarrer le direct : un seul à la fois, séance publiée', () => {
  const direct = seance('direct', '10:15', '11:15', { status: 'live' })
  const suivante = seance('suivante', '11:30', '12:30')
  assert.equal(startLiveBlock(suivante, [direct, suivante], null), 'other_live')
  assert.equal(startLiveBlock(suivante, [suivante], { session_id: 'x', title: null, same_event: false }), 'other_live')
  assert.equal(startLiveBlock(seance('b', '12:00', '13:00', { is_published: false }), [], null), 'not_published')
  assert.equal(startLiveBlock(seance('c', '12:00', '13:00', { status: 'cancelled' }), [], null), 'withdrawn')
  assert.equal(startLiveBlock(suivante, [suivante], null), null)
})

test('le bandeau « Défaire » attend le courriel aux inscrits', () => {
  const geste: RegieGesture = {
    action_id: 'a',
    kind: 'session_cancelled',
    session_id: 's',
    acted_at: new Date(NOW).toISOString(),
    notify_at: new Date(NOW + 5 * 60_000).toISOString(),
    registrants: 38,
    resumable_until: null,
    undone_at: null,
  }
  assert.equal(undoDeadline(geste, NOW), NOW + 5 * 60_000)
  assert.equal(undoDeadline({ ...geste, notify_at: null }, NOW), NOW + 10_000)
  assert.equal(undoDeadline({ ...geste, kind: 'presence_confirmed' }, NOW), NOW + 10_000)
  assert.deepEqual(countdownTo(NOW + 292_000, NOW), { totalMs: 292_000, minutes: 4, seconds: 52 })
  assert.deepEqual(countdownTo(NOW - 1, NOW), { totalMs: 0, minutes: 0, seconds: 0 })
})

test('journal : le bouton dit ce qu’il défait', () => {
  assert.equal(journalUndoMode(entree('presence_confirmed')), 'undo')
  assert.equal(journalUndoMode(entree('session_cancelled')), 'restore')
  assert.equal(journalUndoMode(entree('live_ended')), 'resume')
  assert.equal(journalUndoMode(entree('incident_published')), 'unpublish')
  assert.equal(journalUndoMode(entree('live_started', { can_undo: false })), null)
  assert.equal(journalUndoMode(entree('live_started', { undone_at: at('09:01') })), null)
})

test('l’écran se relit à l’échéance du prochain courriel', () => {
  const due = at('10:45')
  const annulee = seance('annulee', '13:00', '14:00', {
    status: 'cancelled',
    cancellation: {
      cancelled_at: at('10:40'),
      reason_code: 'no_show',
      reason_label: null,
      note: null,
      action_id: 'a',
      notice: { state: 'pending', due_at: due, sent_at: null, recipients: 12 },
    },
  })
  const passe = entree('session_cancelled', { notice: { state: 'pending', due_at: at('10:00'), sent_at: null, recipients: 3 } })
  assert.equal(nextNoticeDue([annulee], [passe], NOW), Date.parse(due))
  assert.equal(nextNoticeDue([], [passe], NOW), null)
})

test('type lisible d’une présentation', () => {
  assert.equal(documentKind('application/vnd.openxmlformats-officedocument.presentationml.presentation', null), 'slides')
  assert.equal(documentKind('application/pdf', 'x.bin'), 'pdf')
  assert.equal(documentKind(null, 'Note.DOCX'), 'text')
  assert.equal(documentKind('application/octet-stream', 'deck.key'), 'slides')
  assert.equal(documentKind(null, null), 'other')
})

test('sujets temps réel de la régie', () => {
  const id = '0198a1b2-0000-7000-8000-000000000001'
  const sujets = regieTopics(id)
  assert.ok(sujets.includes(`event:${id}:regie`))
  assert.ok(sujets.every(isValidTopic))
  assert.deepEqual(regieTopics(null), [])
})

test('organisations : sigle d’abord, liste de la locale', () => {
  const orgs = [
    { name: 'Association internationale des régions francophones', acronym: 'AIRF' },
    { name: 'Business Maurice', acronym: null },
  ]
  assert.equal(regieOrganizationsLabel(orgs, 'fr'), 'AIRF et Business Maurice')
  assert.equal(regieOrganizationsLabel(orgs, 'en'), 'AIRF and Business Maurice')
})
