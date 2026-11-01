/**
 * Régie du jour : la logique pure de l'écran `/admin/regie`, testée par
 * `tests/site/regie.test.ts`. Aucun appel, aucune traduction : l'écran lit
 * l'heure, ce fichier dit ce qu'elle implique.
 */
import type { RegieGesture, RegieJournalEntry, RegieLiveElsewhere, RegieSession } from '~/types/admin-regie'
import type { IncidentSeverity } from '~/types/live'
import type { RealtimeTopic } from '~/types/realtime'
import type { IsoDate } from '~/types/shared'
import { deskTopic, publicEditionTopic } from './realtime-public.ts'

const MINUTE_MS = 60_000

/** Présence encore inconnue à moins d'une heure du début : on la signale. */
export const PRESENCE_WARNING_MS = 60 * MINUTE_MS
/** Durée du bandeau « Défaire » quand aucun courriel n'attend. */
export const UNDO_WINDOW_MS = 10_000

/** Le jour par défaut, comme l'API : aujourd'hui s'il en est, sinon le premier à venir, sinon le premier. */
export function defaultRegieDay(days: readonly IsoDate[], today: IsoDate): IsoDate | null {
  if (days.length === 0) return null
  if (days.includes(today)) return today
  const sorted = [...days].sort()
  return sorted.find((day) => day >= today) ?? sorted[0] ?? null
}

const isWithdrawn = (session: RegieSession): boolean =>
  session.status === 'cancelled' || session.status === 'postponed'

export function liveSession(sessions: readonly RegieSession[]): RegieSession | null {
  return sessions.find((session) => session.status === 'live') ?? null
}

/** La prochaine séance qui peut encore passer à l'antenne : ni annulée, ni close, pas finie. */
export function nextSession(sessions: readonly RegieSession[], nowMs: number): RegieSession | null {
  return (
    sessions.find(
      (session) =>
        (session.status === 'planned' || session.status === 'scheduled') &&
        Date.parse(session.ends_at) > nowMs,
    ) ?? null
  )
}

/** Le bloc « maintenant » : le direct, sinon la prochaine séance. */
export function focusSession(sessions: readonly RegieSession[], nowMs: number): RegieSession | null {
  return liveSession(sessions) ?? nextSession(sessions, nowMs)
}

export interface SlotProgress {
  elapsedMinutes: number
  remainingMinutes: number
  /** Avancement dans le créneau, de 0 à 100. */
  percent: number
  overrun: boolean
}

/** Temps écoulé depuis le début du direct, restant jusqu'à la fin du créneau. */
export function slotProgress(session: RegieSession, nowMs: number): SlotProgress {
  const slotStart = Date.parse(session.starts_at)
  const end = Date.parse(session.ends_at)
  const start = session.live_started_at ? Date.parse(session.live_started_at) : slotStart
  const span = end - slotStart
  const ratio = span > 0 ? (nowMs - slotStart) / span : 1
  return {
    elapsedMinutes: Math.max(0, Math.floor((nowMs - start) / MINUTE_MS)),
    remainingMinutes: Math.max(0, Math.ceil((end - nowMs) / MINUTE_MS)),
    percent: Math.min(100, Math.max(0, Math.round(ratio * 100))),
    overrun: nowMs > end,
  }
}

export type RegieDisplayState = 'live' | 'upcoming' | 'ongoing' | 'past' | 'cancelled' | 'postponed'

export function sessionDisplayState(session: RegieSession, nowMs: number): RegieDisplayState {
  if (session.status === 'live') return 'live'
  if (session.status === 'cancelled') return 'cancelled'
  if (session.status === 'postponed') return 'postponed'
  if (session.status === 'completed' || Date.parse(session.ends_at) <= nowMs) return 'past'
  if (Date.parse(session.starts_at) <= nowMs) return 'ongoing'
  return 'upcoming'
}

export type RegieAlertKind = 'presence' | 'documents' | 'no_meeting' | 'meeting_not_ready'

export interface RegieAlert {
  kind: RegieAlertKind
  session: RegieSession
  /** Minutes avant le début, zéro une fois commencée. */
  minutesUntil: number
}

