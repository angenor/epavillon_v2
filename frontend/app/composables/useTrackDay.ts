import type { Ref } from 'vue'
import type { PublicTrackEntry, PublicTrackPage } from '~/types/programme/track-page'
import type { SessionDisplayState } from '~/components/ui/StatusBadge.vue'

export type TrackDayPhase = 'before' | 'during' | 'after'

/**
 * Le moment de la page d'une journée spéciale : avant, pendant, après, et
 * l'état de chaque ligne du déroulé. Une horloge pour toute la page : l'heure
 * du rendu serveur passe au navigateur, qui l'avance ensuite toutes les 30 s.
 *
 * Une activité annulée ou reportée garde l'état de la base ; le direct l'emporte
 * (règle 4) ; le reste se lit sur l'horloge, comme les moments sans activité,
 * qui n'ont pas d'état en base.
 */
export function useTrackDay(page: Ref<PublicTrackPage | null>) {
  const { isLive } = useLiveSession()
  const { locale } = useI18n()

  const now = useState('journee-maintenant', () => Date.now())
  const mounted = ref(false)
  let clock: ReturnType<typeof setInterval> | undefined

  onMounted(() => {
    mounted.value = true
    now.value = Date.now()
    clock = setInterval(() => (now.value = Date.now()), 30_000)
  })
  onBeforeUnmount(() => clearInterval(clock))

  const timezone = computed(() => page.value?.edition.timezone ?? 'UTC')
  const today = computed(() => dayKeyInZone(now.value, timezone.value))

  function clockState(startsAt: string, endsAt: string | null): SessionDisplayState {
    const start = Date.parse(startsAt)
    const end = Date.parse(endsAt ?? startsAt)
    if (now.value < start) return 'upcoming'
    return now.value < end ? 'ongoing' : 'past'
  }

  function entryState(entry: PublicTrackEntry): SessionDisplayState {
    if (entry.kind === 'moment') return clockState(entry.starts_at, entry.ends_at)
    const session = entry.activity.session
    if (session.temporal_state === 'cancelled' || session.temporal_state === 'postponed') return session.temporal_state
    if (isLive(session.id)) return 'live'
    return clockState(session.starts_at, session.ends_at)
  }

  const firstDay = computed(() => {
    const value = page.value
    if (!value) return null
    const start = value.opens_at ?? value.starts_at
    return start ? dayKeyInZone(start, timezone.value) : value.track.starts_on
  })

  const phase = computed<TrackDayPhase>(() => {
    const value = page.value
    if (!value) return 'before'
    const start = value.opens_at ?? value.starts_at
    if (start && value.ends_at) {
      if (now.value < Date.parse(start)) return 'before'
      return now.value < Date.parse(value.ends_at) ? 'during' : 'after'
    }
    const first = firstDay.value
    const last = value.track.ends_on ?? first
    if (!first || today.value < first) return 'before'
    return last && today.value > last ? 'after' : 'during'
  })

  /** « dans 14 jours », « demain » ; vide le jour même et après. */
  const countdown = computed(() => {
    const first = firstDay.value
    if (!first || phase.value !== 'before') return ''
    const days = Math.round((Date.parse(`${first}T00:00:00Z`) - Date.parse(`${today.value}T00:00:00Z`)) / 86_400_000)
    if (days <= 0) return ''
    return new Intl.RelativeTimeFormat(locale.value, { numeric: 'auto' }).format(days, 'day')
  })

  /** Ce qui se passe en ce moment : le direct d'abord, sinon la première ligne en cours. */
  const current = computed(() => {
    const agenda = page.value?.agenda ?? []
    const states = agenda.map((entry) => ({ entry, state: entryState(entry) }))
    return states.find((item) => item.state === 'live') ?? states.find((item) => item.state === 'ongoing') ?? null
  })

  return { now, mounted, timezone, phase, countdown, entryState, current }
}
