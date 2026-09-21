import { lireChoix, themeAffiche, type ChoixDeTheme, type ThemeAffiche } from '~/utils/guide-nego/theme'

const CLE = 'gn.theme'

/**
 * Le thème de Guide Négo, gardé sur le téléphone et lu avant le premier affichage.
 *
 * Il ne touche ni `<html>`, ni le cookie du site : `data-theme` se pose sur l'élément
 * de l'application, et le réglage du site reste ce qu'il était.
 */
export function useGnTheme() {
  const choix = useState<ChoixDeTheme>('gn-theme', () => 'systeme')
  const telephoneEnSombre = useState('gn-theme-telephone', () => false)
  const ecoute = useState('gn-theme-ecoute', () => false)

  const affiche = computed<ThemeAffiche>(() => themeAffiche(choix.value, telephoneEnSombre.value))

  if (import.meta.client && !ecoute.value) {
    ecoute.value = true
    try {
      choix.value = lireChoix(localStorage.getItem(CLE))
    } catch {
      /* Stockage refusé : le réglage du téléphone fera l'affaire. */
    }
    const requete = window.matchMedia('(prefers-color-scheme: dark)')
    telephoneEnSombre.value = requete.matches
    requete.addEventListener('change', (evenement) => (telephoneEnSombre.value = evenement.matches))
  }

  function choisir(nouveau: ChoixDeTheme) {
    choix.value = nouveau
    try {
      localStorage.setItem(CLE, nouveau)
    } catch {
      /* Le choix vaudra pour cette ouverture seulement. */
    }
  }

  return { choix, affiche, choisir }
}