/** « À surveiller » : ce qui peut encore mal tourner sur les séances à venir. */
export function regieAlerts(sessions: readonly RegieSession[], nowMs: number): RegieAlert[] {
  const alerts: RegieAlert[] = []
  for (const session of sessions) {
    if (isWithdrawn(session) || session.status === 'live' || session.status === 'completed') continue
    if (Date.parse(session.ends_at) <= nowMs) continue
    const untilMs = Date.parse(session.starts_at) - nowMs
    const minutesUntil = Math.max(0, Math.ceil(untilMs / MINUTE_MS))
    if (session.presence.state === 'unknown' && untilMs <= PRESENCE_WARNING_MS) {
      alerts.push({ kind: 'presence', session, minutesUntil })
    }
    if (session.documents.length === 0) alerts.push({ kind: 'documents', session, minutesUntil })
    if (session.is_streamed && !session.meeting) alerts.push({ kind: 'no_meeting', session, minutesUntil })
    else if (session.is_streamed && session.meeting && !session.meeting.ready) {
      alerts.push({ kind: 'meeting_not_ready', session, minutesUntil })
    }
  }
  return alerts
}

export type StartLiveBlock = 'other_live' | 'not_published' | 'withdrawn'

/** Pourquoi « Démarrer le direct » est éteint ; nul s'il peut partir. Un seul direct à la fois (règle n° 4). */
export function startLiveBlock(
  session: RegieSession,
  sessions: readonly RegieSession[],
  liveElsewhere: RegieLiveElsewhere | null,
): StartLiveBlock | null {
  if (isWithdrawn(session)) return 'withdrawn'
  const otherHere = sessions.some((other) => other.status === 'live' && other.session_id !== session.session_id)
  if (otherHere || (liveElsewhere && liveElsewhere.session_id !== session.session_id)) return 'other_live'
  if (!session.is_published) return 'not_published'
  return null
}

export interface CountdownSplit {
  totalMs: number
  minutes: number
  seconds: number
}

export function countdownTo(targetMs: number, nowMs: number): CountdownSplit {
  const totalMs = Math.max(0, targetMs - nowMs)
  const totalSeconds = Math.ceil(totalMs / 1000)
  return { totalMs, minutes: Math.floor(totalSeconds / 60), seconds: totalSeconds % 60 }
}

/** Fin du bandeau « Défaire » : dix secondes, ou jusqu'au courriel aux inscrits s'il est plus tardif. */
export function undoDeadline(gesture: RegieGesture, shownAtMs: number): number {
  const minimum = shownAtMs + UNDO_WINDOW_MS
  if (gesture.kind === 'session_cancelled' && gesture.notify_at) {
    return Math.max(minimum, Date.parse(gesture.notify_at))
  }
  return minimum
}

export type JournalUndoMode = 'undo' | 'restore' | 'resume' | 'unpublish'

/** Le bouton d'une ligne du journal, ou nul si elle ne se défait plus. */
export function journalUndoMode(entry: RegieJournalEntry): JournalUndoMode | null {
  if (!entry.can_undo || entry.undone_at) return null
  switch (entry.kind) {
    case 'session_cancelled':
      return 'restore'
    case 'live_ended':
      return 'resume'
    case 'incident_published':
      return 'unpublish'
    default:
      return 'undo'
  }
}

/** Prochaine échéance d'un courriel différé : l'écran se relit à cette heure-là. */
export function nextNoticeDue(
  sessions: readonly RegieSession[],
  journal: readonly RegieJournalEntry[],
  nowMs: number,
): number | null {
  const dues = [
    ...sessions.map((session) => session.cancellation?.notice ?? null),
    ...journal.map((entry) => entry.notice),
  ]
    .filter((notice) => notice?.state === 'pending' && notice.due_at)
    .map((notice) => Date.parse(notice!.due_at!))
    .filter((due) => due > nowMs)
  return dues.length ? Math.min(...dues) : null
}

export type DocumentKind = 'slides' | 'pdf' | 'text' | 'sheet' | 'image' | 'video' | 'other'

