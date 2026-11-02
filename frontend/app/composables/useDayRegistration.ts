import type { MaybeRefOrGetter } from 'vue'
import type { DayRegistration, DayRegistrationResult } from '~/types/programme/day-registration'

interface DayRegistrationStore {
  mine: Record<string, DayRegistration | null>
  /** Inscrite sans compte dans cet onglet : l'API ne rend rien de lisible ensuite, la page le dit quand même. */
  guest: Record<string, boolean>
  loaded: Record<string, boolean>
}

interface DayDialogState {
  trackId: string | null
  /** L'activité dont on vient : mise en avant dans la liste. */
  highlight: string | null
}

const inFlight = new Map<string, Promise<void>>()

/**
 * L'inscription de la personne connectée à une journée (journée spéciale,
 * atelier), partagée par tout ce qui la montre sur la page : billet, salle,
 * déroulé. Lue dans le navigateur seulement : elle est personnelle.
 *
 * La fenêtre d'inscription est unique par page ; n'importe quel bouton l'ouvre
 * (`openDialog`), celui qui la monte la lit (`dialogOpen`).
 */
export function useDayRegistration(trackId: MaybeRefOrGetter<string | null>) {
  const api = useApi()
  const auth = useAuthStore()
  const store = useState<DayRegistrationStore>('day-registrations', () => ({ mine: {}, guest: {}, loaded: {} }))
  const dialog = useState<DayDialogState>('day-registration-dialog', () => ({ trackId: null, highlight: null }))

  const id = computed(() => toValue(trackId))
  const mine = computed(() => (id.value ? (store.value.mine[id.value] ?? null) : null))
  const guestRegistered = computed(() => Boolean(id.value && store.value.guest[id.value]))
  const registered = computed(() => mine.value !== null || guestRegistered.value)
  const loaded = computed(() => Boolean(id.value && store.value.loaded[id.value]))

  function set(trackKey: string, registration: DayRegistration | null): void {
    store.value = {
      ...store.value,
      mine: { ...store.value.mine, [trackKey]: registration },
      loaded: { ...store.value.loaded, [trackKey]: true },
    }
  }

  async function refresh(force = false): Promise<void> {
    const key = id.value
    if (!key) return
    if (!force && store.value.loaded[key]) return
    const running = inFlight.get(key)
    if (running) return running
    const task = (async () => {
      await auth.ensureLoaded()
      if (!auth.isAuthenticated) return set(key, null)
      try {
        const registration = await api.trackRegistrations.mine(key)
        set(key, registration?.status === 'cancelled' ? null : registration)
      } catch {
        // Lecture manquée : la page reste utilisable, la fenêtre dira « déjà inscrite ».
        store.value = { ...store.value, loaded: { ...store.value.loaded, [key]: true } }
      }
    })().finally(() => inFlight.delete(key))
    inFlight.set(key, task)
    return task
  }

  function openDialog(highlight: string | null = null): void {
    if (id.value) dialog.value = { trackId: id.value, highlight }
  }

  const dialogOpen = computed({
    get: () => id.value !== null && dialog.value.trackId === id.value,
    set: (open: boolean) => {
      if (!open) dialog.value = { trackId: null, highlight: null }
      else openDialog()
    },
  })
  const highlight = computed(() => (dialogOpen.value ? dialog.value.highlight : null))

  /** Issue d'une inscription : l'inscription rendue, ou l'invitée à qui l'API ne rend rien. */
  function registeredWith(result: DayRegistrationResult, asGuest: boolean): void {
    const key = id.value
    if (!key || result.status === 'closed') return
    // Sans compte, l'inscription ne se gère que par le lien du courriel : la page n'en garde que le fait.
    if (asGuest) store.value = { ...store.value, guest: { ...store.value.guest, [key]: true } }
    else if (result.registration) set(key, result.registration)
  }

  function updated(registration: DayRegistration): void {
    if (id.value) set(id.value, registration)
  }

  /** Lève le refus de l'API : l'appelant l'affiche tel quel. */
  async function cancel(): Promise<void> {
    const current = mine.value
    const key = id.value
    if (!current || !key) return
    try {
      await api.trackRegistrations.cancel(current.id)
      set(key, null)
    } catch (error) {
      await refresh(true)
      throw error
    }
  }

  onMounted(() => void refresh())
  watch([id, () => auth.isAuthenticated], () => {
    if (import.meta.client) void refresh(true)
  })

  return { mine, guestRegistered, registered, loaded, refresh, openDialog, dialogOpen, highlight, registeredWith, updated, cancel }
}
