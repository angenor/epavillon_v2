/**
 * Régie du jour, hors ligne : les séances publiées de l'édition, recalées sur
 * l'heure courante (une passée, une en direct, la suivante dans moins d'une
 * heure, une annulée…), et des gestes qui modifient cet état en mémoire.
 */
import type {
  MeetingHostLink,
  RegieActionKind,
  RegieCancelPayload,
  RegieDocument,
  RegieGesture,
  RegieJournalEntry,
  RegieOrganization,
  RegieReason,
  RegieScreen,
  RegieSession,
  RegieSpeakerSummary,
  RegieUndoPayload,
} from '~/types/admin-regie'
import type { OnlineSpeakerCreated, OnlineSpeakerInput, RegieSpeaker, RegieSpeakers, SpeakerLinksSent } from '~/types/programme/online-speaker'
import type { IsoDate, Uuid } from '~/types/shared'
import { ApiRequestError } from '~/utils/api-error'
import { dayKeyInZone } from '~/utils/datetime'
import { events } from './event'
import { organizations } from './org'
import { allSessions } from './sessions'

const MINUTE = 60_000
const NOTICE_DELAY = 5 * MINUTE
const RESUME_DELAY = 15 * MINUTE

const REASONS: RegieReason[] = [
  {
    code: 'no_show',
    label: { fr: 'Les organisateurs ne sont pas venus', en: 'The organisers did not show up' },
    description: { fr: 'Constaté sur place.', en: 'Observed on site.' },
  },
  {
    code: 'notified_absence',
    label: { fr: 'Les organisateurs ont prévenu de leur absence', en: 'The organisers sent notice of absence' },
    description: { fr: 'Par courriel, téléphone ou message.', en: 'By email, phone or message.' },
  },
  {
    code: 'other',
    label: { fr: 'Autre raison', en: 'Other reason' },
    description: { fr: "À préciser pour l'équipe ; elle n'est pas publiée.", en: 'Describe it for the team; it is not published.' },
  },
]

interface MockAction {
  id: Uuid
  kind: RegieActionKind
  session_id: Uuid
  acted_at: string
  undone_at: string | null
  before: RegieSession
}

const days = new Map<Uuid, RegieSession[]>()
const actions: MockAction[] = []
let sequence = 0

const nextId = (): Uuid => `01a05000-0000-7000-8000-${String(++sequence).padStart(12, '0')}`
const iso = (ms: number): string => new Date(ms).toISOString()

function organizationOf(id: string | null): RegieOrganization[] {
  const organization = organizations.find((entry) => entry.id === id)
  if (!organization) return []
  return [{ id: organization.id, name: organization.legal_name, acronym: organization.acronym, logo: null }]
}

function documentsOf(index: number, proposalId: Uuid, uploadedAt: string): RegieDocument[] {
  if (index === 4) return []
  const slides: RegieDocument = {
    id: `01a05100-0000-7000-8000-${String(index * 2 + 1).padStart(12, '0')}`,
    proposal_id: proposalId,
    title: { fr: 'Présentation principale', en: 'Main presentation' },
    filename: 'presentation.pptx',
    mime_type: 'application/vnd.openxmlformats-officedocument.presentationml.presentation',
    byte_size: 4_400_000,
    url: 'https://example.org/presentation.pptx',
    uploaded_at: uploadedAt,
  }
  const note: RegieDocument = {
    id: `01a05100-0000-7000-8000-${String(index * 2 + 2).padStart(12, '0')}`,
    proposal_id: proposalId,
    title: { fr: 'Note de synthèse', en: 'Briefing note' },
    filename: 'note.pdf',
    mime_type: 'application/pdf',
    byte_size: 635_000,
    url: index === 2 ? null : 'https://example.org/note.pdf',
    uploaded_at: uploadedAt,
  }
  return index === 1 || index === 2 ? [slides, note] : [slides]
}

