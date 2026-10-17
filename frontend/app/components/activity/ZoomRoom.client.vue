<script setup lang="ts">
import type { MeetingAccess, PublicSessionMeeting } from '~/types/live'
import type { PublicScheduleRow } from '~/types/views'
import type { MeetingRoomStep } from '~/composables/useMeetingRoom'
import type { ZoomEmbedded, ZoomEmbeddedClient } from '~/utils/zoom-sdk'

/**
 * La salle Zoom d'une activité, sur l'aplat sombre du direct.
 *
 * `stage` : dans la page de l'activité. Sur ordinateur la réunion s'ouvre dans
 * le cadre (vue intégrée du SDK) ; ailleurs le bouton mène à la salle pleine page.
 * `page` : la salle pleine page (vue client du SDK), qui prend tout l'écran et
 * renvoie à `leaveUrl` en sortant.
 */

interface Props {
  session: PublicScheduleRow
  meeting: PublicSessionMeeting
  mode: 'stage' | 'page'
  /** Salle pleine page, pour `stage` hors ordinateur. */
  roomTo?: string
  /** Adresse absolue de l'activité, où la vue client renvoie en sortant. */
  leaveUrl?: string
  zoneLabel?: string
}

const props = defineProps<Props>()
/** Vrai de l'entrée à la sortie : le parent ne démonte pas la salle pendant ce temps. */
const emit = defineEmits<{ 'update:inMeeting': [value: boolean] }>()

const { t, locale } = useI18n()
const { tr } = useI18nText()
const { timeWithZone } = useDateTime()
const route = useRoute()
const localePath = useLocalePath()
const embedded = useEmbeddedZoom()
const { phase, step, error, errorStatus, waitlisted, joinUrl, prefetch, join, left, onRegistered } = useMeetingRoom(
  () => props.session,
)

const box = useTemplateRef<HTMLElement>('box')
const zoomRoot = useTemplateRef<HTMLElement>('zoomRoot')
const dialogOpen = ref(false)
const fullscreen = ref(false)

const inPage = computed(() => props.mode === 'stage' && embedded.value)
const loginTo = computed(() => `${localePath('auth-login')}?redirect=${encodeURIComponent(route.fullPath)}`)
const windowLabel = computed(() => timeWithZone(props.meeting.opens_at, props.session.timezone, props.zoneLabel))
const stepLabel = computed(() => t(`zoom-room.step.${step.value === 'joined' || !step.value ? 'joining' : step.value}`))

let zoom: ZoomEmbedded | null = null
let client: ZoomEmbeddedClient | null = null
let resize: ResizeObserver | undefined

// Le SDK s'annonce lourd : on le charge pendant que la personne lit, pour que le clic garde son geste (le son en dépend).
watch(
  phase,
  (current) => {
    if (current !== 'ready' || (props.mode === 'stage' && !inPage.value)) return
    prefetch()
    if (inPage.value) void loadZoomEmbedded().catch(() => undefined)
  },
  { immediate: true },
)
watch(
  () => phase.value === 'loading' || phase.value === 'joined',
  (active) => emit('update:inMeeting', active),
)

function videoSize() {
  const rect = box.value?.getBoundingClientRect()
  const width = Math.round(rect?.width ?? 960)
  // La barre d'outils du SDK se pose sous la vidéo.
  const height = Math.max(Math.round((rect?.height ?? 540) - 56), 180)
  return { width, height }
}

// Un client par tentative : créé et initialisé ici, détruit à la sortie ou à l'échec, jamais réinitialisé sur la même racine.
async function startEmbedded(access: MeetingAccess, setStep: (next: MeetingRoomStep) => void): Promise<void> {
  zoom = await loadZoomEmbedded()
  disposeEmbedded()
  const current = zoom.createClient()
  client = current
  try {
    await current.init({
      zoomAppRoot: zoomRoot.value ?? undefined,
      language: zoomLanguage(locale.value),
      patchJsMedia: true,
      leaveOnPageUnload: true,
      customize: {
        video: { isResizable: false, popper: { disableDraggable: true }, viewSizes: { default: videoSize() } },
      },
    })
    setStep('joining')
    current.on('connection-change', onConnectionChange)
    await current.join({ ...zoomJoinFields(access), password: access.password })
  } catch (failure) {
    disposeEmbedded()
    throw failure
  }
  resize = new ResizeObserver(() => client?.updateVideoOptions({ viewSizes: { default: videoSize() } }))
  if (box.value) resize.observe(box.value)
}

function disposeEmbedded(): void {
  resize?.disconnect()
  resize = undefined
  client?.off('connection-change', onConnectionChange)
  if (client) zoom?.destroyClient()
  client = null
}

async function startClientView(access: MeetingAccess, setStep: (next: MeetingRoomStep) => void): Promise<void> {
  const zoomMtg = await loadZoomClientView()
  zoomMtg.preLoadWasm()
  zoomMtg.prepareWebSDK()
  await zoomMtg.i18n.load(zoomLanguage(locale.value))
  const root = document.getElementById('zmmtg-root')
  if (root) root.style.display = 'block'
  try {
    await new Promise((resolve, reject) =>
      zoomMtg.init({ leaveUrl: props.leaveUrl ?? window.location.href, patchJsMedia: true, leaveOnPageUnload: true, success: resolve, error: reject }),
    )
    setStep('joining')
    await new Promise((resolve, reject) =>
      zoomMtg.join({ ...zoomJoinFields(access), passWord: access.password, success: resolve, error: reject }),
    )
  } catch (failure) {
    if (root) root.style.display = 'none'
    throw failure
  }
}

function onConnectionChange(payload: { state?: string }): void {
  if (phase.value === 'joined' && (payload.state === 'Closed' || payload.state === 'Fail')) stopEmbedded()
}

