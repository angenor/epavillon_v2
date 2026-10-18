<script setup lang="ts">
/**
 * Ce que la vue client de Zoom ne dit pas assez, pendant la réunion.
 *
 * Écran noir : le SDK affiche un avis passager (image ou son non connectés) sans
 * émettre d'événement. On le repère à son texte, on recharge une fois après un
 * décompte, puis on propose de réessayer ou d'ouvrir l'application Zoom.
 *
 * Partage d'écran : rien ne change dans la page, contrairement au logiciel, qui
 * se range. On ne cache pas la réunion (demande du 17/10) : une pastille et le
 * titre de l'onglet disent que le partage est en cours.
 *
 * Application Zoom : un lien toujours là, 30 s après l'ouverture de la salle,
 * pour qui préfère le logiciel ou reste bloqué au chargement.
 */

interface Props {
  failureTexts: string[]
  joinUrl: string | null
  sessionId: string
}

const props = defineProps<Props>()
const { t } = useI18n()

const RETRY_DELAY_S = 5
/** Compté depuis le chargement de la page : on arrive dans la salle par un lien simple. */
const APP_LINK_DELAY_MS = 30_000
/** Un second échec dans ce délai après le rechargement automatique ne relance plus rien. */
const RETRY_MEMORY_MS = 3 * 60_000
// Barre du SDK pour qui partage, toujours dans la page et cachée par `--hidden` hors partage, et son bouton d'arrêt (6.5.0).
const SHARER_BAR = '.sharer-controlbar-container:not(.sharer-controlbar-container--hidden)'
const SHARER_STOP = '.sharer-controlbar-container__stop'

const failure = ref<'none' | 'countdown' | 'stuck'>('none')
const secondsLeft = ref(RETRY_DELAY_S)
const sharing = ref(false)
const appLinkShown = ref(false)
let appLinkTimer: ReturnType<typeof setTimeout> | undefined
let countdown: ReturnType<typeof setInterval> | undefined
let shareCheck: ReturnType<typeof setTimeout> | undefined
let observer: MutationObserver | undefined
let titleBeforeShare: string | null = null

const retryKey = computed(() => `zoom-room-retry:${props.sessionId}`)

function retriedRecently(): boolean {
  try {
    const at = Number(sessionStorage.getItem(retryKey.value))
    return Number.isFinite(at) && Date.now() - at < RETRY_MEMORY_MS
  } catch {
    return false
  }
}

function reload(): void {
  clearInterval(countdown)
  const url = new URL(window.location.href)
  url.searchParams.set('entrer', '1')
  window.location.replace(url.href)
}

function autoRetry(): void {
  try {
    sessionStorage.setItem(retryKey.value, String(Date.now()))
  } catch {
    // Sans stockage, l'essai suivant relancera encore : préférable à rester sur un écran noir.
  }
  reload()
}

function onFailure(): void {
  if (failure.value !== 'none') return
  if (retriedRecently()) {
    failure.value = 'stuck'
    return
  }
  failure.value = 'countdown'
  secondsLeft.value = RETRY_DELAY_S
  countdown = setInterval(() => {
    secondsLeft.value -= 1
    if (secondsLeft.value <= 0) autoRetry()
  }, 1000)
}

function cancelRetry(): void {
  clearInterval(countdown)
  failure.value = 'stuck'
}

function dismiss(): void {
  clearInterval(countdown)
  failure.value = 'none'
}

function mentionsFailure(node: Node): boolean {
  const text = node.textContent
  return Boolean(text) && props.failureTexts.some((message) => text!.includes(message))
}

function checkShare(): void {
  const next = [...document.querySelectorAll(SHARER_BAR)].some((bar) => bar.getClientRects().length > 0)
  if (next === sharing.value) return
  sharing.value = next
  if (next) {
    titleBeforeShare = document.title
    document.title = `${t('zoom-room.alerts.share.tab')} · ${titleBeforeShare}`
  } else if (titleBeforeShare !== null) {
    document.title = titleBeforeShare
    titleBeforeShare = null
  }
}

function stopSharing(): void {
  const stop = document.querySelector<HTMLElement>(SHARER_STOP)
  ;(stop?.querySelector<HTMLElement>('button') ?? stop)?.click()
}

