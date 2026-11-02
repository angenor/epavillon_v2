<script setup lang="ts">
import type { IncidentListScreen, IncidentPayload, ManagedIncident } from '~/types/admin-incidents'
import type { RegieGesture, RegieJournalEntry, RegieScreen, RegieSession } from '~/types/admin-regie'
import type { EffectivePermission } from '~/types/identity'
import type { IsoDate, Uuid } from '~/types/shared'
import type { ComposerCommand, JournalUndoMode, RegieCancelChoice, RegieRestoreChoice } from '~/utils/regie'

/**
 * RÉGIE DU JOUR : `/admin/regie`, l'écran de l'équipe pendant une CdP.
 * Maquette : docs/maquettes/direction-condensee/regie.html.
 *
 * Chaque geste se défait : bandeau « Défaire » juste après, journal ensuite.
 * L'écran ne recompose aucun état : il relit l'API après chaque geste et à
 * chaque signal temps réel, et le courriel différé d'une annulation à son heure.
 */

definePageMeta({
  layout: 'admin',
  middleware: ['auth'],
  breadcrumb: [{ labelKey: 'nav.admin.regie' }],
})

const { t, locale } = useI18n()
const { tr } = useI18nText()
const dt = useDateTime()
const api = useApi()
const auth = useAuthStore()
const adminScope = useAdminScopeStore()
const route = useRoute()
const router = useRouter()
const toast = useToast()

useHead(() => ({ title: t('admin.regie.title') }))

await adminScope.ensureLoaded()

const eventId = computed(() => adminScope.currentEventId)
const requestedDay = computed<IsoDate | null>(() => {
  const value = route.query.jour
  return typeof value === 'string' && /^\d{4}-\d{2}-\d{2}$/.test(value) ? value : null
})

const { data: screen, status, error, refresh } = await useAsyncData<RegieScreen | null>(
  'admin-regie',
  async () => (eventId.value ? api.adminRegie.screen(eventId.value, requestedDay.value, adminScope.scope) : null),
  { watch: [eventId, requestedDay], lazy: true },
)

const { data: incidents, status: incidentsStatus } = await useAsyncData<IncidentListScreen | null>(
  'admin-regie-incidents',
  async () => (eventId.value ? api.adminIncidents.list(eventId.value, adminScope.scope) : null),
  { watch: [eventId], lazy: true },
)

const { data: granted } = await useAsyncData<EffectivePermission[]>(
  'admin-regie-permissions',
  async () => (auth.person ? api.identity.permissions(auth.person.id) : []),
  { default: () => [], lazy: true },
)

const canSchedule = computed(() => hasPermission(granted.value, 'programme.session.schedule', eventId.value))
const canPublishIncident = computed(() => hasPermission(granted.value, 'live.incident.publish', eventId.value))
const canManageMeeting = computed(() => hasPermission(granted.value, 'live.meeting.manage', eventId.value))

// L'horloge du pavillon. Partagée par le rendu serveur pour que l'hydratation lise la même heure.
const now = useState('admin-regie-now', () => Date.now())
let ticker: ReturnType<typeof setInterval> | null = null
const nuxtApp = useNuxtApp()
onMounted(() => {
  now.value = Date.now()
  ticker = setInterval(() => (now.value = Date.now()), 1000)
  // Un message d'incident s'écrit en français ET en anglais : l'autre locale doit être chargée.
  void Promise.all((['fr', 'en'] as const).map((code) => nuxtApp.$i18n.loadLocaleMessages(code)))
})
onBeforeUnmount(() => {
  if (ticker) clearInterval(ticker)
  if (noticeTimer) clearTimeout(noticeTimer)
  if (hostPoll) clearInterval(hostPoll)
})

const timezone = computed(() => screen.value?.timezone ?? 'UTC')
const zoneLabel = computed(() => screen.value?.zone_label?.trim() || null)
const sessions = computed(() => screen.value?.sessions ?? [])
const live = computed(() => liveSession(sessions.value))
const focus = computed(() => focusSession(sessions.value, now.value))
const next = computed(() => nextSession(sessions.value, now.value))
const alerts = computed(() => regieAlerts(sessions.value, now.value))
const activeIncidents = computed(() => (incidents.value?.rows ?? []).filter((row) => row.state === 'active'))
const editionTitle = computed(() => screen.value?.event_acronym || tr(screen.value?.event_title))
const time = (value: string | null): string => dt.time(value, timezone.value)
const nameOf = (session: RegieSession): string =>
  regieOrganizationsLabel(session.organizations, locale.value) || tr(session.title)

