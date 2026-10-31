/**
 * Temps réel : `useRealtime(['proposal:<id>'], relire)` rappelle `relire` quand
 * un de ces sujets change. La page relit alors sa route, comme elle le fait
 * déjà : rien ne transite que le nom du sujet.
 *
 * Deux transports, choisis par sujet, invisibles pour la page :
 * - un sujet public (`edition:<id>:public…`) suit le **pouls**, une question
 *   toutes les 30 s, avec un délai au hasard de 0 à 5 s avant de relire (des
 *   centaines de pages apprennent la nouvelle ensemble) ;
 * - un sujet privé, pour une personne connectée, suit la **ligne ouverte**
 *   (SSE lu par `fetch`) ; après cinq échecs, ou si l'API la ferme, le pouls
 *   prend le relais toutes les 15 s, et la ligne est retentée en arrière-plan.
 *
 * Une seule ligne par onglet, partagée par tous les composants ; fermée quand
 * l'onglet est caché, rouverte au retour. À chaque reconnexion, ou si l'API a
 * redémarré (`boot`), relecture complète : un signal perdu est rattrapé, jamais
 * rejoué. Rien ne s'affiche quand la mise à jour est coupée (décision du 31/10).
 *
 * Sans `NUXT_PUBLIC_API_BASE` (données d'exemple), ne fait rien. État de module,
 * navigateur seulement : en rendu serveur il serait partagé entre visiteurs.
 * Étude : `docs/progression/decisions/2026-10-31-temps-reel.md`.
 */
import type { MaybeRefOrGetter } from 'vue'
import type { RealtimeApi } from './api/realtime'
import type { RealtimeReady, RealtimeSignal, RealtimeTopic } from '~/types/realtime'
import { SESSION_WITNESS_COOKIE } from './useApiStatus'
import { backoffMs, changedTopics, isPublicTopic, isValidTopic, splitSse, type SseEvent } from '~/utils/realtime'

export type RealtimeHandler = (topics: RealtimeTopic[]) => unknown

const PULSE_PUBLIC_MS = 30_000
const PULSE_FALLBACK_MS = 15_000
const FAILURES_BEFORE_FALLBACK = 5
const STABLE_MS = 60_000
const BROAD_JITTER_MS = 5_000
const SETTLE_MS = 300

interface Subscriber {
  topics: RealtimeTopic[]
  onChange: RealtimeHandler
}

type StreamEnd = 'ended' | 'reauth' | 'evicted' | 'restart'
type Timer = ReturnType<typeof setTimeout>

interface Line {
  api: RealtimeApi
  subscribers: Set<Subscriber>
  /** Refusés par l'API : plus jamais demandés dans cet onglet. */
  refused: Set<RealtimeTopic>
  settle: Timer | null
  // Ligne ouverte
  controller: AbortController | null
  streamKey: string
  retry: Timer | null
  stable: Timer | null
  failures: number
  /** Repli sur le pouls : échecs répétés, ou plus de cinq lignes pour la session. */
  degraded: boolean
  evicted: boolean
  /** Sujets déjà servis par une ligne : à la réouverture, ils se relisent. */
  streamed: Set<RealtimeTopic>
  // Pouls
  pulseTimer: Timer | null
  pulseKey: string
  pulsing: boolean
  etag: string | null
  boot: string | null
  versions: Map<RealtimeTopic, number>
  /** À relire au premier pouls : entrés dans le pouls après une coupure de la ligne. */
  owed: Set<RealtimeTopic>
}

let line: Line | null = null

const isVisible = (): boolean => document.visibilityState === 'visible'

function isConnected(): boolean {
  return document.cookie.split('; ').some((c) => c.startsWith(`${SESSION_WITNESS_COOKIE}=`) && c.length > SESSION_WITNESS_COOKIE.length + 1)
}

const sleep = (ms: number): Promise<void> => new Promise((resolve) => setTimeout(resolve, ms))
const jitter = (max: number): number => Math.floor(Math.random() * max)

function wanted(l: Line): RealtimeTopic[] {
  const all = new Set<RealtimeTopic>()
  for (const s of l.subscribers) for (const t of s.topics) if (!l.refused.has(t)) all.add(t)
  return [...all].sort()
}

/** Rappelle chaque abonné pour ses sujets parmi `topics`. Une erreur de page ne remonte pas. */
async function dispatch(l: Line, topics: RealtimeTopic[], broad: boolean): Promise<void> {
  if (!topics.length) return
  if (broad) await sleep(jitter(BROAD_JITTER_MS))
  const changed = new Set(topics)
  for (const s of [...l.subscribers]) {
    const mine = s.topics.filter((t) => changed.has(t))
    if (!mine.length) continue
    try {
      await s.onChange(mine)
    } catch {
      // La relecture suivante rattrape.
    }
  }
}

