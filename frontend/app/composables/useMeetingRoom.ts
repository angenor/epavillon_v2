import type { MaybeRefOrGetter } from 'vue'
import type { MeetingAccess, PublicSessionMeeting } from '~/types/live'
import type { PublicScheduleRow } from '~/types/views'

/** Ce que voit la personne devant la salle Zoom. */
export type MeetingRoomPhase = 'checking' | 'login' | 'register' | 'ready' | 'loading' | 'joined' | 'error'

/** Étapes de `loading`, dans l'ordre. */
export type MeetingRoomStep = 'access' | 'sdk' | 'joining'

/**
 * La salle s'ouvre (`opens_at`) et ferme (`closes_at` : la fin réelle de la réunion,
 * sinon une borne que l'API repousse tant qu'elle dure) ; `ended` la ferme aussi.
 * Seules les réunions Zoom vivantes comptent. Ne dit rien de qui est déjà dedans :
 * l'appelant garde la salle montée tant qu'on y est.
 */
export function isMeetingOpen(meeting: PublicSessionMeeting | null, now: number | null): boolean {
  if (!meeting || now === null || meeting.provider !== 'zoom') return false
  if (meeting.status === 'cancelled' || meeting.status === 'ended') return false
  return now >= Date.parse(meeting.opens_at) && now < Date.parse(meeting.closes_at)
}

const POLL_MS = 60_000
/** On commence à relire un peu avant l'ouverture, pour que le bouton paraisse à l'heure. */
const POLL_AHEAD_MS = 15 * 60_000

/**
 * La réunion publique d'une séance, relue chaque minute tant que la salle est
 * ouverte ou va l'être, et au retour sur l'onglet : `closes_at` recule tant que
 * Zoom dit la réunion en cours (les COP débordent). Un échec garde la valeur
 * connue ; `keep()` (la personne est dans la réunion) empêche qu'elle disparaisse.
 */
export function useSessionMeeting(session: MaybeRefOrGetter<PublicScheduleRow | null>, keep: () => boolean = () => false) {
  const api = useApi()
  const meeting = ref<PublicSessionMeeting | null>(null)
  const loaded = ref(false)
  let lastLoad = 0
  let timer: ReturnType<typeof setInterval> | undefined

  async function load(): Promise<void> {
    const current = toValue(session)
    if (!current?.is_streamed || current.temporal_state === 'cancelled') {
      loaded.value = true
      return
    }
    lastLoad = Date.now()
    try {
      const next = await api.meetings.forSession(current.id)
      if (current.id !== toValue(session)?.id || (next === null && keep())) return
      if (JSON.stringify(next) !== JSON.stringify(meeting.value)) meeting.value = next
    } catch {
      // Silencieux : la salle garde ce qu'elle savait.
    } finally {
      loaded.value = true
    }
  }

  function worthPolling(): boolean {
    if (keep()) return true
    const current = meeting.value
    if (!current || current.provider !== 'zoom' || current.status === 'cancelled') return false
    const at = Date.now()
    return at >= Date.parse(current.opens_at) - POLL_AHEAD_MS && at < Date.parse(current.closes_at)
  }

  const onReturn = () => {
    if (document.visibilityState === 'visible' && Date.now() - lastLoad > 5_000) void load()
  }

  onMounted(() => {
    void load()
    timer = setInterval(() => worthPolling() && void load(), POLL_MS)
    document.addEventListener('visibilitychange', onReturn)
    window.addEventListener('focus', onReturn)
  })
  onBeforeUnmount(() => {
    clearInterval(timer)
    document.removeEventListener('visibilitychange', onReturn)
    window.removeEventListener('focus', onReturn)
  })
  watch(
    () => toValue(session)?.id,
    (id, previous) => {
      if (id === previous) return
      meeting.value = null
      loaded.value = false
      if (import.meta.client) void load()
    },
  )

  return { meeting, loaded }
}

/** Sur ordinateur (écran large, souris), la réunion s'ouvre dans la page ; ailleurs, en pleine page. */
export function useEmbeddedZoom() {
  const embedded = ref(false)
  let query: MediaQueryList | undefined
  const update = () => (embedded.value = query?.matches ?? false)
  onMounted(() => {
    query = window.matchMedia('(min-width: 1024px) and (pointer: fine)')
    update()
    query.addEventListener('change', update)
  })
  onBeforeUnmount(() => query?.removeEventListener('change', update))
  return embedded
}

/**
 * L'entrée dans la réunion, commune à la vue intégrée et à la pleine page : qui
 * peut entrer, l'accès signé (demandé d'avance pour que le clic aille vite), la
 * présence notée une fois. La vue n'apporte que `start`, qui ouvre le SDK.
 */
export function useMeetingRoom(session: MaybeRefOrGetter<PublicScheduleRow>) {
  const api = useApi()
  const auth = useAuthStore()
  const { t } = useI18n()
  const sessionId = computed(() => toValue(session).id)
  const { mine, refresh } = useSessionRegistration(sessionId)

  const checked = ref(false)
  const step = ref<MeetingRoomStep | 'joined' | null>(null)
  const error = ref<string | null>(null)
  const errorStatus = ref<number | null>(null)
  const access = ref<MeetingAccess | null>(null)
  let presenceRecorded = false

  onMounted(async () => {
    await auth.ensureLoaded()
    if (auth.isAuthenticated) await refresh()
    checked.value = true
  })

  const needsRegistration = computed(() => toValue(session).registration_required !== false)
  // Entrée libre : toute personne connectée entre, sans inscription ni liste d'attente.
  const waitlisted = computed(() => needsRegistration.value && mine.value?.status === 'waitlisted')
  const joinUrl = computed(() => access.value?.join_url ?? null)

  const phase = computed<MeetingRoomPhase>(() => {
    if (!checked.value) return 'checking'
    if (!auth.isAuthenticated) return 'login'
    if (step.value === 'joined') return 'joined'
    if (step.value) return 'loading'
    if (error.value) return 'error'
    if (!mine.value && needsRegistration.value) return 'register'
    return 'ready'
  })

  async function fetchAccess(): Promise<MeetingAccess> {
    const current = access.value
    if (current && Date.parse(current.expires_at) - Date.now() > 60_000) return current
    access.value = await api.meetings.access(sessionId.value)
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
      if (failure.status === 403) void refresh()
      return
    }
    const reason = zoomFailureReason(failure)
    error.value = reason ? t('zoom-room.error.sdkReason', { reason }) : t('zoom-room.error.sdk')
  }

  function recordPresence(): void {
    const registration = mine.value
    if (presenceRecorded || !registration) return
    presenceRecorded = true
    api.meetings.recordJoin(registration.id).catch(() => undefined)
  }

  /** La réunion est finie ou quittée : retour au bouton d'entrée. */
  function left(): void {
    step.value = null
  }

  async function onRegistered(): Promise<void> {
    await refresh()
    error.value = null
  }

  return { phase, step, error, errorStatus, needsRegistration, waitlisted, joinUrl, prefetch, join, left, onRegistered }
}
