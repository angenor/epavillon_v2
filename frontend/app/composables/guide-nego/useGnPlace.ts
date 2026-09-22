/**
 * La place occupée sur le téléphone : la mesurer, et la libérer.
 *
 * La mesure est l'estimation du navigateur, seule à compter l'ensemble — coquille,
 * données lues, documents à venir. Là où elle manque, `place` reste nulle et l'écran
 * le dit (R8). La libération vide les données lues, et à partir de l'étape 1 les
 * documents ; **la coquille reste**, comme ce qui permet de l'ouvrir sans réseau.
 */
import { viderLesGardes } from '~/utils/guide-nego/garde'
import { GARDES_DE_LA_COQUILLE, placeDe, type Place } from '~/utils/guide-nego/place'

export function useGnPlace() {
  const place = ref<Place | null>(null)
  const mesuree = ref(false)

  async function mesurer(): Promise<void> {
    try {
      place.value = placeDe(navigator.storage?.estimate ? await navigator.storage.estimate() : null)
    } catch {
      place.value = null
    }
    mesuree.value = true
  }

  async function liberer(): Promise<void> {
    await viderLesGardes(GARDES_DE_LA_COQUILLE)
    await mesurer()
  }

  return { place: readonly(place), mesuree: readonly(mesuree), mesurer, liberer }
}