/** La journée d'exemple, figée au premier appel pour que les gestes s'y appliquent. */
function sessionsOf(eventId: Uuid): RegieSession[] {
  const known = days.get(eventId)
  if (known) return known
  const anchor = Math.floor(Date.now() / MINUTE) * MINUTE
  const source = allSessions
    .filter((session) => session.event_id === eventId && session.published_at !== null)
    .slice(0, 6)
  const built = source.map((session, index): RegieSession => {
    const startsAt = anchor - 25 * MINUTE + (index - 1) * 75 * MINUTE
    const endsAt = startsAt + 60 * MINUTE
    const orgs = organizationOf(session.organization_id)
    const joint = index === 2 && organizations[1] ? [...orgs, ...organizationOf(organizations[1].id)] : orgs
    const proposalId = session.proposal_id ?? `01a05200-0000-7000-8000-${String(index).padStart(12, '0')}`
    return {
      session_id: session.id,
      title: session.title,
      starts_at: iso(startsAt),
      ends_at: iso(endsAt),
      status: index === 0 ? 'completed' : index === 1 ? 'live' : index === 3 ? 'cancelled' : 'scheduled',
      is_published: true,
      is_streamed: index !== 5,
      is_joint: index === 2,
      room_name: { fr: 'Salle principale', en: 'Main room' },
      cover: null,
      organizations: joint,
      registrants: 30 + index * 4,
      presence:
        index === 2
          ? { state: 'unknown', noted_at: null, noted_by: null }
          : { state: 'confirmed', noted_at: iso(startsAt - 30 * MINUTE), noted_by: 'Aïcha K.' },
      meeting:
        index === 4
          ? null
          : {
              status: index === 1 ? 'started' : index === 0 ? 'ended' : 'scheduled',
              sync_status: 'synced',
              ready: index !== 0,
              present_count: index === 1 ? 212 : null,
            },
      documents: documentsOf(index, proposalId, iso(anchor - 20 * 60 * MINUTE)),
      live_started_at: index === 1 ? iso(startsAt + 2 * MINUTE) : index === 0 ? iso(startsAt) : null,
      live_ended_at: index === 0 ? iso(endsAt) : null,
      resumable_until: null,
      cancellation:
        index === 3
          ? {
              cancelled_at: iso(anchor - 120 * MINUTE),
              reason_code: 'notified_absence',
              reason_label: REASONS[1]!.label,
              note: null,
              action_id: null,
              notice: {
                state: 'sent',
                due_at: iso(anchor - 115 * MINUTE),
                sent_at: iso(anchor - 115 * MINUTE),
                recipients: 41,
              },
            }
          : null,
      last_action_id: null,
      speakers: speakerSummary(seedSpeakers(session.id, index, anchor)),
    }
  })
  days.set(eventId, built)
  return built
}

function findSession(sessionId: Uuid): { list: RegieSession[]; index: number } {
  for (const list of days.values()) {
    const index = list.findIndex((session) => session.session_id === sessionId)
    if (index >= 0) return { list, index }
  }
  throw new ApiRequestError({ code: 'NOT_FOUND', message: "Cette activité n'existe pas, ou n'est pas dans votre périmètre." }, 404)
}

function record(kind: RegieActionKind, sessionId: Uuid, change: (session: RegieSession, action: MockAction) => RegieSession): RegieGesture {
  const { list, index } = findSession(sessionId)
  const before = list[index]!
  const action: MockAction = { id: nextId(), kind, session_id: sessionId, acted_at: iso(Date.now()), undone_at: null, before }
  const after = { ...change(before, action), last_action_id: action.id }
  list[index] = after
  actions.unshift(action)
  return gestureOf(action, after)
}

function gestureOf(action: MockAction, session: RegieSession): RegieGesture {
  return {
    action_id: action.id,
    kind: action.kind,
    session_id: action.session_id,
    acted_at: action.acted_at,
    notify_at: action.kind === 'session_cancelled' ? (session.cancellation?.notice.due_at ?? null) : null,
    registrants: session.registrants,
    resumable_until: session.resumable_until,
    undone_at: action.undone_at,
  }
}