function stopEmbedded(): void {
  if (document.fullscreenElement) void document.exitFullscreen().catch(() => undefined)
  // Hors du rappel du SDK : il finit de se fermer avant qu'on le détruise.
  setTimeout(disposeEmbedded, 0)
  left()
}

function enter(): void {
  void join(props.mode === 'page' ? startClientView : startEmbedded)
}

async function toggleFullscreen(): Promise<void> {
  if (document.fullscreenElement) await document.exitFullscreen().catch(() => undefined)
  else await box.value?.requestFullscreen().catch(() => undefined)
}

const onFullscreen = () => (fullscreen.value = document.fullscreenElement === box.value)
onMounted(() => document.addEventListener('fullscreenchange', onFullscreen))
onBeforeUnmount(() => {
  document.removeEventListener('fullscreenchange', onFullscreen)
  emit('update:inMeeting', false)
  const current = client
  if (!current) return
  const joined = phase.value === 'joined'
  void (async () => {
    if (joined) await current.leaveMeeting().catch(() => undefined)
    disposeEmbedded()
  })()
})
</script>

<template>
  <div ref="box" class="relative size-full bg-surface-inverse text-text-on-inverse">
    <div v-if="inPage" ref="zoomRoot" class="absolute inset-0" />

    <div
      v-if="phase !== 'joined' || !inPage"
      class="relative flex size-full flex-col items-center justify-center gap-4 bg-surface-inverse px-5 py-8 text-center sm:px-8"
      :aria-busy="phase === 'checking' || phase === 'loading' ? 'true' : undefined"
    >
      <span class="inline-flex size-14 items-center justify-center rounded-full bg-live text-live-contrast sm:size-16" aria-hidden="true">
        <UiIcon name="video" size="1.75rem" />
      </span>
      <div>
        <p class="text-xs font-bold text-text-on-inverse-muted uppercase" :style="{ letterSpacing: 'var(--tracking-caps)' }">
          {{ t('zoom-room.label') }}
        </p>
        <p class="mt-1 text-sm text-text-on-inverse-muted">{{ t('zoom-room.window', { time: windowLabel }) }}</p>
      </div>

      <p v-if="phase === 'checking'" class="flex items-center gap-2 text-sm" role="status">
        <UiSpinner size="1rem" />{{ t('zoom-room.checking') }}
      </p>

      <template v-else-if="phase === 'login'">
        <p class="max-w-md text-sm">{{ t('zoom-room.login.message') }}</p>
        <UiButton :to="loginTo" size="lg">{{ t('zoom-room.login.action') }}</UiButton>
      </template>

      <template v-else-if="phase === 'register'">
        <p class="max-w-md text-sm">{{ t('zoom-room.register.message') }}</p>
        <UiButton size="lg" @click="dialogOpen = true">{{ t('zoom-room.register.action') }}</UiButton>
      </template>

      <template v-else-if="phase === 'ready'">
        <p class="max-w-md text-sm">{{ t('zoom-room.ready') }}</p>
        <NuxtLink
          v-if="props.mode === 'stage' && !embedded && props.roomTo"
          :to="props.roomTo"
          class="inline-flex min-h-12 items-center gap-2.5 rounded-md bg-live px-6 text-base font-bold text-live-contrast"
        >
          <UiIcon name="video" size="1.125rem" />
          {{ t('zoom-room.join') }}
        </NuxtLink>
        <button
          v-else
          type="button"
          class="inline-flex min-h-12 cursor-pointer items-center gap-2.5 rounded-md bg-live px-6 text-base font-bold text-live-contrast"
          @click="enter"
        >
          <UiIcon name="video" size="1.125rem" />
          {{ t('zoom-room.join') }}
        </button>
      </template>

      <p v-else-if="phase === 'loading'" class="flex items-center gap-2 text-sm" role="status">
        <UiSpinner size="1rem" />{{ stepLabel }}
      </p>

      <p v-else-if="phase === 'joined'" class="text-sm" role="status">{{ t('zoom-room.joined') }}</p>

      <template v-else-if="phase === 'error'">
        <p class="max-w-md text-sm" role="alert">{{ error }}</p>
        <div class="flex flex-wrap justify-center gap-2">
          <UiButton v-if="errorStatus === 401" :to="loginTo">{{ t('zoom-room.login.action') }}</UiButton>
          <UiButton v-else-if="errorStatus === 403" @click="dialogOpen = true">{{ t('zoom-room.register.action') }}</UiButton>
          <UiButton variant="inverse" icon="refresh" @click="enter">{{ t('zoom-room.retry') }}</UiButton>
        </div>
      </template>

      <p v-if="waitlisted && (phase === 'ready' || phase === 'error')" class="max-w-md text-[0.8125rem] text-text-on-inverse-muted">
        {{ t('zoom-room.waitlisted') }}
      </p>

      <UiButton v-if="joinUrl && phase !== 'checking'" variant="inverse" icon="external-link" :href="joinUrl">
        {{ t('zoom-room.fallback') }}
      </UiButton>
    </div>

    <UiButton
      v-if="phase === 'joined' && inPage"
      class="absolute top-2 right-2 z-10"
      variant="inverse"
      icon-only
      :icon="fullscreen ? 'close' : 'monitor'"
      :label="t(fullscreen ? 'zoom-room.exitFullscreen' : 'zoom-room.fullscreen')"
      @click="toggleFullscreen"
    />

    <ActivityRegistrationDialog
      v-model:open="dialogOpen"
      :session-id="props.session.id"
      :session-title="tr(props.session.title)"
      :timezone="props.session.timezone"
      :timezone-label="props.zoneLabel"
      @done="onRegistered"
    />
  </div>
</template>
