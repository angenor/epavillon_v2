/**
 * Les termes favoris (R7). Sans compte, ils vivent sur le téléphone. Avec un compte,
 * ils le suivent : chaque geste part par la file de 0c, une clé par terme, et l'écran
 * montre l'état lu du serveur plus les choix encore dans la file — le patron des
 * favoris de documents.
 *
 * À la connexion, les favoris posés sans compte rejoignent le compte, une fois ;
 * rien n'est effacé d'aucun côté. Le téléphone garde ensuite le reflet du compte :
 * déconnecté, il retrouve ce qu'il montrait.
 */
import { ApiRequestError, normalizeApiError } from '~/utils/api-error'
import { magasinDesEcritures } from '~/utils/guide-nego/garde'
import { idsLus } from '~/utils/guide-nego/lexique'
import {
  CLE_LEXIQUE_FAVORIS,
  CLE_LEXIQUE_FAVORIS_FUSIONNES,
  lireCle,
  poserCle,
} from '~/utils/guide-nego/stockage'

const PREFIXE = 'lexique.favori.'
const CLE_LECTURE = 'mes-termes-favoris'

interface FavorisLus {
  personne: string
  ids: string[]
}

interface FavoriVoulu {
  favori: boolean
}

export function useGnFavorisLexique() {
  const api = useApi().guideNegoSavoir
  const session = useGnSession()
  const file = useGnFile()

  const surLeTelephone = useState<string[]>('gn-lexique-favoris-telephone', () => [])
  const enFileParPersonne = useState<Record<string, Record<string, boolean>>>('gn-lexique-favoris-en-file', () => ({}))
  const personne = computed(() => (session.connectee.value ? session.compte.value.id : null))
  const enFile = computed(() => enFileParPersonne.value[personne.value ?? ''] ?? {})

  const lecture = useGnLecture<FavorisLus>(CLE_LECTURE, async () => {
    const qui = personne.value
    if (!qui) throw new Error('sans compte')
    try {
      const lu = await api.mesTermesFavoris()
      return { personne: qui, ids: lu.valeur.entry_ids }
    } catch (erreur) {
      if (normalizeApiError(erreur) instanceof ApiRequestError) return { personne: qui, ids: [] }
      throw erreur
    }
  })

  const lusPourMoi = computed(() => {
    const lus = lecture.etat.value.valeur
    return lus && lus.personne === personne.value ? lus : null
  })

  const favoris = computed<ReadonlySet<string>>(() => {
    const ids = new Set(personne.value && lusPourMoi.value ? lusPourMoi.value.ids : surLeTelephone.value)
    for (const [id, voulu] of Object.entries(enFile.value)) {
      if (voulu) ids.add(id)
      else ids.delete(id)
    }
    return ids
  })

  function garderSurLeTelephone(ids: Iterable<string>): void {
    surLeTelephone.value = [...ids]
    poserCle(CLE_LEXIQUE_FAVORIS, JSON.stringify(surLeTelephone.value))
  }

  async function relireLaFile(): Promise<void> {
    const qui = personne.value
    if (!qui) return
    const intentions = await magasinDesEcritures.lire().catch(() => [])
    enFileParPersonne.value = {
      ...enFileParPersonne.value,
      [qui]: Object.fromEntries(
        intentions
          .filter((i) => i.personne === qui && i.cle.startsWith(PREFIXE))
          .map((i) => [i.cle.slice(PREFIXE.length), (i.corps as FavoriVoulu).favori]),
      ),
    }
  }

  file.inscrireFamille(
    PREFIXE,
    (intention) => {
      const id = intention.cle.slice(PREFIXE.length)
      return (intention.corps as FavoriVoulu).favori ? api.poserUnTermeFavori(id) : api.retirerUnTermeFavori(id)
    },
    async () => {
      await lecture.relire()
      await relireLaFile()
    },
  )

  async function vouloir(qui: string, id: string, favori: boolean): Promise<void> {
    enFileParPersonne.value = { ...enFileParPersonne.value, [qui]: { ...enFile.value, [id]: favori } }
    await file.poser(`${PREFIXE}${id}`, { favori } satisfies FavoriVoulu, null)
  }

  /** Les favoris sans compte rejoignent le compte, une fois par connexion. */
  async function fusionner(qui: string): Promise<void> {
    const lus = lusPourMoi.value
    if (!lus || lireCle(CLE_LEXIQUE_FAVORIS_FUSIONNES) === qui) return
    const compte = new Set(lus.ids)
    const manquants = surLeTelephone.value.filter((id) => !compte.has(id))
    for (const id of manquants) await vouloir(qui, id, true)
    poserCle(CLE_LEXIQUE_FAVORIS_FUSIONNES, qui)
    if (manquants.length) await file.partir()
  }

  // Le reflet du compte ne s'écrit qu'après la fusion : il effacerait les favoris sans compte.
  watch(favoris, (ids) => {
    const qui = personne.value
    if (qui && lusPourMoi.value && lireCle(CLE_LEXIQUE_FAVORIS_FUSIONNES) === qui) garderSurLeTelephone(ids)
  })

  async function assurer(): Promise<void> {
    surLeTelephone.value = idsLus(lireCle(CLE_LEXIQUE_FAVORIS))
    await session.assurer()
    const qui = personne.value
    if (!qui) {
      poserCle(CLE_LEXIQUE_FAVORIS_FUSIONNES, '')
      return
    }
    await relireLaFile()
    await lecture.rafraichir()
    await fusionner(qui)
  }

  /** Le choix s'affiche tout de suite ; avec un compte, la file l'envoie quand elle peut. */
  async function basculer(id: string): Promise<void> {
    const voulu = !favoris.value.has(id)
    const qui = personne.value
    if (!qui) {
      const ids = new Set(surLeTelephone.value)
      if (voulu) ids.add(id)
      else ids.delete(id)
      garderSurLeTelephone(ids)
      return
    }
    await vouloir(qui, id, voulu)
    await file.partir()
    await relireLaFile()
  }

  return { favoris, assurer, basculer }
}
