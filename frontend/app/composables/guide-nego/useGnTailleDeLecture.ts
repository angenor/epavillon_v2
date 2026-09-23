/**
 * La taille du texte du lecteur, gardée sur le téléphone : une pour tous les documents
 * (FR-042), réglée depuis le lecteur comme depuis le profil.
 */
import { lireTaille, poserTaille, TAILLES, type TailleDuTexte } from '~/utils/guide-nego/appareil-lecture'
import { lireCle, poserCle } from '~/utils/guide-nego/stockage'

const stockage = { lire: lireCle, poser: poserCle }

export function useGnTailleDeLecture() {
  const taille = useState<TailleDuTexte>('gn-taille-lecture', () => (import.meta.client ? lireTaille(stockage) : TAILLES[0]))

  function choisir(nouvelle: TailleDuTexte): void {
    taille.value = nouvelle
    poserTaille(stockage, nouvelle)
  }

  return { taille: readonly(taille), choisir, TAILLES }
}