const onAir = computed(() => {
  if (live.value) return nameOf(live.value)
  const elsewhere = screen.value?.live_elsewhere
  if (!elsewhere) return null
  return elsewhere.title ? tr(elsewhere.title) : t('admin.regie.top.elsewhere')
})

const isForbidden = computed(() => (!adminScope.isLoading && !adminScope.canAdminister) || isForbiddenError(error.value))
const loadMessage = computed(() => (error.value ? apiErrorMessage(error.value, t) : undefined))

function selectDay(day: IsoDate): void {
  router.replace({ query: { ...route.query, jour: day } })
}

// ---------------------------------------------------------------------------
// Relire : après chaque geste, à chaque signal, à l'heure d'un courriel différé
// ---------------------------------------------------------------------------

async function reload(): Promise<void> {
  const id = eventId.value
  if (!id) return
  try {
    const [regie, list] = await Promise.all([
      api.adminRegie.screen(id, requestedDay.value, adminScope.scope),
      api.adminIncidents.list(id, adminScope.scope),
    ])
    screen.value = regie
    incidents.value = list
  } catch {
    // Le prochain signal rattrape ; l'écran reste affiché.
  }
}

useRealtime(() => regieTopics(eventId.value), () => void reload())

// Démarrer fait créer la réunion chez Zoom, en file : le signal du poste l'annonce, mais il n'est
// servi qu'avec `live.meeting.manage`. Sans lui, on relit tant que le lien d'animateur manque.
const HOST_POLL_MS = 5_000
const awaitingHost = computed(() => Boolean(live.value?.is_streamed && !live.value.meeting?.ready))
let hostPoll: ReturnType<typeof setInterval> | null = null
watch(
  awaitingHost,
  (waiting) => {
    if (hostPoll) clearInterval(hostPoll)
    hostPoll = import.meta.client && waiting ? setInterval(() => void reload(), HOST_POLL_MS) : null
  },
  { immediate: true },
)
watch(
  () => (live.value?.meeting?.ready ? live.value.session_id : null),
  (ready, before) => {
    if (ready && before === null) toast.success(t('admin.regie.now.hostArrived'))
  },
)

let noticeTimer: ReturnType<typeof setTimeout> | null = null
watch(
  () => (screen.value ? nextNoticeDue(screen.value.sessions, screen.value.journal, now.value) : null),
  (due) => {
    if (noticeTimer) clearTimeout(noticeTimer)
    noticeTimer = null
    if (import.meta.client && due) noticeTimer = setTimeout(() => void reload(), due - Date.now() + 1500)
  },
)

// ---------------------------------------------------------------------------
// Gestes
// ---------------------------------------------------------------------------

const busy = ref<string | null>(null)

function fail(message: string): void {
  toast.error(t('admin.regie.errors.title'), { message })
}

/** Un geste : son témoin, et le message de l'API tel quel s'il est refusé. */
async function run<T>(key: string, action: () => Promise<T>, onError: (message: string) => void = fail): Promise<T | null> {
  busy.value = key
  try {
    return await action()
  } catch (caught) {
    onError(apiErrorMessage(caught, t))
    return null
  } finally {
    busy.value = null
  }
}

interface UndoOffer {
  gesture: RegieGesture
  title: string
  /** Le message « annulée » publié avec le geste : défaire le retire aussi. */
  incidentId: Uuid | null
  shownAt: number
  deadline: number
}

const undoOffer = ref<UndoOffer | null>(null)

function offerUndo(gesture: RegieGesture, title: string, incidentId: Uuid | null = null): void {
  const shownAt = Date.now()
  undoOffer.value = { gesture, title, incidentId, shownAt, deadline: undoDeadline(gesture, shownAt) }
}

watch(now, (ms) => {
  if (undoOffer.value && ms >= undoOffer.value.deadline) undoOffer.value = null
})

