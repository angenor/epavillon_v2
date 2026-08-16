import { defineStore } from 'pinia'
import type { I18nText, Uuid } from '~/types/shared'

/** Événement visible dans le sélecteur du back-office. */
export interface AdminScopeEvent {
  id: Uuid
  /** `event.events.title`, colonne `platform.i18n_text` — à résoudre, jamais `.fr`. */
  title: I18nText
  /** `event.events.timezone` — fuseau de référence de l'édition. */
  timezone: string
}

/**
 * Périmètre d'administration.
 *
 * Règle métier n° 8 : un administrateur peut n'avoir accès qu'à un seul
 * événement (`identity.administered_events()`, rôle `admin` attribuable sur la
 * portée `event`). Toute liste du back-office est filtrée par ce périmètre,
 * y compris quand l'URL est forgée à la main — le filtrage définitif reste donc
 * la responsabilité de l'API ; ce store ne porte que l'état de l'interface.
 *
 * La liste est vide tant que rien ne l'alimente : les données simulées arrivent
 * au prompt A0.3, l'API au prompt B7. Le sélecteur affiche alors son état vide.
 */
export const useAdminScopeStore = defineStore('admin-scope', () => {
  const events = ref<AdminScopeEvent[]>([])
  const currentEventId = ref<Uuid | null>(null)
  /** Vrai quand le compte n'administre qu'un seul événement : pas de choix à offrir. */
  const isRestricted = computed(() => events.value.length === 1)
  const isEmpty = computed(() => events.value.length === 0)

  const currentEvent = computed<AdminScopeEvent | null>(
    () => events.value.find((event) => event.id === currentEventId.value) ?? null,
  )

  function setEvents(next: AdminScopeEvent[]): void {
    events.value = next
    // Un périmètre restreint à un seul événement se sélectionne tout seul ;
    // une sélection devenue hors périmètre est abandonnée.
    if (next.length === 1) {
      currentEventId.value = next[0]?.id ?? null
    } else if (!next.some((event) => event.id === currentEventId.value)) {
      currentEventId.value = null
    }
  }

  function selectEvent(id: Uuid | null): void {
    if (id !== null && !events.value.some((event) => event.id === id)) return
    currentEventId.value = id
  }

  return { events, currentEventId, currentEvent, isRestricted, isEmpty, setEvents, selectEvent }
})
