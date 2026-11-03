/**
 * Présence : cet onglet se signale à l'API (`POST /presence`) toutes les 30 s
 * tant qu'il est visible, aussitôt revenu au premier plan, et peu après un
 * changement de zone. C'est ce que compte la carte « En ce moment » du tableau
 * de bord.
 *
 * Monté une fois, par `app.vue`. Navigateur seulement ; sans
 * `NUXT_PUBLIC_API_BASE` (données d'exemple), ne fait rien. Une erreur ne
 * s'affiche jamais : le signal suivant rattrape.
 *
 * L'API lit « connecté » de la session, jamais du corps. Passé un quart d'heure
 * sans appel, le cookie d'accès a expiré : un onglet qui se sait connecté et
 * que l'API compte comme visiteur tourne son jeton, une fois, puis se signale
 * de nouveau.
 */
import { SESSION_WITNESS_COOKIE } from './useApiStatus'
import { presenceTabId, presenceZoneOf } from '~/utils/presence'

const BEAT_MS = 30_000
const SETTLE_MS = 1_500
const ROTATION_PAUSE_MS = 5 * 60_000

type Timer = ReturnType<typeof setTimeout>

let started = false

function witnessed(): boolean {
  return document.cookie
    .split('; ')
    .some((c) => c.startsWith(`${SESSION_WITNESS_COOKIE}=`) && c.length > SESSION_WITNESS_COOKIE.length + 1)
}

function randomTabId(): string {
  if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') return crypto.randomUUID()
  return `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 12)}`
}

function sessionStorageOrNull(): Storage | null {
  try {
    return window.sessionStorage
  } catch {
    return null
  }
}

export function usePresence(): void {
  if (import.meta.server) return
  const api = useApi().presence
  const route = useRoute()
  const zone = computed(() => presenceZoneOf(route.name))

  let beatTimer: Timer | null = null
  let settleTimer: Timer | null = null
  let rotationPausedUntil = 0
  let tab = ''

  const isVisible = (): boolean => document.visibilityState === 'visible'

  async function beat(): Promise<void> {
    if (!isVisible()) return
    const body = { tab, zone: zone.value }
    const ack = await api.beat(body)
    if (!ack || ack.signed_in || !witnessed() || Date.now() < rotationPausedUntil) return
    rotationPausedUntil = Date.now() + ROTATION_PAUSE_MS
    if ((await api.rotation()) === 'renouvelee') await api.beat({ tab, zone: zone.value })
  }

  function stop(): void {
    if (beatTimer) clearInterval(beatTimer)
    if (settleTimer) clearTimeout(settleTimer)
    beatTimer = null
    settleTimer = null
  }

  /** Signale maintenant, puis toutes les 30 s à partir de là. */
  function restart(): void {
    stop()
    if (!isVisible()) return
    void beat()
    beatTimer = setInterval(() => void beat(), BEAT_MS)
  }

  function onVisibility(): void {
    if (isVisible()) restart()
    else stop()
  }

  onMounted(() => {
    if (started || !api.isConfigured()) return
    started = true
    tab = presenceTabId(sessionStorageOrNull(), randomTabId)
    document.addEventListener('visibilitychange', onVisibility)
    watch(zone, () => {
      if (settleTimer) clearTimeout(settleTimer)
      settleTimer = setTimeout(restart, SETTLE_MS)
    })
    restart()
  })

  onBeforeUnmount(() => {
    if (!tab) return
    stop()
    document.removeEventListener('visibilitychange', onVisibility)
    started = false
  })
}
