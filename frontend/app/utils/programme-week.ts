import type { PublicScheduleRow } from '~/types/views'
import type { IsoDate, TimeZoneName } from '~/types/shared'
import { dayKeyInZone, wallClockInZone, type DateInput } from './datetime.ts'

/** Minutes écoulées depuis minuit, à l'heure murale du fuseau. */
export function minutesInZone(value: DateInput, timeZone: TimeZoneName): number {
  const [hours = 0, minutes = 0] = wallClockInZone(value, timeZone).slice(11).split(':').map(Number)
  return hours * 60 + minutes
}

export function addDays(day: IsoDate, count: number): IsoDate {
  const date = new Date(`${day}T12:00:00Z`)
  date.setUTCDate(date.getUTCDate() + count)
  return date.toISOString().slice(0, 10)
}

export function daysBetween(from: IsoDate, to: IsoDate): number {
  return Math.round((Date.parse(`${to}T12:00:00Z`) - Date.parse(`${from}T12:00:00Z`)) / 86_400_000)
}

/** Le lundi de la semaine qui contient ce jour. */
export function weekStart(day: IsoDate): IsoDate {
  const weekday = new Date(`${day}T12:00:00Z`).getUTCDay()
  return addDays(day, -((weekday + 6) % 7))
}

export interface SessionSpan {
  session: PublicScheduleRow
  /** Minutes depuis minuit, dans le fuseau de l'édition. */
  start: number
  end: number
}

/** Une séance qui déborde sur le lendemain s'arrête à minuit dans sa colonne. */
export function spanOf(session: PublicScheduleRow, timeZone: TimeZoneName): SessionSpan {
  const start = minutesInZone(session.starts_at, timeZone)
  const sameDay = dayKeyInZone(session.ends_at, timeZone) === dayKeyInZone(session.starts_at, timeZone)
  const end = sameDay ? minutesInZone(session.ends_at, timeZone) : 24 * 60
  return { session, start, end: Math.max(end, start + 15) }
}

export interface PlacedSession extends SessionSpan {
  lane: number
  lanes: number
}

/**
 * Répartit en couloirs les séances d'une même journée qui se chevauchent. Un
 * seul stand rend le cas exceptionnel, mais il n'est jamais bloqué (règles 2 et
 * 3) : la grille le montre côte à côte plutôt que de cacher une séance sous
 * une autre.
 */
export function layoutDay(spans: SessionSpan[]): PlacedSession[] {
  const sorted = [...spans].sort((a, b) => a.start - b.start || b.end - a.end)
  const placed: PlacedSession[] = []
  let cluster: PlacedSession[] = []
  let clusterEnd = -1
  let laneEnds: number[] = []

  const close = (): void => {
    const lanes = laneEnds.length
    for (const item of cluster) item.lanes = lanes
    cluster = []
    laneEnds = []
  }

  for (const span of sorted) {
    if (span.start >= clusterEnd) {
      close()
      clusterEnd = -1
    }
    let lane = laneEnds.findIndex((end) => end <= span.start)
    if (lane === -1) {
      lane = laneEnds.length
      laneEnds.push(span.end)
    } else {
      laneEnds[lane] = span.end
    }
    const item: PlacedSession = { ...span, lane, lanes: 1 }
    cluster.push(item)
    placed.push(item)
    clusterEnd = Math.max(clusterEnd, span.end)
  }
  close()
  return placed
}

/** Heures pleines qui encadrent le programme : la grille se cale sur l'édition, pas sur une journée type. */
export function hourRange(spans: SessionSpan[], fallback: [number, number] = [9, 18]): [number, number] {
  if (!spans.length) return fallback
  const first = Math.floor(Math.min(...spans.map((span) => span.start)) / 60)
  const last = Math.ceil(Math.max(...spans.map((span) => span.end)) / 60)
  return [Math.min(first, 23), Math.max(last, first + 1)]
}
