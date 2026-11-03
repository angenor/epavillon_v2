import type { PublicScheduleTrack, PublicScheduleTrackPart } from '~/types/programme/track-page'
import type { PublicScheduleRow } from '~/types/views'
import type { I18nText, IsoDate, IsoDateTime, TimeZoneName } from '~/types/shared'
import { dayKeyInZone } from './datetime.ts'

/**
 * Les fils publiés dans `/programmations` (journées en un bloc, 03/11) : ce que
 * la liste, le calendrier et le ruban des jours en tirent, sans DOM.
 *
 * Une journée spéciale publiée et l'atelier préparatoire (`side_programme`)
 * tiennent chacun en un bloc à leur couleur, qui mène à leur page : leurs
 * activités n'apparaissent plus une à une, le détail vit sur la page du fil.
 * Une activité d'aucun fil publié reste une ligne ordinaire, à son heure,
 * avant ou après le bloc.
 */

export interface ProgrammeTrackIndex {
  /** Journées spéciales et atelier publiés, par slug. */
  tracks: Map<string, PublicScheduleTrack>
}

export function indexTracks(tracks: PublicScheduleTrack[]): ProgrammeTrackIndex {
  return { tracks: new Map(tracks.map((track) => [track.slug, track])) }
}

/** Le fil publié qui absorbe l'activité : une journée spéciale d'abord, à défaut l'atelier. */
export function trackOf(session: PublicScheduleRow, index: ProgrammeTrackIndex): PublicScheduleTrack | null {
  const published = session.tracks.flatMap((badge) => {
    const track = index.tracks.get(badge.slug)
    return track ? [track] : []
  })
  return published.find((track) => track.kind === 'special_day') ?? published[0] ?? null
}

/** Le jour où se tient un fil : celui de sa première heure, à défaut sa date annoncée. */
export function trackDayOf(track: PublicScheduleTrack, timezone: TimeZoneName): IsoDate | null {
  if (track.starts_at) return dayKeyInZone(track.starts_at, timezone)
  return track.starts_on
}

export interface TrackBlockHighlight {
  at: IsoDateTime
  title: I18nText
}

/** Un fil un jour donné, tel que son bloc le résume. */
export interface TrackBlock {
  track: PublicScheduleTrack
  /** Première et dernière heure du fil CE jour-là, moments compris. */
  start: IsoDateTime | null
  end: IsoDateTime | null
  /** Ses temps qui tombent ce jour-là. */
  parts: PublicScheduleTrackPart[]
  /** Le premier temps fort du jour, activité ou moment. */
  highlight: TrackBlockHighlight | null
  /** Activités du jour, hors celles qu'un moment regroupe. */
  activities: number
  moments: number
}

export type ProgrammeDayItem =
  | { kind: 'block'; key: string; block: TrackBlock }
  | { kind: 'session'; key: string; session: PublicScheduleRow }

export interface ComposedDay {
  /** Blocs et activités ordinaires retenus, dans l'ordre de l'heure. */
  items: ProgrammeDayItem[]
  blocks: TrackBlock[]
  /** Activités ordinaires retenues. */
  sessions: PublicScheduleRow[]
  /** Ce que le jour montrerait sans recherche : activités ordinaires et blocs. */
  total: number
}

const byTime = (a: string, b: string) => Date.parse(a) - Date.parse(b)

function blockOf(track: PublicScheduleTrack, own: PublicScheduleRow[], day: IsoDate, timezone: TimeZoneName): TrackBlock {
  const onDay = (at: IsoDateTime) => dayKeyInZone(at, timezone) === day
  const moments = track.moments.filter((moment) => onDay(moment.starts_at))
  const grouped = new Set(track.moments.flatMap((moment) => moment.session_ids))

  const starts = [...moments.map((m) => m.starts_at), ...own.map((s) => s.starts_at)].sort(byTime)
  const ends = [...moments.map((m) => m.ends_at ?? m.starts_at), ...own.map((s) => s.ends_at)].sort(byTime)
  const sameDay = track.starts_at !== null && onDay(track.starts_at)

  const highlights: TrackBlockHighlight[] = [
    ...own
      .filter((session) => session.status !== 'cancelled' && session.tracks.some((badge) => badge.slug === track.slug && badge.is_highlight))
      .map((session) => ({ at: session.starts_at, title: session.title })),
    ...moments.filter((moment) => moment.is_highlight).map((moment) => ({ at: moment.starts_at, title: moment.label })),
  ].sort((a, b) => byTime(a.at, b.at))

  return {
    track,
    start: starts[0] ?? (sameDay ? track.starts_at : null),
    end: ends.at(-1) ?? (sameDay ? track.ends_at : null),
    parts: track.parts.filter((part) => part.starts_at !== null && onDay(part.starts_at)),
    highlight: highlights[0] ?? null,
    activities: own.filter((session) => !grouped.has(session.id)).length,
    moments: moments.length,
  }
}

/**
 * Le programme d'un jour : un bloc par fil publié qui s'y tient, ses activités
 * absorbées ; les autres activités retenues par la recherche. Un bloc reste
 * l'entrée d'une recherche qui retrouve son nom ou l'une de ses activités ;
 * sinon il se tait. `own` : les activités qui commencent ce jour-là.
 */
export function composeDay(
  day: IsoDate,
  own: PublicScheduleRow[],
  index: ProgrammeTrackIndex,
  timezone: TimeZoneName,
  matches: (session: PublicScheduleRow) => boolean,
  matchesTrack: (track: PublicScheduleTrack) => boolean,
): ComposedDay {
  const byTrack = new Map<string, PublicScheduleRow[]>()
  const loose: PublicScheduleRow[] = []
  for (const session of own) {
    const track = trackOf(session, index)
    if (track) byTrack.set(track.slug, [...(byTrack.get(track.slug) ?? []), session])
    else loose.push(session)
  }
  for (const track of index.tracks.values()) {
    if (!byTrack.has(track.slug) && trackDayOf(track, timezone) === day) byTrack.set(track.slug, [])
  }

  const blocks: TrackBlock[] = []
  for (const [slug, sessions] of byTrack) {
    const track = index.tracks.get(slug)!
    if (matchesTrack(track) || sessions.some(matches)) blocks.push(blockOf(track, sessions, day, timezone))
  }
  const sessions = loose.filter(matches)

  const items: ProgrammeDayItem[] = [
    ...blocks.map((block): ProgrammeDayItem => ({ kind: 'block', key: `fil-${block.track.slug}`, block })),
    ...sessions.map((session): ProgrammeDayItem => ({ kind: 'session', key: session.id, session })),
  ]
  const at = (item: ProgrammeDayItem) => Date.parse(item.kind === 'block' ? (item.block.start ?? `${day}T00:00:00Z`) : item.session.starts_at)
  // À heure égale, le bloc d'abord : il ouvre la journée.
  items.sort((a, b) => at(a) - at(b) || (a.kind === 'block' ? -1 : 0) - (b.kind === 'block' ? -1 : 0))

  return { items, blocks, sessions, total: loose.length + byTrack.size }
}
