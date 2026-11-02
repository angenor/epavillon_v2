import type { MaybeRefOrGetter } from 'vue'
import type { MeetingAccess, PublicSessionMeeting } from '~/types/live'
import type { PublicScheduleRow } from '~/types/views'

/** Ce que voit la personne devant la salle Zoom. */
export type MeetingRoomPhase = 'checking' | 'login' | 'register' | 'ready' | 'loading' | 'joined' | 'error'

/** Étapes de `loading`, dans l'ordre. */
export type MeetingRoomStep = 'access' | 'sdk' | 'joining'

/**
 * La salle d'une activité, ou celle d'une journée (journée spéciale, atelier) :
 * un seul direct pour toute la journée, à qui s'est inscrit à la journée.
 */
export type MeetingRoomTarget = { kind: 'session'; session: PublicScheduleRow } | { kind: 'day'; trackId: string }

/**
 * Pas de fenêtre horaire : la salle est ouverte dès que la séance a une réunion Zoom
 * programmée, en cours ou terminée (avant le début, Zoom fait patienter ; après la
 * fin prévue, elle peut durer). `opens_at` et `closes_at` ne sont qu'indicatifs.
 * Ne dit rien de qui est déjà dedans : l'appelant garde la salle montée tant qu'on y est.
 */
export function isMeetingJoinable(meeting: PublicSessionMeeting | null): boolean {
  if (!meeting || meeting.provider !== 'zoom') return false
  return meeting.status === 'scheduled' || meeting.status === 'started' || meeting.status === 'ended'
}

/** Relecture rapprochée après un changement, espacée tant que rien ne bouge. */
const POLL_FAST_MS = 60_000
const POLL_SLOW_MS = 120_000

interface PolledMeetingSource {
  /** Séance ou fil. */
  id: string
  eventId: string
  /** Faux : rien à relire (séance non diffusée, annulée). */
  watched: boolean
}

/**
 * Une réunion publique relue tant que la page est ouverte (elle peut apparaître,
 * démarrer ou finir) et au retour sur l'onglet. Un échec garde la valeur connue ;
 * `keep()` (la personne est dans la réunion) empêche qu'elle disparaisse.
 */
function usePolledMeeting(
  source: () => PolledMeetingSource | null,
  read: (id: string) => Promise<PublicSessionMeeting | null>,
  keep: () => boolean,
) {
  const meeting = ref<PublicSessionMeeting | null>(null)
  const loaded = ref(false)
  let lastLoad = 0
  let timer: ReturnType<typeof setTimeout> | undefined
  let mounted = false

  /** Vrai quand la réunion a changé. */
  async function load(): Promise<boolean> {
    const current = source()
    if (!current?.watched) {
      loaded.value = true
      return false
    }
    lastLoad = Date.now()
    try {
      const next = await read(current.id)
      if (current.id !== source()?.id || (next === null && keep())) return false
      if (JSON.stringify(next) === JSON.stringify(meeting.value)) return false
      meeting.value = next
      return true
    } catch {
      // Silencieux : la salle garde ce qu'elle savait.
      return false
    } finally {
      loaded.value = true
    }
  }

  function schedule(delay: number): void {
    clearTimeout(timer)
    if (!mounted) return
    timer = setTimeout(async () => {
      const changed = source()?.watched && document.visibilityState === 'visible' ? await load() : false
      schedule(changed ? POLL_FAST_MS : POLL_SLOW_MS)
    }, delay)
  }

  const onReturn = () => {
    if (document.visibilityState === 'visible' && Date.now() - lastLoad > 5_000) void load().then((changed) => changed && schedule(POLL_FAST_MS))
  }

  // La réunion démarre ou finit avec la séance : le signal du direct la relit sans attendre la minute.
  useRealtime(
    () => publicEditionTopics([source()?.eventId], ['live']),
    () => load().then((changed) => changed && schedule(POLL_FAST_MS)),
  )

  onMounted(() => {
    mounted = true
    void load()
    schedule(POLL_FAST_MS)
    document.addEventListener('visibilitychange', onReturn)
    window.addEventListener('focus', onReturn)
  })
  onBeforeUnmount(() => {
    mounted = false
    clearTimeout(timer)
    document.removeEventListener('visibilitychange', onReturn)
    window.removeEventListener('focus', onReturn)
  })
  watch(
    () => source()?.id,
    (id, previous) => {
      if (id === previous) return
      meeting.value = null
      loaded.value = false
      if (import.meta.client) void load()
    },
  )

  return { meeting, loaded }
}

/** La réunion publique d'une séance. */
export function useSessionMeeting(session: MaybeRefOrGetter<PublicScheduleRow | null>, keep: () => boolean = () => false) {
  const api = useApi()
  return usePolledMeeting(
    () => {
      const current = toValue(session)
      if (!current) return null
      return { id: current.id, eventId: current.event_id, watched: Boolean(current.is_streamed) && current.temporal_state !== 'cancelled' }
    },
    (id) => api.meetings.forSession(id),
    keep,
  )
}

