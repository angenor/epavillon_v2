import type { ProgrammeChange } from '~/types/programme/changes'
import type { EventId } from '~/types/shared'

/**
 * Ce qui a changé dans le programme d'une édition, et ce qui est neuf depuis la
 * dernière visite de ce navigateur.
 *
 * La visite compte dès la page ouverte, sans « marquer comme vu » : on lit
 * l'instant précédent (il sert au compte et au marquage pendant toute cette
 * visite), puis on écrit maintenant. Retenue par édition, dans ce navigateur
 * seulement : un autre appareil reverra simplement les changements une fois.
 */

const PREFIX = 'epavillon:programme-visit:'

type Status = 'idle' | 'loading' | 'ready' | 'error'

export function useProgrammeChanges(eventId: MaybeRefOrGetter<EventId | null>) {
  const api = useApi()
  const { t } = useI18n()

  const changes = ref<ProgrammeChange[]>([])
  const status = ref<Status>('idle')
  const error = ref<string | null>(null)
  /** La visite précédente ; nulle à la première, qui ne marque rien. */
  const since = ref<number | null>(null)
  const open = ref(false)

  function visit(id: EventId): void {
    since.value = null
    try {
      const previous = Date.parse(localStorage.getItem(PREFIX + id) ?? '')
      since.value = Number.isFinite(previous) ? previous : null
      localStorage.setItem(PREFIX + id, new Date().toISOString())
    } catch {
      // Stockage refusé : chaque visite est une première visite.
    }
  }

  async function load(id: EventId, quiet = false): Promise<void> {
    if (!quiet) status.value = 'loading'
    try {
      const list = await api.programmeChanges.forEdition(id)
      if (toValue(eventId) !== id) return
      changes.value = list
      status.value = 'ready'
    } catch (failure) {
      if (quiet) return
      error.value = apiErrorMessage(failure, t)
      status.value = 'error'
    }
  }

  function start(id: EventId | null): void {
    changes.value = []
    status.value = 'idle'
    if (!id) return
    visit(id)
    void load(id)
  }

  onMounted(() => start(toValue(eventId)))
  watch(() => toValue(eventId), (id, previous) => {
    if (import.meta.client && id !== previous) start(id)
  })

  useRealtime(
    () => publicEditionTopics([toValue(eventId)], PROGRAMME_CHANNELS),
    () => {
      const id = toValue(eventId)
      if (id) void load(id, true)
    },
  )

  const isNew = (change: ProgrammeChange): boolean =>
    since.value !== null && Date.parse(change.occurred_at) > since.value

  const newCount = computed(() => changes.value.filter(isNew).length)

  return {
    open,
    changes,
    status,
    error,
    hasSince: computed(() => since.value !== null),
    isNew,
    newCount,
    retry: () => {
      const id = toValue(eventId)
      if (id) void load(id)
    },
  }
}
