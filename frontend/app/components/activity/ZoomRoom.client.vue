<script setup lang="ts">
import type { MeetingAccess, PublicSessionMeeting } from '~/types/live'
import type { PublicScheduleRow } from '~/types/views'
import type { MeetingRoomStep } from '~/composables/useMeetingRoom'
import type { SuspensionViewType } from '@zoom/meetingsdk/embedded'
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
const route = useRoute()
const localePath = useLocalePath()
const embedded = useEmbeddedZoom()
const { phase, step, error, errorStatus, needsRegistration, waitlisted, joinUrl, prefetch, join, left, onRegistered } = useMeetingRoom(
  () => props.session,
)

const box = useTemplateRef<HTMLElement>('box')
const zoomRoot = useTemplateRef<HTMLElement>('zoomRoot')
const dialogOpen = ref(false)
const expanded = ref(false)

const inPage = computed(() => props.mode === 'stage' && embedded.value)
const loginTo = computed(() => `${localePath('auth-login')}?redirect=${encodeURIComponent(route.fullPath)}`)
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

/** La barre d'outils du SDK se pose sous la vidéo, dans la même racine. */
const TOOLBAR_PX = 56

/** Taille de la vidéo : le cadre, ou l'écran en mode agrandi. Nulle tant que la mise en page n'est pas faite. */
function videoSize(): { width: number; height: number } | null {
  const rect = box.value?.getBoundingClientRect()
  const width = Math.round(expanded.value ? window.innerWidth : (rect?.width ?? 0))
  const height = Math.round((expanded.value ? window.innerHeight : (rect?.height ?? 0)) - TOOLBAR_PX)
  return width > 0 && height > 0 ? { width, height } : null
}

function applyVideoSize(): void {
  const size = videoSize()
  if (size) client?.updateVideoOptions({ viewSizes: { default: size, ribbon: size } })
}

// Un client par tentative : créé et initialisé ici, détruit à la sortie ou à l'échec, jamais réinitialisé sur la même racine.
async function startEmbedded(access: MeetingAccess, setStep: (next: MeetingRoomStep) => void): Promise<void> {
  zoom = await loadZoomEmbedded()
  disposeEmbedded()
  const current = zoom.createClient()
  client = current
  await nextTick()
  const size = videoSize() ?? { width: 960, height: 540 - TOOLBAR_PX }
  try {
    await current.init({
      zoomAppRoot: zoomRoot.value ?? undefined,
      language: zoomLanguage(locale.value),
      patchJsMedia: true,
      leaveOnPageUnload: true,
      customize: {
        // Orateur : la galerie demande SharedArrayBuffer (en-têtes COOP/COEP), absent ici ; sans réglage, le SDK part en ruban.
        video: {
          isResizable: false,
          popper: { disableDraggable: true },
          defaultViewType: 'speaker' as SuspensionViewType,
          viewSizes: { default: size, ribbon: size },
        },
      },
    })
    setStep('joining')
    current.on('connection-change', onConnectionChange)
    await current.join({ ...zoomJoinFields(access), password: access.password })
  } catch (failure) {
    disposeEmbedded()
    throw failure
  }
  await nextTick()
  applyVideoSize()
  resize = new ResizeObserver(applyVideoSize)
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
  void setExpanded(false)
  // Hors du rappel du SDK : il finit de se fermer avant qu'on le détruise.
  setTimeout(disposeEmbedded, 0)
  left()
}

function enter(): void {
  void join(props.mode === 'page' ? startClientView : startEmbedded)
}

/**
 * Agrandir sans mettre le cadre en plein écran : les menus du SDK (participants,
 * réglages, discussion) se rendent dans `body` et resteraient derrière l'élément
 * en plein écran. Le cadre passe en fixe sur toute la fenêtre, et c'est le
 * document entier qui passe en plein écran.
 */
async function setExpanded(next: boolean): Promise<void> {
  if (expanded.value === next) return
  expanded.value = next
  document.documentElement.style.overflow = next ? 'hidden' : ''
  if (next && !document.fullscreenElement) await document.documentElement.requestFullscreen().catch(() => undefined)
  if (!next && document.fullscreenElement) await document.exitFullscreen().catch(() => undefined)
  await nextTick()
  applyVideoSize()
}

const onFullscreenChange = () => {
  if (!document.fullscreenElement && expanded.value) void setExpanded(false)
}
const onKey = (event: KeyboardEvent) => {
  if (event.key === 'Escape' && expanded.value && !document.fullscreenElement) void setExpanded(false)
}
const onWindowResize = () => expanded.value && applyVideoSize()
onMounted(() => {
  document.addEventListener('fullscreenchange', onFullscreenChange)
  document.addEventListener('keydown', onKey)
  window.addEventListener('resize', onWindowResize)
})
onBeforeUnmount(() => {
  document.removeEventListener('fullscreenchange', onFullscreenChange)
  document.removeEventListener('keydown', onKey)
  window.removeEventListener('resize', onWindowResize)
  if (expanded.value) {
    document.documentElement.style.overflow = ''
    if (document.fullscreenElement) void document.exitFullscreen().catch(() => undefined)
  }
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
  <!-- z-[60] : au-dessus de la barre du site (z-30) et du bandeau du billet (z-40), sous les menus du SDK (999 et plus). -->
  <div
    ref="box"
    class="bg-surface-inverse text-text-on-inverse"
    :class="expanded ? 'fixed! inset-0 z-[60]' : 'relative size-full'"
  >
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
          <UiButton v-else-if="errorStatus === 403 && needsRegistration" @click="dialogOpen = true">{{ t('zoom-room.register.action') }}</UiButton>
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
      :icon="expanded ? 'close' : 'monitor'"
      :label="t(expanded ? 'zoom-room.exitFullscreen' : 'zoom-room.fullscreen')"
      @click="setExpanded(!expanded)"
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