function journalOf(eventId: Uuid): RegieJournalEntry[] {
  const ids = new Set(sessionsOf(eventId).map((session) => session.session_id))
  const nowMs = Date.now()
  return actions
    .filter((action) => ids.has(action.session_id))
    .map((action) => {
      const current = findSession(action.session_id)
      const session = current.list[current.index]!
      const resumable = action.kind !== 'live_ended' || (session.resumable_until !== null && Date.parse(session.resumable_until) > nowMs)
      return {
        id: action.id,
        kind: action.kind,
        session_id: action.session_id,
        session_title: session.title,
        session_starts_at: session.starts_at,
        incident_message: null,
        incident_kind_code: null,
        acted_at: action.acted_at,
        actor_name: null,
        actor_is_me: true,
        undone_at: action.undone_at,
        undone_by: action.undone_at ? 'vous' : null,
        can_undo: action.undone_at === null && resumable && session.last_action_id === action.id,
        notice: action.kind === 'session_cancelled' ? (session.cancellation?.notice ?? null) : null,
      }
    })
}

/** L'heure de passage d'un courriel différé : passée, il est parti. */
function settleNotices(eventId: Uuid): void {
  const nowMs = Date.now()
  const list = sessionsOf(eventId)
  list.forEach((session, index) => {
    const notice = session.cancellation?.notice
    if (session.cancellation && notice?.state === 'pending' && notice.due_at && Date.parse(notice.due_at) <= nowMs) {
      list[index] = { ...session, cancellation: { ...session.cancellation, notice: { ...notice, state: 'sent', sent_at: notice.due_at } } }
    }
  })
}

export function regieScreen(eventId: Uuid, day: IsoDate | null): RegieScreen | null {
  const event = events.find((entry) => entry.id === eventId)
  if (!event) return null
  const nowMs = Date.now()
  const today = dayKeyInZone(nowMs, event.timezone)
  const dayList = [-1, 0, 1].map((offset) => dayKeyInZone(nowMs + offset * 24 * 60 * MINUTE, event.timezone))
  const shown = day && dayList.includes(day) ? day : today
  settleNotices(eventId)
  const sessions = shown === today ? sessionsOf(eventId) : []
  return {
    event_id: event.id,
    event_title: event.title,
    event_acronym: event.acronym,
    timezone: event.timezone,
    zone_label: event.city,
    days: dayList,
    day: shown,
    today,
    now: iso(nowMs),
    live_elsewhere: null,
    sessions: sessions.map((session) => ({ ...session, speakers: speakerSummary(speakersOf(session.session_id)) })),
    journal: shown === today ? journalOf(eventId) : [],
    reasons: REASONS,
  }
}

export function regieConfirmPresence(sessionId: Uuid): RegieGesture {
  return record('presence_confirmed', sessionId, (session) => ({
    ...session,
    presence: { state: 'confirmed', noted_at: iso(Date.now()), noted_by: 'vous' },
  }))
}

export function regieCancel(sessionId: Uuid, payload: RegieCancelPayload): RegieGesture {
  return record('session_cancelled', sessionId, (session, action) => {
    const reason = REASONS.find((entry) => entry.code === payload.reason_code) ?? REASONS[2]!
    const notify = payload.notify_registrants && session.registrants > 0
    const absent = payload.reason_code === 'no_show' || payload.reason_code === 'notified_absence'
    return {
      ...session,
      status: 'cancelled',
      presence: absent ? { state: 'absent', noted_at: action.acted_at, noted_by: 'vous' } : session.presence,
      cancellation: {
        cancelled_at: action.acted_at,
        reason_code: reason.code,
        reason_label: reason.label,
        note: payload.note,
        action_id: action.id,
        notice: {
          state: notify ? 'pending' : 'none',
          due_at: notify ? iso(Date.parse(action.acted_at) + NOTICE_DELAY) : null,
          sent_at: null,
          recipients: notify ? session.registrants : 0,
        },
      },
    }
  })
}