/** La réunion unique d'une journée : la même de l'ouverture à la clôture. */
export function useTrackMeeting(track: MaybeRefOrGetter<{ id: string; eventId: string } | null>, keep: () => boolean = () => false) {
  const api = useApi()
  return usePolledMeeting(
    () => {
      const current = toValue(track)
      return current ? { ...current, watched: true } : null
    },
    (id) => api.meetings.forTrack(id),
    keep,
  )
}

/**
 * L'entrée dans la réunion, dans la salle pleine page : qui
 * peut entrer, l'accès signé (demandé d'avance pour que le clic aille vite), la
 * présence notée une fois. La vue n'apporte que `start`, qui ouvre le SDK.
 *
 * Pour une journée, c'est l'inscription à la journée qui ouvre la salle, et
 * l'accès est celui de la journée ; la présence, l'API la note elle-même.
 */
export function useMeetingRoom(target: MaybeRefOrGetter<MeetingRoomTarget>) {
  const api = useApi()
  const auth = useAuthStore()
  const { t } = useI18n()
  const current = computed(() => toValue(target))
  // Le genre de salle ne change pas sous un composant monté : une seule des deux inscriptions est lue.
  const forDay = current.value.kind === 'day'
  const sessionId = computed(() => (current.value.kind === 'session' ? current.value.session.id : ''))
  const trackId = computed(() => (current.value.kind === 'day' ? current.value.trackId : null))
  const sessionRegistration = forDay ? null : useSessionRegistration(sessionId)
  const dayRegistration = forDay ? useDayRegistration(trackId) : null

  const checked = ref(false)
  const step = ref<MeetingRoomStep | 'joined' | null>(null)
  const error = ref<string | null>(null)
  const errorStatus = ref<number | null>(null)
  const access = ref<MeetingAccess | null>(null)
  let presenceRecorded = false

  async function refreshRegistration(): Promise<void> {
    if (sessionRegistration) await sessionRegistration.refresh()
    if (dayRegistration) await dayRegistration.refresh(true)
  }

  onMounted(async () => {
    await auth.ensureLoaded()
    if (auth.isAuthenticated) await refreshRegistration()
    checked.value = true
  })

  const needsRegistration = computed(() => {
    const value = current.value
    return value.kind === 'day' || value.session.registration_required !== false
  })
  const registered = computed(() =>
    dayRegistration ? dayRegistration.mine.value !== null : sessionRegistration?.mine.value != null,
  )
  // Entrée libre : toute personne connectée entre, sans inscription ni liste d'attente.
  const waitlisted = computed(() => needsRegistration.value && sessionRegistration?.mine.value?.status === 'waitlisted')
  const joinUrl = computed(() => access.value?.join_url ?? null)

  const phase = computed<MeetingRoomPhase>(() => {
    if (!checked.value) return 'checking'
    if (!auth.isAuthenticated) return 'login'
    if (step.value === 'joined') return 'joined'
    if (step.value) return 'loading'
    if (error.value) return 'error'
    if (!registered.value && needsRegistration.value) return 'register'
    return 'ready'
  })

  async function fetchAccess(): Promise<MeetingAccess> {
    const known = access.value
    if (known && Date.parse(known.expires_at) - Date.now() > 60_000) return known
    access.value = trackId.value ? await api.meetings.trackAccess(trackId.value) : await api.meetings.access(sessionId.value)
    return access.value
  }

  /** Silencieux : le clic redemandera, et dira pourquoi. */
  function prefetch(): void {
    fetchAccess().catch(() => undefined)
  }

  async function join(start: (access: MeetingAccess, setStep: (next: MeetingRoomStep) => void) => Promise<void>): Promise<void> {
    if (step.value) return
    error.value = null
    errorStatus.value = null
    step.value = 'access'
    try {
      const granted = await fetchAccess()
      step.value = 'sdk'
      await start(granted, (next) => (step.value = next))
      step.value = 'joined'
      recordPresence()
    } catch (failure) {
      step.value = null
      fail(failure)
    }
  }

  function fail(failure: unknown): void {
    if (failure instanceof ApiRequestError || failure instanceof ApiUnreachableError) {
      error.value = apiErrorMessage(failure, t)
      errorStatus.value = failure.status
      if (failure.status === 403) void refreshRegistration()
      return
    }
    const reason = zoomFailureReason(failure)
    error.value = reason ? t('zoom-room.error.sdkReason', { reason }) : t('zoom-room.error.sdk')
  }

  function recordPresence(): void {
    const registration = sessionRegistration?.mine.value
    if (presenceRecorded || !registration) return
    presenceRecorded = true
    api.meetings.recordJoin(registration.id).catch(() => undefined)
  }

  /** La réunion est finie ou quittée : retour au bouton d'entrée. */
  function left(): void {
    step.value = null
  }

  async function onRegistered(): Promise<void> {
    await refreshRegistration()
    error.value = null
  }

  /** Salle d'une journée : ouvre la fenêtre d'inscription à la journée montée par la page. */
  function openDayRegistration(): void {
    dayRegistration?.openDialog()
  }

  return { phase, step, error, errorStatus, needsRegistration, waitlisted, joinUrl, prefetch, join, left, onRegistered, openDayRegistration }
}
