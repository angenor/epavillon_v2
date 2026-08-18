import { defineStore } from 'pinia'
import type { AdministeredEvents } from '~/types/identity'
import type { I18nText, Uuid } from '~/types/shared'

/** Événement visible dans le sélecteur du back-office. */
export interface AdminScopeEvent {
  id: Uuid
  /** `event.events.title`, colonne `platform.i18n_text` — à résoudre, jamais `.fr`. */
  title: I18nText
  /** `event.events.timezone` — fuseau de référence de l'édition. */
  timezone: string
  /** Sigle court (« COP31 ») : ce qui tient dans un sélecteur, quand il existe. */
  acronym: string | null
  /**
   * Ville de l'édition, et non un ornement : c'est ELLE qui nomme le fuseau à
   * l'écran. Déduit du seul identifiant IANA, « America/Belem » donne « heure de
   * Belem », sans accent — la convention du projet est « heure de Belém », et
   * l'accent vient de `event.events.city`. Même usage qu'au bandeau d'A3.
   */
  city: string | null
  /** Rangement du sélecteur : la plus récente en tête. */
  starts_at: string
}

/**
 * Périmètre d'administration.
 *
 * RÈGLE MÉTIER N° 8 : un administrateur peut n'avoir accès qu'à un seul
 * événement (`identity.administered_events()`, rôle `admin` attribuable sur la
 * portée `event`). Toute liste du back-office est filtrée par ce périmètre, y
 * compris quand l'URL est forgée à la main — le filtrage définitif reste donc la
 * responsabilité de l'API ; ce store ne porte que l'état de l'interface.
 *
 * TROIS SITUATIONS, ET ELLES NE SE RENDENT PAS PAREIL :
 *
 *   `is_global`        toutes les éditions, un sélecteur ordinaire ;
 *   une seule édition  AUCUN SÉLECTEUR, et rien qui laisse deviner les autres —
 *                      ni liste à une entrée, ni mention « votre périmètre »,
 *                      ni compteur « 1 sur 4 ». Le back-office se lit comme s'il
 *                      n'existait qu'une COP, parce que pour cette personne
 *                      c'est le cas ;
 *   aucun droit        l'accès est refusé, ce qui n'est pas la même chose qu'un
 *                      tableau de bord vide.
 *
 * POURQUOI LE CHARGEMENT VIT ICI. La question « quelles éditions cette personne
 * administre-t-elle ? » se pose à chaque écran du back-office, au layout qui
 * affiche le sélecteur, et à chaque appel de données qui la passe en argument.
 * Trois lectures indépendantes, ce sont trois réponses possiblement différentes
 * dans une même navigation.
 */
export const useAdminScopeStore = defineStore('admin-scope', () => {
  const api = useApi()
  const auth = useAuthStore()

  const events = ref<AdminScopeEvent[]>([])
  const scope = ref<AdministeredEvents>({ is_global: false, event_ids: [] })
  const currentEventId = ref<Uuid | null>(null)
  const isLoading = ref(false)
  const loadError = ref<Error | null>(null)
  const loadedFor = ref<string | null>(null)

  /** Vrai quand le compte n'administre qu'un seul événement : pas de choix à offrir. */
  const isRestricted = computed(() => events.value.length === 1)
  const isEmpty = computed(() => events.value.length === 0)
  /** A-t-elle le moindre droit d'administration ? La seule question qui ouvre la porte. */
  const canAdminister = computed(() => events.value.length > 0)

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
      // À défaut de choix explicite, l'édition la plus récente : c'est celle sur
      // laquelle l'équipe travaille, et ouvrir sur la COP29 close depuis deux ans
      // obligerait à choisir avant de pouvoir lire quoi que ce soit.
      currentEventId.value = next[0]?.id ?? null
    }
  }

  function selectEvent(id: Uuid | null): void {
    if (id !== null && !events.value.some((event) => event.id === id)) return
    currentEventId.value = id
  }

  /**
   * Charge le périmètre une fois par personne. Idempotent : le layout et l'écran
   * l'appellent sans se coordonner.
   */
  async function ensureLoaded(): Promise<void> {
    await auth.ensureLoaded()
    const person = auth.person
    if (!person) {
      setEvents([])
      scope.value = { is_global: false, event_ids: [] }
      loadedFor.value = null
      return
    }
    if (loadedFor.value === person.id || isLoading.value) return

    isLoading.value = true
    loadError.value = null
    try {
      const administered = await api.identity.administeredEvents(person.id)
      scope.value = administered
      // La liste est demandée AVEC le périmètre : ce n'est pas au store de
      // filtrer ce que l'API doit déjà avoir filtré.
      const visible = await api.events.list(administered)
      setEvents(
        visible
          .map((event) => ({
            id: event.id,
            title: event.title,
            timezone: event.timezone,
            acronym: event.acronym,
            city: event.city,
            starts_at: event.starts_at,
          }))
          .sort((a, b) => b.starts_at.localeCompare(a.starts_at)),
      )
      loadedFor.value = person.id
    } catch (error) {
      loadError.value = error instanceof Error ? error : new Error(String(error))
    } finally {
      isLoading.value = false
    }
  }

  /**
   * Recharge le périmètre, même s'il l'a déjà été.
   *
   * `ensureLoaded()` est idempotent par personne — c'est ce qui évite au layout et
   * à chaque écran de se coordonner. Mais une édition CRÉÉE au back-office (A10)
   * entre dans le périmètre : sans ce forçage, le sélecteur de la tête de page
   * ignorerait l'édition qu'on vient de créer jusqu'au prochain rechargement
   * complet de l'application.
   */
  async function reload(): Promise<void> {
    loadedFor.value = null
    await ensureLoaded()
  }

  return {
    events,
    scope,
    currentEventId,
    currentEvent,
    isRestricted,
    isEmpty,
    canAdminister,
    isLoading,
    loadError,
    ensureLoaded,
    reload,
    setEvents,
    selectEvent,
  }
})