export function regieStartLive(sessionId: Uuid): RegieGesture {
  const { list } = findSession(sessionId)
  if (list.some((session) => session.status === 'live' && session.session_id !== sessionId)) {
    throw new ApiRequestError(
      { code: 'CONFLICT', message: "Un autre direct est en cours : terminez-le d'abord. Un seul direct à la fois." },
      409,
    )
  }
  return record('live_started', sessionId, (session, action) => ({
    ...session,
    status: 'live',
    live_started_at: action.acted_at,
    live_ended_at: null,
    resumable_until: null,
  }))
}

export function regieEndLive(sessionId: Uuid): RegieGesture {
  return record('live_ended', sessionId, (session, action) => ({
    ...session,
    status: 'completed',
    live_ended_at: action.acted_at,
    resumable_until: iso(Date.parse(action.acted_at) + RESUME_DELAY),
  }))
}

export function regieUndo(actionId: Uuid, _payload: RegieUndoPayload): RegieGesture {
  const action = actions.find((entry) => entry.id === actionId)
  if (!action || action.undone_at) {
    throw new ApiRequestError({ code: 'CONFLICT', message: 'Ce geste a déjà été défait.' }, 409)
  }
  const { list, index } = findSession(action.session_id)
  const current = list[index]!
  if (action.kind === 'live_ended' && (!current.resumable_until || Date.parse(current.resumable_until) <= Date.now())) {
    throw new ApiRequestError(
      { code: 'CONFLICT', message: 'Le direct est terminé depuis plus de 15 minutes : il ne se reprend plus.' },
      409,
    )
  }
  action.undone_at = iso(Date.now())
  const restored: RegieSession = { ...action.before, last_action_id: null }
  list[index] = restored
  return gestureOf(action, restored)
}

export function meetingHostLink(_sessionId: Uuid): MeetingHostLink {
  return { url: 'https://zoom.us/s/0000000000' }
}

// ---------------------------------------------------------------------------
// Intervenants en ligne : trois inscrits dont un par le lien, aucun, sans porteur
// ---------------------------------------------------------------------------

const speakerLists = new Map<Uuid, RegieSpeakers>()

function seedSpeakers(sessionId: Uuid, index: number, anchor: number): RegieSpeakers {
  const known = speakerLists.get(sessionId)
  if (known) return known
  const sentAt = iso(anchor - 22 * 60 * MINUTE)
  const speaker = (n: number, fields: Omit<RegieSpeaker, 'speaker_id'>): RegieSpeaker => ({
    speaker_id: `01a05500-0000-7000-8000-${String(index * 10 + n).padStart(12, '0')}`,
    ...fields,
  })
  const speakers: RegieSpeaker[] =
    index === 2
      ? [
          speaker(1, { display_name: 'Mme Aminata Diallo', job_title: 'Directrice', organization: 'Exemple de ministère', email: 'aminata.diallo@exemple.org', source: 'dossier', is_listed: true, created_at: iso(anchor - 30 * 24 * 60 * MINUTE), link_sent_at: sentAt }),
          speaker(2, { display_name: 'M. Jean-Pierre Mbala', job_title: 'Conseiller', organization: "Exemple d'agence", email: 'jp.mbala@exemple.org', source: 'regie', is_listed: true, created_at: iso(anchor - 2 * 24 * 60 * MINUTE), link_sent_at: sentAt }),
          speaker(3, { display_name: 'Dr Koffi Mensah', job_title: 'Chercheur', organization: "Exemple d'université", email: 'k.mensah@exemple.org', source: 'lien', is_listed: false, created_at: iso(anchor - 3 * 60 * MINUTE), link_sent_at: iso(anchor - 3 * 60 * MINUTE) }),
          speaker(4, { display_name: 'M. Paul Kasongo', job_title: 'Expert', organization: null, email: null, source: 'dossier', is_listed: true, created_at: iso(anchor - 30 * 24 * 60 * MINUTE), link_sent_at: null }),
        ]
      : []
  const list: RegieSpeakers = {
    session_id: sessionId,
    speakers,
    signup_url: `https://epavillon.exemple.org/intervenant?jeton=exemple-${index}`,
    owner_emails: index === 1 ? [] : ['porteur@exemple.org'],
    owner_link_sent_at: index === 4 ? iso(anchor - 46 * 60 * MINUTE) : null,
  }
  speakerLists.set(sessionId, list)
  return list
}

