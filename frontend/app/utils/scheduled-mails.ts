import type { ScheduledMailBatchRow, ScheduledMailKind, ScheduledMailType } from '~/types/admin-scheduled-mails'

/**
 * COURRIELS PROGRAMMÉS (04/11) : décompte et regroupement, purs.
 *
 * Le décompte se cale sur l'heure du serveur : l'API rend `observed_at`, et
 * l'écart avec l'horloge locale à la réception corrige une machine qui avance
 * ou retarde. Sans lui, un poste à l'heure fausse annoncerait un départ qui a
 * déjà eu lieu.
 */

/** Écart, en millisecondes, entre l'horloge du serveur et l'horloge locale. */
export function clockOffsetMs(observedAt: string, receivedAt: number): number {
  const server = Date.parse(observedAt)
  return Number.isFinite(server) ? server - receivedAt : 0
}

export interface CountdownParts {
  days: number
  hours: number
  minutes: number
  seconds: number
  /** Secondes restantes, jamais négatives. */
  total: number
}

/** Ce qui reste avant `target`, à la seconde, vu depuis `nowMs` (heure du serveur). */
export function countdownParts(target: string, nowMs: number): CountdownParts {
  const at = Date.parse(target)
  let rest = Number.isFinite(at) ? Math.max(0, Math.floor((at - nowMs) / 1000)) : 0
  const total = rest
  const days = Math.floor(rest / 86_400)
  rest -= days * 86_400
  const hours = Math.floor(rest / 3600)
  rest -= hours * 3600
  const minutes = Math.floor(rest / 60)
  return { days, hours, minutes, seconds: rest - minutes * 60, total }
}

/** Un décalage en minutes, dans l'unité où il se dit : « 2 jours », « 1 heure », « 30 minutes ». */
export function offsetUnit(minutes: number): { unit: 'day' | 'hour' | 'minute'; count: number } {
  if (minutes % 1440 === 0) return { unit: 'day', count: minutes / 1440 }
  if (minutes % 60 === 0) return { unit: 'hour', count: minutes / 60 }
  return { unit: 'minute', count: minutes }
}

export function scheduledTypeKey(kind: ScheduledMailKind, offsetMinutes: number): string {
  return kind === 'day' ? 'day' : `activity.${offsetMinutes}`
}

/**
 * Les envois en types, comme l'API (`domain::programmes::regrouper`) : la
 * journée d'abord, puis les activités du plus lointain au plus proche ; les
 * envois par heure de départ.
 */
export function groupScheduledBatches(
  rows: ScheduledMailBatchRow[],
  customTypes: ReadonlySet<string> = new Set(),
): ScheduledMailType[] {
  const types = new Map<string, ScheduledMailType>()
  for (const row of rows) {
    const key = scheduledTypeKey(row.kind, row.offset_minutes)
    const type = types.get(key) ?? {
      key,
      kind: row.kind,
      offset_minutes: row.kind === 'day' ? null : row.offset_minutes,
      count: 0,
      next_run_at: row.run_at,
      has_custom_text: customTypes.has(key),
      batches: [],
    }
    type.count += row.recipients
    if (Date.parse(row.run_at) < Date.parse(type.next_run_at)) type.next_run_at = row.run_at
    type.batches.push(row)
    types.set(key, type)
  }
  const list = [...types.values()]
  for (const type of list) {
    type.batches.sort((a, b) => Date.parse(a.run_at) - Date.parse(b.run_at) || a.title.localeCompare(b.title))
  }
  return list.sort(
    (a, b) => Number(a.kind !== 'day') - Number(b.kind !== 'day') || (b.offset_minutes ?? 0) - (a.offset_minutes ?? 0),
  )
}

/** « 06/11 09:00 » dans le fuseau de l'édition. */
export function shortDayTime(value: string, timeZone: string, locale: string): string {
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return ''
  const day = new Intl.DateTimeFormat(locale, { timeZone, day: '2-digit', month: '2-digit' }).format(date)
  const time = new Intl.DateTimeFormat(locale, { timeZone, hour: '2-digit', minute: '2-digit', hourCycle: 'h23' }).format(date)
  return `${day} ${time}`
}
