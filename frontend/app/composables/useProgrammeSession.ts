import type { PublicScheduleRow, TemporalState } from '~/types/views'

export type ProgrammeSessionState = TemporalState | 'live'

/** État et couleur d'une activité sur l'affiche — partagés par la semaine, la liste et la fiche. */
export function useProgrammeSession() {
  const { isLive } = useLiveSession()
  const localePath = useLocalePath()

  /** Le direct l'emporte sur l'état temporel, pour la seule séance déclarée (règle 4). */
  const state = (session: PublicScheduleRow): ProgrammeSessionState =>
    isLive(session.id) ? 'live' : session.temporal_state

  const themeColor = (session: PublicScheduleRow): string | null => session.themes[0]?.color ?? null

  /** L'aplat d'une activité : la couleur de sa thématique, adoucie dans le papier. */
  const fill = (color: string | null): string =>
    color
      ? `color-mix(in oklab, ${color} var(--poster-theme-strength), var(--color-poster-paper-raised))`
      : 'var(--color-poster-paper-raised)'

  const link = (editionSlug: string, session: PublicScheduleRow): string =>
    localePath({ name: 'activity-edition-session', params: { edition: editionSlug, session: session.slug } })

  return { state, themeColor, fill, link }
}
