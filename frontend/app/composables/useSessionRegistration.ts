import type { MaybeRefOrGetter } from 'vue'
import type { CancelRegistrationResult, Registration } from '~/types/programme/registration'
import { estInchange } from '~/composables/api/etiquete'

/**
 * L'inscription vivante de la personne connectée à une séance.
 *
 * Lue côté client seulement : elle est personnelle, et n'a rien à faire dans le
 * payload d'une page publique. Une lecture qui échoue laisse `mine` vide — la
 * page reste utilisable, le dialogue d'inscription dira « déjà inscrit·e ».
 */
export function useSessionRegistration(sessionId: MaybeRefOrGetter<string>) {
  const api = useApi()
  const auth = useAuthStore()
  const mine = ref<Registration | null>(null)
  const loading = ref(false)

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
    void refresh()
  })
  watch([() => toValue(sessionId), () => auth.isAuthenticated], () => {
    if (import.meta.client) void refresh()
  })

  return { mine, loading, refresh, cancel }
}
