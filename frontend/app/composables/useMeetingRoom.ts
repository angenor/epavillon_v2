import type { MaybeRefOrGetter } from 'vue'
import type { MeetingAccess, PublicSessionMeeting } from '~/types/live'
import type { PublicScheduleRow } from '~/types/views'

/** Ce que voit la personne devant la salle Zoom. */
export type MeetingRoomPhase = 'checking' | 'login' | 'register' | 'ready' | 'loading' | 'joined' | 'error'

/** Étapes de `loading`, dans l'ordre. */
export type MeetingRoomStep = 'access' | 'sdk' | 'joining'

/**
 * La salle s'ouvre (`opens_at`) et ferme (`closes_at`, la borne de l'accès) d'elle-même ;
 * seules les réunions Zoom vivantes comptent. Ne dit rien de qui est déjà dedans :
 * l'appelant garde la salle montée tant qu'on y est.
 */
export function isMeetingOpen(meeting: PublicSessionMeeting | null, now: number | null): boolean {
  if (!meeting || now === null || meeting.provider !== 'zoom') return false
  if (meeting.status === 'cancelled' || meeting.status === 'ended') return false
  return now >= Date.parse(meeting.opens_at) && now < Date.parse(meeting.closes_at)
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
  const waitlisted = computed(() => mine.value?.status === 'waitlisted')
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

  return { phase, step, error, errorStatus, waitlisted, joinUrl, prefetch, join, left, onRegistered }
}
