/**
 * La place occupée sur le téléphone.
 *
 * La mesure est l'estimation du navigateur, seule à compter l'ensemble — coquille,
 * données lues, documents à venir. Là où elle manque, `place` reste nulle et l'écran
 * le dit (R8). Libérer viendra avec les documents, à l'étape 1, et ne visera qu'eux.
 */
import { placeDe, type Place } from '~/utils/guide-nego/place'

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

  return { place: readonly(place), mesuree: readonly(mesuree), mesurer }
}