function relire(l: Line, topics: RealtimeTopic[]): void {
  void dispatch(l, topics.filter((t) => !isPublicTopic(t)), false)
  void dispatch(l, topics.filter(isPublicTopic), true)
}

// ---------------------------------------------------------------------------
// Ligne ouverte
// ---------------------------------------------------------------------------

function closeStream(l: Line): void {
  l.controller?.abort()
  l.controller = null
  l.streamKey = ''
  if (l.retry) clearTimeout(l.retry)
  l.retry = null
  if (l.stable) clearTimeout(l.stable)
  l.stable = null
}

function retryLater(l: Line, ms: number): void {
  if (l.retry) clearTimeout(l.retry)
  l.retry = setTimeout(() => {
    l.retry = null
    l.streamKey = ''
    reconcile(l)
  }, ms)
}

function onStreamFailure(l: Line, topics: RealtimeTopic[]): void {
  l.failures += 1
  if (l.failures >= FAILURES_BEFORE_FALLBACK && !l.degraded) {
    l.degraded = true
    for (const t of topics) l.owed.add(t)
    reconcilePulse(l)
  }
  retryLater(l, backoffMs(l.failures))
}

/** Lit le flux jusqu'à sa fin ; rend ce qui l'a terminé. */
async function read(l: Line, body: ReadableStream<Uint8Array>, topics: RealtimeTopic[]): Promise<StreamEnd> {
  const reader = body.getReader()
  const decoder = new TextDecoder()
  let buffer = ''
  for (;;) {
    const { value, done } = await reader.read()
    if (done) return 'ended'
    const { events, rest } = splitSse(buffer + decoder.decode(value, { stream: true }))
    buffer = rest
    for (const event of events) {
      const end = onEvent(l, event, topics)
      if (end) return end
    }
  }
}

function onEvent(l: Line, { name, data }: SseEvent, topics: RealtimeTopic[]): StreamEnd | null {
  const payload = (): unknown => {
    try {
      return data ? JSON.parse(data) : {}
    } catch {
      return {}
    }
  }
  switch (name) {
    case 'ready': {
      const ready = payload() as Partial<RealtimeReady>
      for (const t of ready.denied ?? []) l.refused.add(t)
      // Une ligne qui revient : ce qui était déjà suivi se relit.
      const owed = topics.filter((t) => l.streamed.has(t) || l.owed.has(t))
      for (const t of topics) {
        l.streamed.add(t)
        l.owed.delete(t)
      }
      if (l.degraded) {
        l.degraded = false
        reconcilePulse(l)
      }
      if (l.stable) clearTimeout(l.stable)
      l.stable = setTimeout(() => (l.failures = 0), STABLE_MS)
      relire(l, owed)
      return null
    }
    case 'signal':
      void dispatch(l, (payload() as Partial<RealtimeSignal>).topics ?? [], false)
      return null
    case 'resync':
      relire(l, topics)
      return null
    case 'revoked':
      for (const t of (payload() as Partial<RealtimeSignal>).topics ?? []) l.refused.add(t)
      return null
    case 'reauth':
    case 'evicted':
    case 'restart':
      return name
    default:
      return null
  }
}

async function openStream(l: Line, topics: RealtimeTopic[]): Promise<void> {
  const controller = new AbortController()
  l.controller = controller
  l.streamKey = topics.join(',')
  let end: StreamEnd | 'failed' | 'refused' | 'signedOut' = 'failed'
  try {
    let response = await l.api.stream(topics, controller.signal)
    if (response.status === 401) {
      const issue = await l.api.rotation()
      if (issue === 'finie') end = 'signedOut'
      else if (issue === 'renouvelee') response = await l.api.stream(topics, controller.signal)
    }
    if (end !== 'signedOut') {
      if (response.status === 403) end = 'refused'
      else if (response.ok && response.body) end = await read(l, response.body, topics)
    }
  } catch {
    if (controller.signal.aborted) return
  }
  if (controller.signal.aborted || l.controller !== controller) return
  controller.abort()
  l.controller = null
  l.streamKey = ''

  switch (end) {
    case 'refused':
      for (const t of topics) l.refused.add(t)
      reconcile(l)
      return
    case 'signedOut':
      return
    case 'evicted':
      l.evicted = true
      for (const t of topics) l.owed.add(t)
      reconcilePulse(l)
      return
    case 'restart':
      // L'API redémarre : tout le monde revient, mais pas au même instant.
      retryLater(l, jitter(BROAD_JITTER_MS))
      return
    case 'reauth': {
      const issue = await l.api.rotation()
      if (issue !== 'finie') retryLater(l, issue === 'renouvelee' ? 0 : 1000)
      return
    }
    default:
      onStreamFailure(l, topics)
  }
}

