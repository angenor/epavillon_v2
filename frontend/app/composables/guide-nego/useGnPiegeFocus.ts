import type { Ref } from 'vue'

/**
 * Le piège à focus des surfaces modales — feuille basse et boîte de confirmation.
 *
 * Trois choses qu'une modale rate presque toujours, et que celle-ci tient : le focus qui
 * s'échappe derrière le voile, la page qui défile dessous, et le focus perdu à la fermeture
 * — la personne repart alors du haut de l'écran au lieu du bouton qu'elle venait de toucher.
 *
 * Empilées, seule la dernière ouverte répond au clavier : sans cela, une confirmation
 * ouverte depuis une feuille refermerait les deux d'une seule touche Échap.
 */

const FOCALISABLES = [
  'a[href]',
  'button:not(:disabled)',
  'input:not(:disabled)',
  'select:not(:disabled)',
  'textarea:not(:disabled)',
  '[tabindex]:not([tabindex="-1"])',
].join(',')

const pile: symbol[] = []
let defilementRendu = ''

export interface OptionsPiegeFocus {
  /** Échap et, si la surface le permet, le voile passent par là. */
  fermer: () => void
  /** Ce qui reçoit le focus à l'ouverture ; par défaut le premier élément focalisable. */
  focusInitial?: () => HTMLElement | null | undefined
}

export function useGnPiegeFocus(
  ouverte: Ref<boolean>,
  surface: () => HTMLElement | null | undefined,
  options: OptionsPiegeFocus,
) {
  const jeton = Symbol('gn-piege-focus')
  let origine: HTMLElement | null = null
  let engage = false

  function focalisables(): HTMLElement[] {
    const racine = surface()
    if (!racine) return []
    return Array.from(racine.querySelectorAll<HTMLElement>(FOCALISABLES)).filter(
      (element) => element.getClientRects().length > 0,
    )
  }

  function auClavier(evenement: KeyboardEvent) {
    if (pile[pile.length - 1] !== jeton) return

    if (evenement.key === 'Escape') {
      evenement.preventDefault()
      options.fermer()
      return
    }
    if (evenement.key !== 'Tab') return

    const cibles = focalisables()
    const premier = cibles[0]
    const dernier = cibles[cibles.length - 1]
    if (!premier || !dernier) {
      evenement.preventDefault()
      return
    }

    const racine = surface()
    const actif = document.activeElement
    const dehors = !racine || !(actif instanceof Node) || !racine.contains(actif)

    if (evenement.shiftKey && (dehors || actif === premier)) {
      evenement.preventDefault()
      dernier.focus()
    } else if (!evenement.shiftKey && (dehors || actif === dernier)) {
      evenement.preventDefault()
      premier.focus()
    }
  }

  function ouvrir() {
    origine = document.activeElement instanceof HTMLElement ? document.activeElement : null
    if (pile.length === 0) {
      defilementRendu = document.body.style.overflow
      document.body.style.overflow = 'hidden'
    }
    pile.push(jeton)
    document.addEventListener('keydown', auClavier, true)
    void nextTick(() => {
      const cible = options.focusInitial?.() ?? focalisables()[0] ?? surface()
      cible?.focus()
    })
  }

  function refermer() {
    document.removeEventListener('keydown', auClavier, true)
    const rang = pile.lastIndexOf(jeton)
    if (rang !== -1) pile.splice(rang, 1)
    if (pile.length === 0) document.body.style.overflow = defilementRendu

    // La surface n'est pas encore démontée : rendre le focus après, sinon le navigateur
    // le reprend sur `<body>` et la personne perd sa place.
    const retour = origine
    origine = null
    void nextTick(() => retour?.focus())
  }

  function basculer(estOuverte: boolean) {
    if (estOuverte === engage) return
    engage = estOuverte
    if (estOuverte) ouvrir()
    else refermer()
  }

  if (import.meta.client) {
    watch(ouverte, basculer, { immediate: true })
    onBeforeUnmount(() => basculer(false))
  }
}
