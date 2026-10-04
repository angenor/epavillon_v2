import type { PublicScheduleRow, TemporalState } from '~/types/views'

export type ProgrammeSessionState = TemporalState | 'live'

/** État et couleur d'une activité sur l'affiche — partagés par la semaine, la liste et la fiche. */
export function useProgrammeSession() {
  const { isLive } = useLiveSession()
  const localePath = useLocalePath()
  const { t } = useI18n()

  /** Le direct l'emporte sur l'état temporel, pour la seule séance déclarée (règle 4). */
  const state = (session: PublicScheduleRow): ProgrammeSessionState =>
    isLive(session.id) ? 'live' : session.temporal_state

  const themeColor = (session: PublicScheduleRow): string | null => session.themes[0]?.color ?? null

  /** L'aplat d'une activité : la couleur de sa thématique, adoucie dans le papier. */
  const fill = (color: string | null): string =>
    color
      ? `color-mix(in oklab, ${color} var(--poster-theme-strength), var(--color-poster-paper-raised))`
      : 'var(--color-poster-paper-raised)'

  /** « Sur place » sur une séance diffusée laisse croire qu'il faut venir : on le tait. */
  const showFormat = (session: PublicScheduleRow): boolean => !(session.is_streamed && session.format === 'in_person')

  const link = (editionSlug: string, session: PublicScheduleRow): string =>
    localePath({ name: 'activity-edition-session', params: { edition: editionSlug, session: session.slug } })

  /** « 1 h 30 », « 45 min ». */
  function duration(session: PublicScheduleRow): string {
    const parts = durationParts(Math.round((Date.parse(session.ends_at) - Date.parse(session.starts_at)) / 60_000))
    if (!parts) return ''
    if (!parts.hours) return t('programme.list.minutes', { minutes: parts.minutes })
    return parts.minutes
      ? t('programme.list.hoursMinutes', { hours: parts.hours, minutes: String(parts.minutes).padStart(2, '0') })
      : t('programme.list.hours', { hours: parts.hours })
  }

  /** « Marchés carbone : quelles garanties ? » — la partie avant les deux-points porte le gras. */
  function titleParts(title: string): { lead: string; rest: string } {
    const index = title.search(/[\s\u00a0\u202f](?::[\s\u00a0\u202f]|\?)/)
    return index > 0 ? { lead: title.slice(0, index), rest: title.slice(index) } : { lead: title, rest: '' }
  }

  return { state, themeColor, fill, showFormat, link, duration, titleParts }
}
