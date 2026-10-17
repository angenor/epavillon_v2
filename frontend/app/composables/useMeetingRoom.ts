import type { MaybeRefOrGetter } from 'vue'
import type { MeetingAccess, PublicSessionMeeting } from '~/types/live'
import type { PublicScheduleRow } from '~/types/views'

/** Ce que voit la personne devant la salle Zoom. */
export type MeetingRoomPhase = 'checking' | 'login' | 'register' | 'ready' | 'loading' | 'joined' | 'error'

/** Étapes de `loading`, dans l'ordre. */
export type MeetingRoomStep = 'access' | 'sdk' | 'joining'

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

/**
 * La réunion publique d'une séance, relue tant que la page est ouverte (elle peut
 * apparaître, démarrer ou finir) et au retour sur l'onglet. Un échec garde la
 * valeur connue ; `keep()` (la personne est dans la réunion) empêche qu'elle disparaisse.
 */
export function useSessionMeeting(session: MaybeRefOrGetter<PublicScheduleRow | null>, keep: () => boolean = () => false) {
  const api = useApi()
  const meeting = ref<PublicSessionMeeting | null>(null)
  const loaded = ref(false)
  let lastLoad = 0
  let timer: ReturnType<typeof setTimeout> | undefined
  let mounted = false

  const watched = () => {
    const current = toValue(session)
    return Boolean(current?.is_streamed) && current?.temporal_state !== 'cancelled'
  }

  /** Vrai quand la réunion a changé. */
  async function load(): Promise<boolean> {
    const current = toValue(session)
    if (!current || !watched()) {
      loaded.value = true
      return false
    }
    lastLoad = Date.now()
    try {
      const next = await api.meetings.forSession(current.id)
      if (current.id !== toValue(session)?.id || (next === null && keep())) return false
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
      const changed = watched() && document.visibilityState === 'visible' ? await load() : false
      schedule(changed ? POLL_FAST_MS : POLL_SLOW_MS)
    }, delay)
  }

  const onReturn = () => {
    if (document.visibilityState === 'visible' && Date.now() - lastLoad > 5_000) void load().then((changed) => changed && schedule(POLL_FAST_MS))
  }

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

/**
 * L'entrée dans la réunion, dans la salle pleine page : qui
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
