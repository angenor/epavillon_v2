import type { PublicEditionRow, PublicScheduleRow } from '~/types/views'

/** Le direct en cours sur la plateforme (règle n° 4 : un seul à la fois), avec son édition. */
export interface LiveNow {
  session: PublicScheduleRow
  edition: PublicEditionRow
}

const POLL_MS = 60_000
const DISMISSED_KEY = 'epavillon.live-banner-dismissed'

/**
 * Le bandeau « En direct » de toutes les pages publiques et la pastille de la
 * barre lisent le même état. Refermer vaut pour CE direct : le suivant rouvre
 * le bandeau.
 */
export function useLiveNow() {
  const live = useState<LiveNow | null>('live-now', () => null)
  const dismissedId = useState<string | null>('live-now-dismissed', () => null)
  const route = useRoute()
  const localePath = useLocalePath()
  const { link } = useProgrammeSession()

  /** Sur la page même du direct, ou celle de sa journée : on y est déjà. */
  const isHere = computed(() => {
    const current = live.value
    if (!current || route.params.edition !== current.edition.slug) return false
    const day = dayTrackOf(current.session.tracks)
    return route.params.session === current.session.slug || (day !== null && route.params.track === day.slug)
  })

  /** Une activité de journée se suit au direct de la journée ; les autres, au lecteur de leur page. */
  const to = computed(() => {
    const current = live.value
    if (!current) return ''
    const day = dayTrackOf(current.session.tracks)
    return day
      ? localePath({ name: 'track-edition-track', params: { edition: current.edition.slug, track: day.slug } })
      : `${link(current.edition.slug, current.session)}#direct`
  })

  const bannerShown = computed(() => live.value !== null && !isHere.value && dismissedId.value !== live.value.session.id)
  const pillShown = computed(() => live.value !== null && !isHere.value && dismissedId.value === live.value.session.id)

  function dismiss(): void {
    const id = live.value?.session.id ?? null
    dismissedId.value = id
    try {
      if (id) localStorage.setItem(DISMISSED_KEY, id)
    } catch {
      // Navigation privée : refermé jusqu'au rechargement seulement.
    }
  }

  return { live, to, bannerShown, pillShown, dismiss }
}

/**
 * La relecture, montée une seule fois par la mise en page : au chargement,
 * chaque minute quand l'onglet est visible, au retour sur l'onglet, et au
 * signal du direct des éditions en cours (démarrage ou fin par la régie).
 * Navigateur seulement : aucune page n'attend ce bandeau pour s'afficher.
 */
export function useLiveNowWatcher(): void {
  const api = useApi()
  const { live } = useLiveNow()
  const dismissedId = useState<string | null>('live-now-dismissed', () => null)
  const editions = ref<PublicEditionRow[]>([])
  let timer: ReturnType<typeof setInterval> | undefined

  async function load(): Promise<void> {
    try {
      if (editions.value.length === 0) editions.value = await api.events.publicList()
      const session = (await api.home.current()).find((row) => row.status === 'live') ?? null
      const edition = session ? editions.value.find((row) => row.id === session.event_id) : undefined
      live.value = session && edition ? { session, edition } : null
    } catch {
      // Silencieux : le bandeau garde ce qu'il savait.
    }
  }

  const onReturn = () => {
    if (document.visibilityState === 'visible') void load()
  }

  useRealtime(
    () =>
      publicEditionTopics(
        editions.value.filter((edition) => edition.temporal_state === 'ongoing').map((edition) => edition.id),
        ['live'],
      ),
    () => load(),
  )

  onMounted(() => {
    try {
      dismissedId.value = localStorage.getItem(DISMISSED_KEY)
    } catch {
      // Navigation privée : rien de retenu.
    }
    void load()
    timer = setInterval(() => document.visibilityState === 'visible' && void load(), POLL_MS)
    document.addEventListener('visibilitychange', onReturn)
  })
  onBeforeUnmount(() => {
    clearInterval(timer)
    document.removeEventListener('visibilitychange', onReturn)
  })
}
