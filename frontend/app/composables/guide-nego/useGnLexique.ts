/**
 * Ce que le lexique retient sur le téléphone, hors du savoir : d'où il a été ouvert,
 * et les derniers termes consultés.
 */
import { derniersApres, idsLus } from '~/utils/guide-nego/lexique'
import { CLE_LEXIQUE_DERNIERS, lireCle, poserCle } from '~/utils/guide-nego/stockage'

const LEXIQUE = '/guide-nego/lexique'

/** La croix ramène à l'écran d'où « Aa » a été touché ; sans lui, à l'accueil. */
export function useGnOrigineDuLexique() {
  const origine = useState<string>('gn-lexique-origine', () => '/guide-nego')

  /** À l'ouverture d'un écran du lexique : l'écran d'avant, s'il n'en est pas un. */
  function retenir(): void {
    const avant: unknown = window.history.state?.back
    if (typeof avant !== 'string' || avant === LEXIQUE || avant.startsWith(`${LEXIQUE}/`) || avant.startsWith(`${LEXIQUE}?`)) return
    origine.value = avant
  }

  return { origine, retenir }
}

export function useGnDerniersTermes() {
  const derniers = useState<string[]>('gn-lexique-derniers', () => [])

  return {
    derniers,
    relire: () => (derniers.value = idsLus(lireCle(CLE_LEXIQUE_DERNIERS))),
    noter(id: string): void {
      derniers.value = derniersApres(idsLus(lireCle(CLE_LEXIQUE_DERNIERS)), id)
      poserCle(CLE_LEXIQUE_DERNIERS, JSON.stringify(derniers.value))
    },
  }
}