const undoDetail = computed(() => {
  const offer = undoOffer.value
  if (!offer) return null
  if (offer.gesture.kind === 'session_cancelled' && offer.gesture.notify_at) {
    const left = countdownTo(Date.parse(offer.gesture.notify_at), now.value)
    const delay = t('admin.regie.undo.delay', { minutes: left.minutes, seconds: left.seconds })
    return t('admin.regie.undo.notifyIn', { count: offer.gesture.registrants, delay }, offer.gesture.registrants)
  }
  return offer.gesture.kind === 'live_ended' ? t('admin.regie.undo.resumeHint') : null
})
const undoRemaining = computed(() => {
  const offer = undoOffer.value
  if (!offer) return 0
  return Math.min(1, Math.max(0, (offer.deadline - now.value) / (offer.deadline - offer.shownAt)))
})

async function unpublishIncident(incidentId: Uuid, key = `unpublish:${incidentId}`): Promise<boolean> {
  const id = eventId.value
  if (!id) return false
  const result = await run(key, () =>
    api.adminIncidents.unpublish({ incident_id: incidentId, reason: t('admin.regie.incidents.unpublishReason') }, id, auth.person?.id ?? null),
  )
  if (!result) return false
  if (result.status !== 'unpublished') {
    fail(t(`admin.incident.form.error.${result.status}`))
    return false
  }
  return true
}

async function undoAction(actionId: Uuid, notify: boolean, key: string): Promise<boolean> {
  const gesture = await run(key, () => api.adminRegie.undo(actionId, { notify_registrants: notify }))
  if (!gesture) return false
  if (undoOffer.value?.gesture.action_id === actionId) undoOffer.value = null
  return true
}

async function undoFromBar(): Promise<void> {
  const offer = undoOffer.value
  if (!offer) return
  if (!(await undoAction(offer.gesture.action_id, false, `undo:${offer.gesture.action_id}`))) return
  if (offer.incidentId) await unpublishIncident(offer.incidentId)
  toast.success(t(offer.gesture.kind === 'live_ended' ? 'admin.regie.undo.resumed' : 'admin.regie.undo.done'))
  await reload()
}

async function confirmPresence(session: RegieSession): Promise<void> {
  const gesture = await run(`presence:${session.session_id}`, () => api.adminRegie.confirmPresence(session.session_id))
  if (!gesture) return
  offerUndo(gesture, t('admin.regie.undo.presence', { name: nameOf(session) }))
  await reload()
}

/** Réunion déjà prête : Zoom s'ouvre du même clic. Sinon son lien viendra s'afficher en grand. */
async function startLive(session: RegieSession): Promise<void> {
  const popup = session.meeting?.ready && canManageMeeting.value ? window.open('', '_blank') : null
  const gesture = await run(`start:${session.session_id}`, () => api.adminRegie.startLive(session.session_id))
  if (!gesture) {
    popup?.close()
    return
  }
  offerUndo(gesture, t('admin.regie.undo.started', { name: nameOf(session) }))
  if (popup) void sendHost(popup, session)
  await reload()
}

async function resumeLive(session: RegieSession): Promise<void> {
  if (!session.last_action_id) return
  if (await undoAction(session.last_action_id, false, `resume:${session.session_id}`)) {
    toast.success(t('admin.regie.undo.resumed'))
    await reload()
  }
}

async function createMeeting(session: RegieSession): Promise<void> {
  const meeting = await run(`meeting:${session.session_id}`, () => api.meetings.sync(session.session_id, { external_id: null }))
  if (!meeting) return
  toast.success(t('admin.regie.meetingCreated'))
  await reload()
}

/** L'onglet s'ouvre au clic, avant l'appel : ouvert après, un bloqueur de fenêtres le refuserait. */
async function openHost(session: RegieSession): Promise<void> {
  await sendHost(window.open('', '_blank'), session)
}

async function sendHost(popup: Window | null, session: RegieSession): Promise<void> {
  const link = await run(`host:${session.session_id}`, () => api.meetings.hostLink(session.session_id))
  if (!link) {
    popup?.close()
    return
  }
  if (popup) {
    popup.opener = null
    popup.location.href = link.url
  } else {
    window.open(link.url, '_blank', 'noopener')
  }
}

// --- Fin du direct -----------------------------------------------------------

