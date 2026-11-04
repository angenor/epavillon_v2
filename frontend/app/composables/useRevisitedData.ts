import type { WatchSource } from 'vue'

interface RevisitOptions<T> {
  default?: () => T
  watch?: WatchSource[]
  /** Refuser la dernière lecture : celle d'une autre édition que celle demandée par l'adresse. */
  reusable?: (kept: T) => boolean
}

/**
 * Une page publique déjà vue revient aussitôt, sans squelette, puis se relit en
 * silence (04/11, demande du commanditaire). Nuxt ne réutilise ses données qu'à
 * l'hydratation et les purge en quittant la page ; une règle `getCachedData` à
 * soi les garde dans `payload.data`, la mémoire de l'onglet. La première visite
 * charge comme avant. Une relecture manquée garde ce qui est affiché.
 */
export function useRevisitedData<T>(key: MaybeRefOrGetter<string>, handler: () => Promise<T>, options: RevisitOptions<T> = {}) {
  const { reusable, ...rest } = options
  const nuxtApp = useNuxtApp()

  const result = useAsyncData(key, handler, {
    lazy: true,
    ...rest,
    getCachedData(current, app, context) {
      const kept = app.payload.data[current] as T | undefined
      if (app.isHydrating) return kept
      if (import.meta.server || context.cause !== 'initial' || kept === undefined) return undefined
      if (reusable && !reusable(kept)) return undefined
      void nextTick(() => refreshQuietly(current))
      return kept
    },
  })

  async function refreshQuietly(current: string): Promise<void> {
    try {
      const fresh = await handler()
      if (toValue(key) !== current) return
      result.data.value = fresh
      nuxtApp.payload.data[current] = fresh
    } catch {
      // Hors ligne ou API muette : la dernière lecture reste.
    }
  }

  return result
}
