/**
 * Les favoris de documents, qui suivent le compte d'un appareil à l'autre (FR-029).
 *
 * Posés et retirés par la file de 0c, une clé par document : un favori posé sans
 * réseau part à son retour, une seule fois. Le corps est l'état voulu, sans
 * `If-Match` — poser deux fois ne crée rien de plus.
 *
 * Ce qui s'affiche est l'état lu du serveur, **plus les choix encore dans la file** :
 * une lecture partie avant un envoi ne rallume ni n'éteint rien à tort. La garde
 * porte la personne : sur un téléphone partagé, les favoris d'une autre ne
 * s'affichent pas, et la déconnexion les efface (`useGnSession`).
 */
import { ApiRequestError, normalizeApiError } from '~/utils/api-error'
import { CLE_LECTURE_FAVORIS, magasinDesEcritures } from '~/utils/guide-nego/garde'

const PREFIXE = 'favori-'

interface FavorisLus {
  personne: string
  ids: string[]
}

interface FavoriVoulu {
  favori: boolean
}

export function useGnFavoris() {
  const api = useApi().guideNegoDocuments
  const session = useGnSession()
  const file = useGnFile()
  const enFile = useState<Record<string, boolean>>('gn-favoris-en-file', () => ({}))

  const lecture = useGnLecture<FavorisLus>(CLE_LECTURE_FAVORIS, async () => {
    const personne = session.compte.value.id
    if (!personne) throw new Error('sans compte')
    try {
      const lu = await api.mesFavoris()
      return { personne, ids: lu.valeur.bookmarks.map((b) => b.document_id) }
    } catch (erreur) {
      // L'API a parlé — session close ailleurs : ce n'est pas une panne de réseau.
      if (normalizeApiError(erreur) instanceof ApiRequestError) return { personne, ids: [] }
      throw erreur
    }
  })

  async function relireLaFile(): Promise<void> {
    const personne = session.compte.value.id
    const intentions = await magasinDesEcritures.lire().catch(() => [])
    enFile.value = Object.fromEntries(
      intentions
        .filter((i) => i.personne === personne && i.cle.startsWith(PREFIXE))
        .map((i) => [i.cle.slice(PREFIXE.length), (i.corps as FavoriVoulu).favori]),
    )
  }

  const favoris = computed<ReadonlySet<string>>(() => {
    const lus = lecture.etat.value.valeur
    const ids = new Set(lus && lus.personne === session.compte.value.id ? lus.ids : [])
    for (const [id, voulu] of Object.entries(enFile.value)) {
      if (voulu) ids.add(id)
      else ids.delete(id)
    }
    return ids
  })

  file.inscrireFamille(
    PREFIXE,
    (intention) => {
      const id = intention.cle.slice(PREFIXE.length)
      return (intention.corps as FavoriVoulu).favori ? api.poserUnFavori(id) : api.retirerUnFavori(id)
    },
    async () => {
      await lecture.relire()
      await relireLaFile()
    },
  )

  // Sans compte, la lecture échouerait, et l'échec passerait pour une panne de réseau.
  function assurer(): void {
    if (!session.connectee.value) return
    void relireLaFile()
    void lecture.rafraichir()
  }

  /** Le choix s'affiche tout de suite ; la file l'envoie quand elle peut. */
  async function basculer(id: string): Promise<void> {
    if (!session.compte.value.id) return
    const voulu = !favoris.value.has(id)
    enFile.value = { ...enFile.value, [id]: voulu }
    await file.poser(`${PREFIXE}${id}`, { favori: voulu } satisfies FavoriVoulu, null)
    await file.partir()
    await relireLaFile()
  }

  return {
    favoris,
    pret: computed(() => lecture.etat.value.pret),
    luA: computed(() => lecture.etat.value.luA),
    assurer,
    basculer,
  }
}