const ending = ref<RegieSession | null>(null)
const endOpen = ref(false)
const dialogError = ref<string | null>(null)

function openEnd(session: RegieSession): void {
  ending.value = session
  dialogError.value = null
  endOpen.value = true
}

async function confirmEnd(): Promise<void> {
  const session = ending.value
  if (!session) return
  const gesture = await run('end', () => api.adminRegie.endLive(session.session_id), (message) => (dialogError.value = message))
  if (!gesture) return
  endOpen.value = false
  offerUndo(gesture, t('admin.regie.undo.ended', { name: nameOf(session) }))
  await reload()
}

// --- Annulation --------------------------------------------------------------

const cancelling = ref<RegieSession | null>(null)
const cancelOpen = ref(false)
const cancelReason = ref<string | null>(null)

function openCancel(session: RegieSession, reason: string | null = null): void {
  cancelling.value = session
  cancelReason.value = reason
  dialogError.value = null
  cancelOpen.value = true
}

function cancellationNotice(session: RegieSession): IncidentPayload {
  const timeIn = (code: 'fr-FR' | 'en-GB') => formatTime(session.starts_at, { timeZone: timezone.value, locale: code })
  return {
    scope: 'session',
    event_id: null,
    event_day_id: null,
    session_id: session.session_id,
    organization_id: null,
    incident_kind_code: 'cancellation',
    severity: 'error',
    title: null,
    message: {
      fr: t('admin.regie.dialogs.cancel.message', { time: timeIn('fr-FR') }, { locale: 'fr' }),
      en: t('admin.regie.dialogs.cancel.message', { time: timeIn('en-GB') }, { locale: 'en' }),
    },
    action_url: null,
    is_dismissible: false,
    display_from: new Date().toISOString(),
    display_until: Date.parse(session.ends_at) > Date.now() ? session.ends_at : null,
    publish: true,
  }
}

async function submitCancel(choice: RegieCancelChoice): Promise<void> {
  const session = cancelling.value
  const id = eventId.value
  if (!session || !id) return
  const gesture = await run(
    'cancel',
    () => api.adminRegie.cancel(session.session_id, { reason_code: choice.reason_code, note: choice.note, notify_registrants: choice.notify_registrants }),
    (message) => (dialogError.value = message),
  )
  if (!gesture) return
  cancelOpen.value = false
  let incidentId: Uuid | null = null
  if (choice.publish_incident) {
    const result = await run('cancel-incident', () =>
      api.adminIncidents.create({ ...cancellationNotice(session), from_event_id: id }, auth.person?.id ?? null),
    )
    if (result && result.status !== 'published') fail(t(`admin.incident.form.error.${result.status}`))
    incidentId = result?.incident?.incident_id ?? null
  }
  offerUndo(gesture, t('admin.regie.undo.cancelled', { time: time(session.starts_at) }), incidentId)
  await reload()
}

// --- Rétablissement ----------------------------------------------------------

const restoring = ref<{ session: RegieSession; actionId: Uuid } | null>(null)
const restoreOpen = ref(false)
const restoreIncident = computed<ManagedIncident | null>(() => {
  const target = restoring.value?.session.session_id
  return activeIncidents.value.find((row) => row.kind_code === 'cancellation' && row.target_id === target) ?? null
})

function openRestore(session: RegieSession, actionId: Uuid | null): void {
  if (!actionId) return
  restoring.value = { session, actionId }
  dialogError.value = null
  restoreOpen.value = true
}

async function submitRestore(choice: RegieRestoreChoice): Promise<void> {
  const target = restoring.value
  if (!target) return
  const incident = restoreIncident.value
  const gesture = await run(
    'restore',
    () => api.adminRegie.undo(target.actionId, { notify_registrants: choice.notify_registrants }),
    (message) => (dialogError.value = message),
  )
  if (!gesture) return
  restoreOpen.value = false
  if (choice.unpublish_incident && incident) await unpublishIncident(incident.incident_id)
  toast.success(t('admin.regie.undo.restored', { time: time(target.session.starts_at) }))
  await reload()
}

// --- Journal -----------------------------------------------------------------

