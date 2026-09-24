/**
 * Le mode de lecture gardé sur le téléphone — « Pages » ou « Texte agrandi » —, et
 * l'annonce qui dit, une fois, que le second existe (FR-020 bis, FR-021).
 */
import {
  annoncerLeTexteAgrandi,
  lireMode,
  noterLAnnonceVue,
  poserMode,
  type ModeDeLecture,
} from '~/utils/guide-nego/appareil-lecture'
import { lireCle, poserCle } from '~/utils/guide-nego/stockage'

const stockage = { lire: lireCle, poser: poserCle }

export function useGnModeDeLecture() {
  const mode = useState<ModeDeLecture>('gn-mode-lecture', () => (import.meta.client ? lireMode(stockage) : 'pages'))

  function choisir(nouveau: ModeDeLecture): void {
    mode.value = nouveau
    poserMode(stockage, nouveau)
  }

  const annoncer = (texteOffert: boolean): boolean =>
    import.meta.client && annoncerLeTexteAgrandi(stockage, texteOffert, { largeur: window.innerWidth, hauteur: window.innerHeight })

  return { mode: readonly(mode), choisir, annoncer, noterLAnnonceVue: () => noterLAnnonceVue(stockage) }
}