onMounted(() => {
  appLinkTimer = setTimeout(() => (appLinkShown.value = true), Math.max(0, APP_LINK_DELAY_MS - performance.now()))
  observer = new MutationObserver((mutations) => {
    if (failure.value === 'none' && props.failureTexts.length) {
      const failed = mutations.some((mutation) =>
        mutation.type === 'characterData' ? mentionsFailure(mutation.target) : [...mutation.addedNodes].some(mentionsFailure),
      )
      if (failed) onFailure()
    }
    clearTimeout(shareCheck)
    shareCheck = setTimeout(checkShare, 200)
  })
  // `class` : la barre de partage apparaît et disparaît par sa classe, sans changer de place dans la page.
  observer.observe(document.body, { childList: true, subtree: true, characterData: true, attributes: true, attributeFilter: ['class'] })
  if (mentionsFailure(document.body)) onFailure()
  checkShare()
})

onBeforeUnmount(() => {
  observer?.disconnect()
  clearInterval(countdown)
  clearTimeout(shareCheck)
  clearTimeout(appLinkTimer)
  if (titleBeforeShare !== null) document.title = titleBeforeShare
})
</script>

<template>
  <Teleport to="body">
    <div id="zoom-room-alerts">
      <!-- Au-dessus de `#zmmtg-root` et de ses menus (MUI, 1300 et plus). -->
      <div
        v-if="sharing"
        class="fixed top-14 left-3 z-[3000] flex items-center gap-3 rounded-md border border-border-subtle bg-surface-raised py-1.5 pr-1.5 pl-3 text-sm text-text shadow-lg"
        role="status"
      >
        <span class="size-2.5 shrink-0 rounded-full bg-live motion-safe:animate-pulse" aria-hidden="true" />
        <span class="font-bold">{{ t('zoom-room.alerts.share.status') }}</span>
        <UiButton variant="secondary" @click="stopSharing">{{ t('zoom-room.alerts.share.stop') }}</UiButton>
      </div>

      <!-- En haut à droite : sous l'en-tête de Zoom, à l'écart de sa barre d'outils et de sa barre de partage. -->
      <div v-if="appLinkShown && props.joinUrl" class="fixed top-14 right-3 z-[3000] rounded-md bg-surface-raised shadow-lg">
        <UiButton variant="secondary" icon="external-link" :href="props.joinUrl">{{ t('zoom-room.fallback') }}</UiButton>
      </div>

      <div
        v-if="failure !== 'none'"
        class="fixed inset-0 z-[3000] flex items-center justify-center bg-scrim/60 p-4"
      >
        <div class="w-full max-w-md rounded-lg border border-border-subtle bg-surface-raised p-6 text-text shadow-lg" role="alert">
          <p class="text-lg font-bold">{{ t('zoom-room.alerts.failure.title') }}</p>
          <p v-if="failure === 'countdown'" class="mt-2 text-sm text-text-muted">
            {{ t('zoom-room.alerts.failure.countdown', { seconds: secondsLeft }) }}
          </p>
          <p v-else class="mt-2 text-sm text-text-muted">{{ t('zoom-room.alerts.failure.stuck') }}</p>

          <div class="mt-5 flex flex-wrap gap-2">
            <template v-if="failure === 'countdown'">
              <UiButton icon="refresh" @click="reload">{{ t('zoom-room.alerts.failure.retryNow') }}</UiButton>
              <UiButton variant="secondary" @click="cancelRetry">{{ t('zoom-room.alerts.failure.cancel') }}</UiButton>
            </template>
            <template v-else>
              <UiButton icon="refresh" @click="reload">{{ t('zoom-room.retry') }}</UiButton>
              <UiButton v-if="props.joinUrl" variant="secondary" icon="external-link" :href="props.joinUrl">
                {{ t('zoom-room.fallback') }}
              </UiButton>
              <UiButton variant="ghost" @click="dismiss">{{ t('zoom-room.alerts.failure.close') }}</UiButton>
            </template>
          </div>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style>
/*
 * Les feuilles du SDK, chargées depuis Zoom et hors des couches de Tailwind,
 * l'emportent sur toute classe utilitaire (boutons sans fond, marges changées).
 * Dans nos avis seulement, on les annule : `revert-layer` rend la main aux couches.
 */
#zoom-room-alerts,
#zoom-room-alerts * {
  all: revert-layer;
}
</style>