async function actOnJournal(entry: RegieJournalEntry, mode: JournalUndoMode): Promise<void> {
  if (mode === 'restore') {
    const session = sessions.value.find((s) => s.session_id === entry.session_id)
    if (session) openRestore(session, entry.id)
    return
  }
  if (mode === 'unpublish') {
    if (await unpublishIncident(entry.id, `undo:${entry.id}`)) await reload()
    return
  }
  if (await undoAction(entry.id, false, `undo:${entry.id}`)) {
    toast.success(t(mode === 'resume' ? 'admin.regie.undo.resumed' : 'admin.regie.undo.done'))
    await reload()
  }
}

// --- Messages d'incident -----------------------------------------------------

const command = ref<ComposerCommand | null>(null)
const composerEl = ref<HTMLElement | null>(null)
const publishing = ref(false)
let commandNonce = 0

async function removeIncident(row: ManagedIncident): Promise<void> {
  if (await unpublishIncident(row.incident_id)) await reload()
}

async function reportOverrun(session: RegieSession): Promise<void> {
  const template = await run(`overrun:${session.session_id}`, () => api.adminIncidents.overrunTemplate(session.session_id))
  command.value = { nonce: ++commandNonce, template: 'overrun', sessionId: template?.session_id ?? session.session_id }
  composerEl.value?.scrollIntoView({ behavior: 'smooth', block: 'center' })
}

async function publishIncident(payload: IncidentPayload): Promise<void> {
  const id = eventId.value
  if (!id) return
  publishing.value = true
  try {
    const result = await api.adminIncidents.create({ ...payload, from_event_id: id }, auth.person?.id ?? null)
    if (result.status !== 'published') {
      fail(t(`admin.incident.form.error.${result.status}`))
      return
    }
    toast.success(t('admin.regie.composer.published'))
    command.value = { nonce: ++commandNonce, template: 'delay', sessionId: focus.value?.session_id ?? null }
    await reload()
  } catch (caught) {
    fail(apiErrorMessage(caught, t))
  } finally {
    publishing.value = false
  }
}
</script>

