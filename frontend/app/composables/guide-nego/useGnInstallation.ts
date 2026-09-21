/**
 * L'installation sur l'écran d'accueil.
 *
 * Chrome n'offre la proposition qu'à un instant qu'il choisit, et une seule fois : on
 * retient l'événement pour le rejouer quand la personne touche le bouton. Safari n'en
 * propose aucune — d'où les étapes écrites, qui restent la seule voie sur iPhone.
 */
interface EvenementInstallation extends Event {
  prompt: () => Promise<void>
  userChoice: Promise<{ outcome: 'accepted' | 'dismissed' }>
}

interface NavigateurApple extends Navigator {
  standalone?: boolean
}

export function useGnInstallation() {
  const proposition = shallowRef<EvenementInstallation | null>(null)
  const proposable = computed(() => proposition.value !== null)
  const installee = ref(false)

  function capter(evenement: Event) {
    evenement.preventDefault()
    proposition.value = evenement as EvenementInstallation
  }

  function oublier() {
    proposition.value = null
    installee.value = true
  }

  onMounted(() => {
    installee.value =
      window.matchMedia('(display-mode: standalone)').matches ||
      (navigator as NavigateurApple).standalone === true
    window.addEventListener('beforeinstallprompt', capter)
    window.addEventListener('appinstalled', oublier)
  })

  onBeforeUnmount(() => {
    window.removeEventListener('beforeinstallprompt', capter)
    window.removeEventListener('appinstalled', oublier)
  })

  /** Rend faux quand le navigateur n'a rien à proposer : l'écran montre alors les étapes. */
  async function installer(): Promise<boolean> {
    const evenement = proposition.value
    if (!evenement) return false
    await evenement.prompt()
    const { outcome } = await evenement.userChoice
    proposition.value = null
    return outcome === 'accepted'
  }

  return { proposable, installee, installer }
}