/** Le rendu serveur a lu l'écran dans son propre exemplaire des mocks : le navigateur reconstruit la journée. */
function speakersOf(sessionId: Uuid): RegieSpeakers {
  const eventId = allSessions.find((session) => session.id === sessionId)?.event_id
  if (!speakerLists.has(sessionId) && eventId) sessionsOf(eventId)
  const known = speakerLists.get(sessionId)
  if (known) return known
  findSession(sessionId)
  return seedSpeakers(sessionId, -1, Date.now())
}

function speakerSummary(list: RegieSpeakers): RegieSpeakerSummary {
  const sent = list.speakers.map((entry) => entry.link_sent_at).filter((at): at is string => at !== null).sort()
  return {
    online: sent.length,
    reachable: list.speakers.filter((entry) => entry.email !== null).length,
    to_list: list.speakers.filter((entry) => entry.source === 'lien' && !entry.is_listed).length,
    links_sent_at: sent.at(-1) ?? null,
    owner_link_sent_at: list.owner_link_sent_at,
    has_owner: list.owner_emails.length > 0,
  }
}

const copy = (list: RegieSpeakers): RegieSpeakers => ({ ...list, speakers: list.speakers.map((entry) => ({ ...entry })) })

export function regieSpeakers(sessionId: Uuid): RegieSpeakers {
  return copy(speakersOf(sessionId))
}

export function regieAddSpeaker(sessionId: Uuid, input: OnlineSpeakerInput): OnlineSpeakerCreated {
  const list = speakersOf(sessionId)
  if (list.speakers.some((entry) => entry.email?.toLowerCase() === input.email.toLowerCase())) {
    throw new ApiRequestError({ code: 'CONFLICT', message: 'Cette adresse est déjà inscrite comme intervenant de cette activité.', field: 'email' }, 409)
  }
  const civility = input.civility ? { mme: 'Mme', m: 'M.', dr: 'Dr', pr: 'Pr', other: '' }[input.civility] : ''
  const now = iso(Date.now())
  const speaker: RegieSpeaker = {
    speaker_id: nextId(),
    display_name: [civility, input.first_name, input.last_name].filter(Boolean).join(' '),
    job_title: input.job_title,
    organization: input.organization,
    email: input.email,
    source: 'regie',
    is_listed: true,
    created_at: now,
    link_sent_at: now,
  }
  list.speakers.push(speaker)
  return { speaker_id: speaker.speaker_id }
}

export function regieListSpeaker(sessionId: Uuid, speakerId: Uuid): RegieSpeakers {
  const list = speakersOf(sessionId)
  const speaker = list.speakers.find((entry) => entry.speaker_id === speakerId)
  if (!speaker) throw new ApiRequestError({ code: 'NOT_FOUND', message: "Cet intervenant n'existe pas." }, 404)
  speaker.is_listed = true
  return copy(list)
}

export function regieResendZoomLinks(sessionId: Uuid): SpeakerLinksSent {
  const list = speakersOf(sessionId)
  const now = iso(Date.now())
  const reachable = list.speakers.filter((entry) => entry.email !== null)
  reachable.forEach((entry) => (entry.link_sent_at = now))
  return { recipients: reachable.length }
}

export function regieSendOwnerLink(sessionId: Uuid): SpeakerLinksSent {
  const list = speakersOf(sessionId)
  if (!list.owner_emails.length) {
    throw new ApiRequestError({ code: 'CONFLICT', message: "Cette activité n'a pas de porteur à qui envoyer le lien." }, 409)
  }
  list.owner_link_sent_at = iso(Date.now())
  return { recipients: list.owner_emails.length }
}