<template>
  <div data-direction="condensee" class="-mx-4 -mt-6 sm:-mx-6 sm:-mt-8" :class="undoOffer && 'pb-24'">
    <UiForbiddenState
      v-if="isForbidden"
      class="px-4 py-8 sm:px-6"
      :required-scope="t('admin.regie.forbidden')"
      action-to="/"
      :action-label="t('nav.admin.backToSite')"
    />

    <div v-else-if="!screen && status === 'pending'" class="grid gap-6 px-4 py-6 sm:px-6" :aria-label="t('common.a11y.loading')">
      <UiSkeletonLoader height="5rem" />
      <div class="grid items-start gap-6 xl:grid-cols-[minmax(0,1fr)_25rem]">
        <div class="grid gap-6">
          <UiSkeletonLoader height="16rem" />
          <UiSkeletonLoader height="24rem" />
        </div>
        <UiSkeletonLoader height="30rem" />
      </div>
    </div>

    <UiErrorState
      v-else-if="!screen && error"
      class="px-4 py-8 sm:px-6"
      :description="loadMessage"
      :retry-label="t('common.actions.retry')"
      :retrying="status === 'pending'"
      @retry="refresh()"
    />

    <UiEmptyState
      v-else-if="!screen"
      class="px-4 py-8 sm:px-6"
      icon="calendar"
      :title="t('admin.regie.noEvent.title')"
      :description="t('admin.regie.noEvent.description')"
    />

    <template v-else>
      <AdminRegieTopBar
        class="z-10 lg:sticky lg:top-(--admin-header-height)"
        :title="editionTitle"
        :days="screen.days"
        :day="screen.day"
        :today="screen.today"
        :timezone="timezone"
        :zone-label="zoneLabel"
        :now-ms="now"
        :on-air="onAir"
        :loading="status === 'pending'"
        @select-day="selectDay"
      />

      <div class="mx-auto grid max-w-[90rem] items-start gap-6 px-4 py-6 sm:px-6 xl:grid-cols-[minmax(0,1fr)_25rem]">
        <div class="grid min-w-0 gap-6">
          <AdminRegieNowPanel
            v-if="focus"
            :session="focus"
            :now-ms="now"
            :timezone="timezone"
            :zone-label="zoneLabel"
            :can-schedule="canSchedule"
            :can-manage-meeting="canManageMeeting"
            :can-publish-incident="canPublishIncident"
            :start-block="startLiveBlock(focus, sessions, screen.live_elsewhere)"
            :on-air="onAir"
            :busy="busy"
            @open-host="openHost(focus)"
            @end-live="openEnd(focus)"
            @report-overrun="reportOverrun(focus)"
            @start-live="startLive(focus)"
            @create-meeting="createMeeting(focus)"
          />

          <AdminRegieDayList
            v-if="sessions.length"
            :day="screen.day"
            :sessions="sessions"
            :live-elsewhere="screen.live_elsewhere"
            :next-id="next?.session_id ?? null"
            :on-air="onAir"
            :now-ms="now"
            :timezone="timezone"
            :zone-label="zoneLabel"
            :can-schedule="canSchedule"
            :can-manage-meeting="canManageMeeting"
            :busy="busy"
            @presence="confirmPresence"
            @absent="openCancel($event, 'no_show')"
            @cancel="openCancel($event)"
            @start-live="startLive"
            @end-live="openEnd"
            @create-meeting="createMeeting"
            @restore="openRestore($event, $event.cancellation?.action_id ?? null)"
            @resume="resumeLive"
          />
          <UiEmptyState
            v-else
            icon="calendar"
            :title="t('admin.regie.empty.title')"
            :description="t('admin.regie.empty.description')"
          />
        </div>

        <aside
          class="grid min-w-0 gap-4 xl:sticky xl:top-[calc(var(--admin-header-height)+6rem)] xl:max-h-[calc(100dvh-var(--admin-header-height)-7.5rem)] xl:overflow-y-auto"
          :aria-label="t('admin.regie.side')"
        >
          <AdminRegieJournal
            :entries="screen.journal"
            :is-today="screen.day === screen.today"
            :now-ms="now"
            :timezone="timezone"
            :can-schedule="canSchedule"
            :can-publish-incident="canPublishIncident"
            :busy="busy"
            @act="actOnJournal"
          />
          <AdminRegieWatchlist :alerts="alerts" :timezone="timezone" />
          <AdminRegieIncidents
            :rows="activeIncidents"
            :timezone="timezone"
            :can-publish="canPublishIncident"
            :busy="busy"
            :loading="!incidents && incidentsStatus === 'pending'"
            @unpublish="removeIncident"
          >
            <div v-if="canPublishIncident && eventId" ref="composerEl">
              <AdminRegieComposer
                :event-id="eventId"
                :sessions="sessions"
                :focus-id="focus?.session_id ?? null"
                :kinds="incidents?.kinds ?? []"
                :timezone="timezone"
                :now-ms="now"
                :submitting="publishing"
                :command="command"
                @publish="publishIncident"
              />
            </div>
          </AdminRegieIncidents>
        </aside>
      </div>

      <AdminRegieCancelDialog
        v-model:open="cancelOpen"
        :session="cancelling"
        :reasons="screen.reasons"
        :initial-reason="cancelReason"
        :timezone="timezone"
        :can-publish-incident="canPublishIncident"
        :submitting="busy === 'cancel' || busy === 'cancel-incident'"
        :error="dialogError"
        @submit="submitCancel"
      />
      <AdminRegieEndLiveDialog
        v-model:open="endOpen"
        :session="ending"
        :now-ms="now"
        :submitting="busy === 'end'"
        :error="dialogError"
        @confirm="confirmEnd"
      />
      <AdminRegieRestoreDialog
        v-model:open="restoreOpen"
        :session="restoring?.session ?? null"
        :incident="restoreIncident"
        :timezone="timezone"
        :can-publish-incident="canPublishIncident"
        :submitting="busy === 'restore'"
        :error="dialogError"
        @submit="submitRestore"
      />
      <AdminRegieUndoBar
        v-if="undoOffer"
        :title="undoOffer.title"
        :detail="undoDetail"
        :remaining="undoRemaining"
        :action-label="undoOffer.gesture.kind === 'live_ended' ? t('admin.regie.slot.resume') : t('admin.regie.undo.action')"
        :busy="busy === `undo:${undoOffer.gesture.action_id}`"
        @undo="undoFromBar"
        @close="undoOffer = null"
      />
    </template>
  </div>
</template>
