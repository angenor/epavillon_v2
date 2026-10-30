import type { AttentionSnapshot, WorkspaceEditionTab } from '~/utils/workspace-editions'

/**
 * Ce que la personne a déjà vu de son espace, pour éteindre les pastilles.
 *
 * Deux relevés : un par onglet d'édition (vu en ouvrant l'onglet) et un pour le
 * bouton du compte (vu en ouvrant « Mes activités »). Gardés dans ce navigateur :
 * c'est un confort de lecture, et un autre appareil rallume simplement la
 * pastille une fois.
 */

const PREFIX = 'epavillon:workspace-seen:'

interface SeenState {
  nav: AttentionSnapshot
  tabs: Record<string, AttentionSnapshot>
}

export function useWorkspaceSeen() {
  const auth = useAuthStore()
  const state = useState<SeenState>('workspace-seen', () => ({ nav: {}, tabs: {} }))
  const loadedFor = useState<string | null>('workspace-seen-person', () => null)

  const storageKey = computed(() => (auth.person ? PREFIX + auth.person.id : null))

  function load(): void {
    const key = storageKey.value
    if (!import.meta.client || !key || loadedFor.value === key) return
    loadedFor.value = key
    try {
      const raw = localStorage.getItem(key)
      const parsed = raw ? (JSON.parse(raw) as Partial<SeenState>) : null
      state.value = { nav: parsed?.nav ?? {}, tabs: parsed?.tabs ?? {} }
    } catch {
      state.value = { nav: {}, tabs: {} }
    }
  }

  function save(): void {
    const key = storageKey.value
    if (!key) return
    try {
      localStorage.setItem(key, JSON.stringify(state.value))
    } catch {
      // Stockage refusé : la pastille se rallumera au prochain chargement.
    }
  }

  onMounted(load)
  watch(storageKey, load)

  return {
    tabUnseen: (tab: WorkspaceEditionTab): number =>
      unseenAttention(editionTabSnapshot(tab), state.value.tabs[tab.edition.id]),
    navUnseen: (tabs: WorkspaceEditionTab[]): number => unseenAttention(mergedSnapshot(tabs), state.value.nav),
    markTab(tab: WorkspaceEditionTab): void {
      load()
      state.value = { ...state.value, tabs: { ...state.value.tabs, [tab.edition.id]: editionTabSnapshot(tab) } }
      save()
    },
    markNav(tabs: WorkspaceEditionTab[]): void {
      load()
      state.value = { ...state.value, nav: mergedSnapshot(tabs) }
      save()
    },
  }
}
