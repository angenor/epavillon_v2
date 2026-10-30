/**
 * Relit une donnée à intervalle, côté navigateur seulement : de quoi tenir un
 * fil de messages à jour sans flux temps réel (aucun n'existe dans l'API).
 *
 * Onglet caché : rien ne part, et la relecture reprend aussitôt au retour. Un
 * échec est silencieux : la prochaine relecture le rattrape.
 */
export function usePolling(read: () => Promise<unknown>, intervalMs: () => number): void {
  let timer: ReturnType<typeof setTimeout> | null = null
  let reading = false

  function schedule(): void {
    if (timer !== null) clearTimeout(timer)
    timer = setTimeout(tick, intervalMs())
  }

  async function tick(): Promise<void> {
    if (document.visibilityState === 'visible' && !reading) {
      reading = true
      try {
        await read()
      } catch {
        // La relecture suivante rattrape.
      } finally {
        reading = false
      }
    }
    schedule()
  }

  function onVisibility(): void {
    if (document.visibilityState === 'visible') void tick()
  }

  // Un intervalle qui change (fenêtre ouverte ou fermée) repart de zéro.
  watch(intervalMs, schedule)

  onMounted(() => {
    document.addEventListener('visibilitychange', onVisibility)
    schedule()
  })

  onBeforeUnmount(() => {
    document.removeEventListener('visibilitychange', onVisibility)
    if (timer !== null) clearTimeout(timer)
  })
}
