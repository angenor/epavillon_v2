import type { Ref } from 'vue'

interface ReadOnSightOptions {
  /** Conteneur où chercher les éléments `[data-comment-id]`. */
  root: Ref<HTMLElement | null>
  /** Identifiants encore non lus, donc à surveiller. */
  targets: () => string[]
  /** Envoi groupé ; un rejet remet les messages en attente d'une prochaine vue. */
  onRead: (ids: string[]) => Promise<void>
  /** Temps d'affichage avant de compter un message comme lu. */
  dwellMs?: number
  /** Délai de regroupement des envois. */
  batchMs?: number
}

/**
 * MARQUE LU CE QUI A ÉTÉ AFFICHÉ, sans clic.
 *
 * Un message compte comme vu quand la moitié de sa hauteur (ou la moitié de
 * l'écran, pour un long message) est restée visible `dwellMs`, onglet au
 * premier plan : un défilement rapide ou un onglet en arrière-plan ne lit rien.
 * Les envois sont regroupés, et un message remis « non lu » à la main n'est plus
 * repris pendant la visite, sinon il redeviendrait lu sous les yeux de qui
 * vient de le marquer.
 */
export function useReadOnSight(options: ReadOnSightOptions) {
  const dwellMs = options.dwellMs ?? 800
  const batchMs = options.batchMs ?? 1500

  const sent = new Set<string>()
  const suppressed = new Set<string>()
  const dwellTimers = new Map<string, ReturnType<typeof setTimeout>>()
  const observed = new Map<string, Element>()
  let queue = new Set<string>()
  let batchTimer: ReturnType<typeof setTimeout> | null = null
  let observer: IntersectionObserver | null = null

  const isForeground = (): boolean => document.visibilityState === 'visible'

  function clearDwell(id: string): void {
    const timer = dwellTimers.get(id)
    if (timer) clearTimeout(timer)
    dwellTimers.delete(id)
  }

  function flush(): void {
    if (batchTimer) clearTimeout(batchTimer)
    batchTimer = null
    if (queue.size === 0 || !isForeground()) return
    const ids = [...queue]
    queue = new Set()
    for (const id of ids) sent.add(id)
    options.onRead(ids).catch(() => {
      for (const id of ids) sent.delete(id)
    })
  }

  function seen(id: string): void {
    if (sent.has(id) || suppressed.has(id)) return
    queue.add(id)
    batchTimer ??= setTimeout(flush, batchMs)
  }

  function onEntries(entries: IntersectionObserverEntry[]): void {
    for (const entry of entries) {
      const id = (entry.target as HTMLElement).dataset.commentId
      if (!id) continue
      const viewport = entry.rootBounds?.height ?? window.innerHeight
      const visible =
        entry.isIntersecting &&
        (entry.intersectionRatio >= 0.5 || entry.intersectionRect.height >= viewport * 0.5)
      if (visible && isForeground()) {
        if (!dwellTimers.has(id)) {
          dwellTimers.set(id, setTimeout(() => {
            dwellTimers.delete(id)
            seen(id)
          }, dwellMs))
        }
      } else {
        clearDwell(id)
      }
    }
  }

  function unobserve(id: string): void {
    const element = observed.get(id)
    if (element) observer?.unobserve(element)
    observed.delete(id)
    clearDwell(id)
  }

  function sync(): void {
    const root = options.root.value
    if (!observer || !root) return
    const wanted = new Set(options.targets().filter((id) => !sent.has(id) && !suppressed.has(id)))
    for (const [id, element] of observed) {
      if (!wanted.has(id) || !element.isConnected) unobserve(id)
    }
    for (const id of wanted) {
      if (observed.has(id)) continue
      const element = root.querySelector(`[data-comment-id="${CSS.escape(id)}"]`)
      if (!element) continue
      observer.observe(element)
      observed.set(id, element)
    }
  }

  /** Au retour au premier plan, l'observateur ne redit rien de ce qui n'a pas bougé : on le relance. */
  function onVisibilityChange(): void {
    if (!isForeground()) {
      for (const id of [...dwellTimers.keys()]) clearDwell(id)
      return
    }
    flush()
    for (const element of observed.values()) {
      observer?.unobserve(element)
      observer?.observe(element)
    }
  }

  onMounted(() => {
    if (typeof IntersectionObserver === 'undefined') return
    observer = new IntersectionObserver(onEntries, { threshold: [0, 0.25, 0.5, 0.75, 1] })
    document.addEventListener('visibilitychange', onVisibilityChange)
    sync()
  })

  watch(() => options.targets().join(','), () => nextTick(sync))

  onBeforeUnmount(() => {
    flush()
    observer?.disconnect()
    observer = null
    for (const id of [...dwellTimers.keys()]) clearDwell(id)
    document.removeEventListener('visibilitychange', onVisibilityChange)
  })

  return {
    /** Le message vient d'être remis « non lu » : ne plus le reprendre pendant la visite. */
    suppress(id: string): void {
      suppressed.add(id)
      sent.delete(id)
      queue.delete(id)
      unobserve(id)
    },
  }
}
