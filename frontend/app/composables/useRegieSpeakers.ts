import type { RegieSpeakers } from '~/types/programme/online-speaker'
import type { TimeZoneName, Uuid } from '~/types/shared'

/** La liste des intervenants en ligne d'une activité, relue à chaque ouverture d'une fenêtre de la régie. */
export function useRegieSpeakers(sessionId: () => Uuid | null) {
  const api = useApi()
  const { t } = useI18n()
  const dt = useDateTime()

  const detail = ref<RegieSpeakers | null>(null)
  const state = ref<'idle' | 'loading' | 'ready' | 'error'>('idle')
  const error = ref<string | null>(null)

  async function load(): Promise<void> {
    const id = sessionId()
    if (!id) return
    state.value = 'loading'
    error.value = null
    try {
      detail.value = await api.adminRegie.speakers(id)
      state.value = 'ready'
    } catch (caught) {
      error.value = apiErrorMessage(caught, t)
      state.value = 'error'
    }
  }

  /** `{ date, time }` pour « le 12 novembre à 11:30 », au fuseau de l'édition. */
  const moment = (value: string, timezone: TimeZoneName) => ({
    date: dayMonth(value, timezone, dt.intlLocale.value),
    time: dt.time(value, timezone),
  })

  return { detail, state, error, load, moment }
}
