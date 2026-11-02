import type { TrackKind } from '~/types/event/edition'
import type { PublicTrackActivity, PublicTrackEntry, PublicTrackPage, PublicTrackPart } from '~/types/programme/track-page'
import type { ScheduleTrackBadge } from '~/types/views'

/**
 * L'inscription à une journée (journée spéciale, atelier préparatoire) : ce
 * que la fenêtre d'inscription range et compte, sans Vue, pour les tests.
 */

/** Les fils qui se suivent par un seul direct, toute la journée. */
export const isDayTrackKind = (kind: TrackKind): boolean => kind === 'special_day' || kind === 'side_programme'

/** La journée dont relève une activité : la première journée spéciale, à défaut l'atelier. */
export function dayTrackOf(tracks: readonly ScheduleTrackBadge[]): ScheduleTrackBadge | null {
  return tracks.find((track) => track.kind === 'special_day') ?? tracks.find((track) => track.kind === 'side_programme') ?? null
}

/** Une activité se garde ou se décoche ; annulée, elle n'est plus proposée (comme le M de l'API). */
export const isPickable = (activity: PublicTrackActivity): boolean => activity.session.status !== 'cancelled'

/** Les activités qu'on peut garder, dans l'ordre du déroulé. */
export function pickableIds(page: PublicTrackPage): string[] {
  return page.agenda.flatMap((entry) => (entry.kind === 'activity' && isPickable(entry.activity) ? [entry.activity.session.id] : []))
}

export type DayPickEntry =
  | { kind: 'activity'; startsAt: string; activity: PublicTrackActivity }
  | { kind: 'moment'; startsAt: string; key: string; entry: Extract<PublicTrackEntry, { kind: 'moment' }> }

export interface DayPickGroup {
  /** Nulle : ce qui n'appartient à aucune partie, en tête. */
  part: PublicTrackPart | null
  entries: DayPickEntry[]
}

/** Le déroulé rangé par partie, comme `TrackAgenda` : activités à cocher, moments compris dans le direct. */
export function dayPickGroups(page: PublicTrackPage): DayPickGroup[] {
  const known = new Set(page.parts.map((part) => part.id))
  const toPick = (entry: PublicTrackEntry, index: number): DayPickEntry[] => {
    if (entry.kind === 'moment') return [{ kind: 'moment', startsAt: entry.starts_at, key: `m-${index}`, entry }]
    return isPickable(entry.activity) ? [{ kind: 'activity', startsAt: entry.starts_at, activity: entry.activity }] : []
  }
  const indexed = page.agenda.map((entry, index) => ({ entry, index }))
  const loose = indexed.filter(({ entry }) => !entry.part_id || !known.has(entry.part_id)).flatMap(({ entry, index }) => toPick(entry, index))
  const groups: DayPickGroup[] = loose.length ? [{ part: null, entries: loose }] : []
  for (const part of page.parts) {
    const entries = indexed.filter(({ entry }) => entry.part_id === part.id).flatMap(({ entry, index }) => toPick(entry, index))
    if (entries.some((entry) => entry.kind === 'activity')) groups.push({ part, entries })
  }
  return groups
}

/** Les activités gardées encore proposées : une activité annulée depuis ne compte plus. */
export function keptIds(kept: readonly string[], page: PublicTrackPage): string[] {
  const keep = new Set(kept)
  return pickableIds(page).filter((id) => keep.has(id))
}