const EXTENSIONS: Record<string, DocumentKind> = {
  ppt: 'slides',
  pptx: 'slides',
  odp: 'slides',
  key: 'slides',
  pdf: 'pdf',
  doc: 'text',
  docx: 'text',
  odt: 'text',
  xls: 'sheet',
  xlsx: 'sheet',
  ods: 'sheet',
  csv: 'sheet',
  png: 'image',
  jpg: 'image',
  jpeg: 'image',
  webp: 'image',
  mp4: 'video',
  mov: 'video',
  webm: 'video',
}

/** Type lisible d'un fichier : le type MIME d'abord, l'extension à défaut. */
export function documentKind(mimeType: string | null, filename: string | null): DocumentKind {
  const mime = (mimeType ?? '').toLowerCase()
  if (mime.includes('presentation') || mime.includes('powerpoint')) return 'slides'
  if (mime === 'application/pdf') return 'pdf'
  if (mime.includes('wordprocessing') || mime === 'application/msword' || mime.includes('opendocument.text')) return 'text'
  if (mime.includes('spreadsheet') || mime.includes('excel') || mime === 'text/csv') return 'sheet'
  if (mime.startsWith('image/')) return 'image'
  if (mime.startsWith('video/')) return 'video'
  const extension = filename?.split('.').pop()?.toLowerCase() ?? ''
  return EXTENSIONS[extension] ?? 'other'
}

export type RegieTemplateId = 'delay' | 'overrun' | 'technical' | 'room' | 'free'

export interface RegieTemplate {
  id: RegieTemplateId
  /** Code de la taxonomie `incident_kind`. */
  kind: string
  /** Suggérée : jaune pour ce qui demande attention, rouge pour une panne. */
  severity: IncidentSeverity
}

/** Les modèles du nouveau message, dans l'ordre de la maquette. */
export const REGIE_TEMPLATES: readonly RegieTemplate[] = [
  { id: 'delay', kind: 'delay', severity: 'warning' },
  { id: 'overrun', kind: 'overrun', severity: 'warning' },
  { id: 'technical', kind: 'technical_issue', severity: 'error' },
  { id: 'room', kind: 'room_change', severity: 'info' },
  { id: 'free', kind: 'information', severity: 'info' },
]

/** Ordre de la page au nouveau message : un modèle et une activité (raccourci « débordement », remise à zéro). */
export interface ComposerCommand {
  nonce: number
  template: RegieTemplateId
  sessionId: string | null
}

/** Ce que la fenêtre d'annulation décide : le geste, plus le message d'incident qui le suit. */
export interface RegieCancelChoice {
  reason_code: string
  note: string | null
  notify_registrants: boolean
  publish_incident: boolean
}

/** Ce que la fenêtre de rétablissement décide. */
export interface RegieRestoreChoice {
  unpublish_incident: boolean
  notify_registrants: boolean
}

/** Nature retenue : celle du modèle si la taxonomie la connaît, « information » sinon. */
export function templateKind(template: RegieTemplate, knownKinds: readonly string[]): string {
  if (!knownKinds.length || knownKinds.includes(template.kind)) return template.kind
  return knownKinds.includes('information') ? 'information' : template.kind
}

/** « AIRF et AIMF » : le sigle s'il existe, la liste à la manière de la locale. */
export function regieOrganizationsLabel(
  organizations: readonly { name: string; acronym: string | null }[],
  locale: string,
): string {
  const names = organizations.map((organization) => organization.acronym?.trim() || organization.name)
  return new Intl.ListFormat(locale, { type: 'conjunction' }).format(names)
}

/** Les sujets qui font relire la régie. Un signal ne porte rien : on relit l'écran. */
export function regieTopics(eventId: string | null): RealtimeTopic[] {
  if (!eventId) return []
  return [
    `event:${eventId}:regie`,
    `event:${eventId}:planner`,
    deskTopic(eventId),
    publicEditionTopic(eventId, 'incidents'),
    publicEditionTopic(eventId, 'live'),
  ]
}
