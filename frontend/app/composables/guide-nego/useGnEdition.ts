import { lireGarde } from '~/utils/guide-nego/garde'
import { editionComplete, editionDuGuide, type EditionGardee } from '~/utils/guide-nego/edition'

const CLE = 'edition'

/**
 * L'édition que Guide Négo accompagne, gardée pour le hors connexion : son
 * libellé, et depuis 3a le `slug` et le fuseau dont les sessions ont besoin.
 */
export function useGnEdition() {
  const api = useApi()
  const lecture = useGnLecture<EditionGardee | null>(CLE, async () => editionDuGuide(await api.events.publicList()))

  const edition = computed<EditionGardee | null>(() => {
    const valeur = lecture.etat.value.valeur
    return editionComplete(valeur) ? valeur : null
  })

  /** Sans attendre le réseau : ce qui est en mémoire, sinon la garde. */
  async function gardee(): Promise<EditionGardee | null> {
    if (edition.value) return edition.value
    const valeur = (await lireGarde<EditionGardee | null>(CLE))?.valeur
    return editionComplete(valeur) ? valeur : null
  }

  /** Relue par le réseau une fois ; sinon la garde. */
  async function lue(): Promise<EditionGardee | null> {
    if (lecture.etat.value.source !== 'reseau') await lecture.rafraichir()
    return edition.value
  }

  return { edition, etat: lecture.etat, gardee, lue, rafraichir: lecture.rafraichir }
}
