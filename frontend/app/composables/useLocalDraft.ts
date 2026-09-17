import type { IsoDateTime } from '~/types/shared'

/**
 * Une saisie gardée dans le navigateur, pour la retrouver un autre jour.
 *
 * Rien ne part au serveur : c'est un filet, pas un enregistrement. La saisie
 * retrouvée n'est jamais appliquée d'office — la fiche a pu changer depuis —,
 * l'écran propose de la reprendre ou de l'écarter.
 */

const PREFIX = 'epavillon:draft:'
const VERSION = 1

interface StoredDraft<T> {
  v: number
  savedAt: IsoDateTime
  data: T
}

export function clearLocalDraft(key: string | null | undefined): void {
  if (!key) return
  try {
    localStorage.removeItem(PREFIX + key)
  } catch {
    // Stockage refusé : il n'y a rien à effacer.
  }
}

function readDraft<T>(key: string): StoredDraft<T> | null {
  try {
    const raw = localStorage.getItem(PREFIX + key)
    if (!raw) return null
    const parsed = JSON.parse(raw) as StoredDraft<T>
    return parsed.v === VERSION ? parsed : null
  } catch {
    return null
  }
}

export interface UseLocalDraftOptions<T> {
  /** `null` : pas de brouillon (personne inconnue, par exemple). */
  key: MaybeRefOrGetter<string | null>
  /** Ce qu'on garde ; lu à chaque modification. */
  snapshot: () => T
  /** Remet une saisie retrouvée dans l'écran. */
  apply: (data: T) => void
  debounceMs?: number
}

export function useLocalDraft<T>(options: UseLocalDraftOptions<T>) {
  const debounceMs = options.debounceMs ?? 600
  /** La saisie retrouvée, en attente d'une décision. */
  const found = ref<StoredDraft<T> | null>(null) as Ref<StoredDraft<T> | null>
  const active = ref(false)
  let baseline = ''
  let timer: ReturnType<typeof setTimeout> | undefined

  function write(): void {
    const key = toValue(options.key)
    if (!key || !active.value || found.value) return
    const data = options.snapshot()
    const serialized = JSON.stringify(data)
    // Revenue à son point de départ, la saisie n'a plus rien à garder.
    if (serialized === baseline) {
      clearLocalDraft(key)
      return
    }
    try {
      const stored: StoredDraft<T> = { v: VERSION, savedAt: new Date().toISOString(), data }
      localStorage.setItem(PREFIX + key, JSON.stringify(stored))
    } catch {
      // Stockage plein ou refusé : l'écran fonctionne sans filet.
    }
  }

  onMounted(() => {
    baseline = JSON.stringify(options.snapshot())
    const key = toValue(options.key)
    const stored = key ? readDraft<T>(key) : null
    if (stored && JSON.stringify(stored.data) !== baseline) found.value = stored
    else if (key) clearLocalDraft(key)
    active.value = true
  })

  watch(
    options.snapshot,
    () => {
      if (!active.value) return
      if (timer) clearTimeout(timer)
      timer = setTimeout(write, debounceMs)
    },
    { deep: true },
  )

  onBeforeUnmount(() => {
    if (timer) {
      clearTimeout(timer)
      write()
    }
  })

  function restore(): void {
    if (!found.value) return
    options.apply(found.value.data)
    found.value = null
  }

  function discard(): void {
    found.value = null
    clearLocalDraft(toValue(options.key))
    write()
  }

  /** Écrit tout de suite : après un envoi réussi, une frappe en attente ne doit pas recréer le brouillon. */
  function flush(): void {
    if (timer) clearTimeout(timer)
    timer = undefined
    write()
  }

  function clear(): void {
    if (timer) clearTimeout(timer)
    timer = undefined
    clearLocalDraft(toValue(options.key))
  }

  return {
    found: computed(() => found.value),
    restore,
    discard,
    flush,
    clear,
  }
}
