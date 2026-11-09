import type { MaybeRefOrGetter } from 'vue'
import type { CancelRegistrationResult, Registration } from '~/types/programme/registration'
import { estInchange } from '~/composables/api/etiquete'

/**
 * L'inscription vivante de la personne connectée à une séance.
 *
 * Lue côté client seulement : elle est personnelle, et n'a rien à faire dans le
 * payload d'une page publique. Une lecture qui échoue laisse `mine` vide — la
 * page reste utilisable, le dialogue d'inscription dira « déjà inscrit·e ».
 *
 * Sans compte, l'API ne rend rien de lisible : ce navigateur retient seulement
 * le fait (`guestRegistered`), pour ne pas reproposer de s'inscrire.
 */
const GUEST_KEY = 'epavillon.activity-guest-registrations'

function readGuestIds(): string[] {
  try {
    const stored: unknown = JSON.parse(localStorage.getItem(GUEST_KEY) ?? '[]')
    return Array.isArray(stored) ? stored.filter((value): value is string => typeof value === 'string') : []
  } catch {
    return []
  }
}

export function useSessionRegistration(sessionId: MaybeRefOrGetter<string>) {
  const api = useApi()
  const auth = useAuthStore()
  const mine = ref<Registration | null>(null)
  const loading = ref(false)
  const guestKnown = ref(false)
  const guestRegistered = computed(() => guestKnown.value && !auth.isAuthenticated)

  function readGuest(): void {
    guestKnown.value = readGuestIds().includes(toValue(sessionId))
  }

  function rememberGuest(): void {
    guestKnown.value = true
    try {
      localStorage.setItem(GUEST_KEY, JSON.stringify([...new Set([...readGuestIds(), toValue(sessionId)])]))
    } catch {
      // Stockage indisponible : le fait vaut pour cette page.
    }
  }

  async function refresh(): Promise<void> {
    const id = toValue(sessionId)
    await auth.ensureLoaded()
    if (!auth.isAuthenticated || !id) {
      mine.value = null
      return
    }
    loading.value = true
    try {
      const lu = await api.pavillon.mesInscriptions(null)
      if (estInchange(lu)) return
      mine.value =
        lu.valeur
          .filter((r) => r.session_id === id && r.status !== 'cancelled')
          .sort((a, b) => b.created_at.localeCompare(a.created_at))[0] ?? null
    } catch {
      mine.value = null
    } finally {
      loading.value = false
    }
  }

  /** Lève le refus de l'API : l'appelant l'affiche tel quel. */
  async function cancel(): Promise<CancelRegistrationResult | null> {
    const current = mine.value
    if (!current) return null
    try {
      const result = await api.pavillon.annuler(current.id)
      mine.value = null
      return result
    } catch (error) {
      await refresh()
      throw error
    }
  }

  onMounted(() => {
    readGuest()
    void refresh()
  })
  watch([() => toValue(sessionId), () => auth.isAuthenticated], () => {
    if (!import.meta.client) return
    readGuest()
    void refresh()
  })

  return { mine, loading, refresh, cancel, guestRegistered, rememberGuest }
}
