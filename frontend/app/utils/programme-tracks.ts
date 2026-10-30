import type { PublicScheduleTrack, PublicScheduleTrackMoment } from '~/types/programme/track-page'
import type { PublicScheduleRow } from '~/types/views'
import type { IsoDate, IsoDateTime, TimeZoneName } from '~/types/shared'
import { dayKeyInZone } from './datetime.ts'

/**
 * Les fils publiés dans `/programmations` (direction F, 30/10) : ce que la
 * liste, la semaine et le ruban des jours en tirent, sans DOM.
 *
 * Une activité rattachée à une journée spéciale publiée se range dans son
 * cadre ; une activité de l'atelier préparatoire (`side_programme`) se range
 * sous son encadré, qui n'a pas de déroulé. Un fil absent de la liste (non
 * publié) ne change rien : l'activité reste une ligne ordinaire.
 */

export interface ProgrammeTrackIndex {
  /** Journées spéciales publiées, par slug. */
  specials: Map<string, PublicScheduleTrack>
  /** Atelier préparatoire (et tout autre programme parallèle publié). */
  sides: PublicScheduleTrack[]
}

export function indexTracks(tracks: PublicScheduleTrack[]): ProgrammeTrackIndex {
  return {
    specials: new Map(tracks.filter((track) => track.kind === 'special_day').map((track) => [track.slug, track])),
    sides: tracks.filter((track) => track.kind === 'side_programme'),
  }
}

/** La première journée spéciale publiée de l'activité. */
export function specialOf(session: PublicScheduleRow, index: ProgrammeTrackIndex): PublicScheduleTrack | null {
  for (const badge of session.tracks) {
    const track = badge.kind === 'special_day' ? index.specials.get(badge.slug) : undefined
    if (track) return track
  }
  return null
}

export function sideOf(session: PublicScheduleRow, index: ProgrammeTrackIndex): PublicScheduleTrack | null {
  return index.sides.find((side) => session.tracks.some((badge) => badge.slug === side.slug)) ?? null
}

/** Le jour où se tient un fil : celui de sa première heure, à défaut sa date annoncée. */
export function trackDayOf(track: PublicScheduleTrack, timezone: TimeZoneName): IsoDate | null {
  if (track.starts_at) return dayKeyInZone(track.starts_at, timezone)
  return track.starts_on
}

export type FrameItem<Row> =
  | { kind: 'activity'; session: PublicScheduleRow; row: Row; index: number }
  | { kind: 'moments'; key: string; moments: PublicScheduleTrackMoment[] }

export interface DayFrame<Row> {
  track: PublicScheduleTrack
  /** Première et dernière heure du fil CE jour-là, moments compris. */
  start: IsoDateTime | null
  end: IsoDateTime | null
  items: FrameItem<Row>[]
}

/**
 * Le cadre d'une journée un jour donné : ses activités du jour, et, hors
 * recherche, ses moments, chaque suite de moments sans activité entre eux
 * réunie en une ligne. `own` : toutes les activités du fil ce jour-là ;
 * `kept` : celles que la recherche retient. Nul si aucune n'est retenue.
 */
export function composeFrame<Row>(
  track: PublicScheduleTrack,
  own: PublicScheduleRow[],
  kept: Set<string>,
  day: IsoDate,
  timezone: TimeZoneName,
  searching: boolean,
  toRow: (session: PublicScheduleRow) => Row,
): DayFrame<Row> | null {
  if (!own.some((session) => kept.has(session.id))) return null
  const moments = track.moments.filter((moment) => dayKeyInZone(moment.starts_at, timezone) === day)

  // Un moment passe avant une activité de même heure, comme dans le déroulé de la page.
  const timeline = [
    ...moments.map((moment) => ({ at: moment.starts_at, rank: 0, moment, session: null })),
    ...own.map((session) => ({ at: session.starts_at, rank: 1, moment: null, session })),
  ].sort((a, b) => Date.parse(a.at) - Date.parse(b.at) || a.rank - b.rank)

  const items: FrameItem<Row>[] = []
  let index = 0
  for (const entry of timeline) {
    if (entry.session) {
      if (!kept.has(entry.session.id)) continue
      items.push({ kind: 'activity', session: entry.session, row: toRow(entry.session), index: index++ })
    } else if (!searching && entry.moment) {
      const last = items[items.length - 1]
      if (last?.kind === 'moments') last.moments.push(entry.moment)
      else items.push({ kind: 'moments', key: `m-${entry.moment.starts_at}`, moments: [entry.moment] })
    }
  }

  const starts = [...moments.map((m) => m.starts_at), ...own.map((s) => s.starts_at)]
  const ends = [...moments.map((m) => m.ends_at ?? m.starts_at), ...own.map((s) => s.ends_at)]
  const byTime = (a: string, b: string) => Date.parse(a) - Date.parse(b)
  return {
    track,
    start: starts.sort(byTime)[0] ?? null,
    end: ends.sort(byTime).at(-1) ?? null,
    items,
  }
}