// ---------------------------------------------------------------------------
// Pouls
// ---------------------------------------------------------------------------

function pulseTopics(l: Line): RealtimeTopic[] {
  const connected = isConnected()
  return wanted(l).filter((t) => isPublicTopic(t) || (connected && (l.degraded || l.evicted)))
}

function reconcilePulse(l: Line): void {
  if (l.pulseTimer) clearTimeout(l.pulseTimer)
  l.pulseTimer = null
  const topics = pulseTopics(l)
  if (!topics.length || !isVisible()) return
  const key = topics.join(',')
  let wait = topics.some((t) => !isPublicTopic(t)) ? PULSE_FALLBACK_MS : PULSE_PUBLIC_MS
  if (key !== l.pulseKey) {
    l.pulseKey = key
    l.etag = null
    // Un sujet nouveau pose sa version tout de suite : posée 30 s plus tard,
    // un changement survenu entre-temps passerait pour l'état initial (vu à l'essai).
    if (topics.some((t) => !l.versions.has(t))) wait = 0
  }
  l.pulseTimer = setTimeout(() => void pulse(l), wait)
}

async function pulse(l: Line): Promise<void> {
  const topics = pulseTopics(l)
  if (l.pulsing || !topics.length || !isVisible()) return reconcilePulse(l)
  l.pulsing = true
  try {
    const reading = await l.api.pulse(topics, l.etag)
    if (reading.kind === 'unauthenticated') {
      await l.api.rotation()
    } else if (reading.kind === 'fresh') {
      const { boot, versions, denied } = reading.pulse
      for (const t of denied) l.refused.add(t)
      const changed = changedTopics(topics, l, reading.pulse, l.owed)
      for (const t of topics) {
        const version = versions[t]
        if (version === undefined) continue
        l.versions.set(t, version)
        l.owed.delete(t)
      }
      l.boot = boot
      l.etag = reading.etag
      relire(l, changed)
    }
  } finally {
    l.pulsing = false
    reconcilePulse(l)
  }
}

// ---------------------------------------------------------------------------
// Accord de l'ensemble
// ---------------------------------------------------------------------------

function reconcile(l: Line): void {
  const topics = wanted(l)
  for (const t of l.streamed) if (!topics.includes(t)) l.streamed.delete(t)
  for (const t of l.versions.keys()) if (!topics.includes(t)) l.versions.delete(t)

  const privateTopics = isConnected() ? topics.filter((t) => !isPublicTopic(t)) : []
  const key = privateTopics.join(',')
  if (!privateTopics.length || !isVisible() || l.evicted) closeStream(l)
  else if (key !== l.streamKey && !l.retry) {
    closeStream(l)
    void openStream(l, privateTopics)
  }
  reconcilePulse(l)
}

function schedule(l: Line): void {
  if (l.settle) clearTimeout(l.settle)
  l.settle = setTimeout(() => {
    l.settle = null
    reconcile(l)
  }, SETTLE_MS)
}

function createLine(api: RealtimeApi): Line {
  const l: Line = {
    api,
    subscribers: new Set(),
    refused: new Set(),
    settle: null,
    controller: null,
    streamKey: '',
    retry: null,
    stable: null,
    failures: 0,
    degraded: false,
    evicted: false,
    streamed: new Set(),
    pulseTimer: null,
    pulseKey: '',
    pulsing: false,
    etag: null,
    boot: null,
    versions: new Map(),
    owed: new Set(),
  }
  document.addEventListener('visibilitychange', () => {
    if (!isVisible()) {
      closeStream(l)
      reconcilePulse(l)
      return
    }
    // Retour sur l'onglet : la ligne rouvre (et relit), le pouls repose sa question.
    l.evicted = false
    reconcile(l)
    void pulse(l)
  })
  window.addEventListener('online', () => {
    l.failures = 0
    closeStream(l)
    reconcile(l)
  })
  return l
}

export function useRealtime(topics: MaybeRefOrGetter<RealtimeTopic[]>, onChange: RealtimeHandler): void {
  if (import.meta.server) return
  const api = useApi().realtime
  if (!api.isConfigured()) return
  line ??= createLine(api)
  const l = line
  const subscriber: Subscriber = { topics: [], onChange }

  watch(
    () => toValue(topics).filter(isValidTopic),
    (list) => {
      subscriber.topics = [...new Set(list)]
      schedule(l)
    },
    { immediate: true },
  )
  onMounted(() => {
    l.subscribers.add(subscriber)
    schedule(l)
  })
  onBeforeUnmount(() => {
    l.subscribers.delete(subscriber)
    schedule(l)
  })
}
