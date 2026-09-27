/**
 * Les coches du parcours « Ma première COP » (R7) — le patron des termes favoris.
 * Sans compte, elles vivent sur le téléphone. Avec un compte, chaque geste part par la
 * file de 0c, une clé par étape, et l'écran montre l'état lu du serveur plus les gestes
 * encore dans la file : deux appareils qui touchent la même étape, le dernier arrivé
 * l'emporte.
 *
 * À la connexion, les coches posées sans compte rejoignent le compte, une fois ; rien
 * n'est effacé d'aucun côté. Quand une relecture change ce que l'écran montrait, la
 * ligne « mises à jour depuis un autre appareil » le dit (FR-028).
 */
import { ApiRequestError, normalizeApiError } from '~/utils/api-error'
import { magasinDesEcritures } from '~/utils/guide-nego/garde'
import { idsLus } from '~/utils/guide-nego/lexique'
import { avancement, changeesAilleurs } from '~/utils/guide-nego/ma-premiere-cop'
import { CLE_PARCOURS, CLE_PARCOURS_FUSIONNE, lireCle, poserCle } from '~/utils/guide-nego/stockage'

const PREFIXE = 'parcours.'
const CLE_LECTURE = 'mon-parcours'

interface CochesLues {
  personne: string
  ids: string[]
}

interface CocheVoulue {
  cochee: boolean
}

export function useGnParcours() {
  const api = useApi().guideNegoSavoir
  const session = useGnSession()
  const file = useGnFile()
  const savoir = useGnSavoir()

  const surLeTelephone = useState<string[]>('gn-parcours-telephone', () => [])
  const enFileParPersonne = useState<Record<string, Record<string, boolean>>>('gn-parcours-en-file', () => ({}))
  const misesAJourAilleurs = useState('gn-parcours-ailleurs', () => false)
  const personne = computed(() => (session.connectee.value ? session.compte.value.id : null))
  const enFile = computed(() => enFileParPersonne.value[personne.value ?? ''] ?? {})
  const publiees = computed(() => new Set(savoir.parcours.value.flatMap((g) => g.steps.map((s) => s.id))))

  const lecture = useGnLecture<CochesLues>(CLE_LECTURE, async (connue) => {
    const qui = personne.value
    if (!qui) throw new Error('sans compte')
    let ids: string[]
    try {
      ids = (await api.monParcours()).valeur.step_ids
    } catch (erreur) {
      if (!(normalizeApiError(erreur) instanceof ApiRequestError)) throw erreur
      ids = []
    }
    const publie = publiees.value.size ? publiees.value : null
    if (connue?.personne === qui && changeesAilleurs(connue.ids, ids, Object.keys(enFile.value), publie)) {
      misesAJourAilleurs.value = true
    }
    return { personne: qui, ids }
  })

  const lusPourMoi = computed(() => {
    const lus = lecture.etat.value.valeur
    return lus && lus.personne === personne.value ? lus : null
  })

  const coches = computed<ReadonlySet<string>>(() => {
    const ids = new Set(personne.value && lusPourMoi.value ? lusPourMoi.value.ids : surLeTelephone.value)
    for (const [id, voulue] of Object.entries(enFile.value)) {
      if (voulue) ids.add(id)
      else ids.delete(id)
    }
    return ids
  })

  const etat = computed(() => avancement(savoir.parcours.value, coches.value))

  function garderSurLeTelephone(ids: Iterable<string>): void {
    surLeTelephone.value = [...ids]
    poserCle(CLE_PARCOURS, JSON.stringify(surLeTelephone.value))
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
          .map((i) => [i.cle.slice(PREFIXE.length), (i.corps as CocheVoulue).cochee]),
      ),
    }
  }

  file.inscrireFamille(
    PREFIXE,
    (intention) => {
      const id = intention.cle.slice(PREFIXE.length)
      return (intention.corps as CocheVoulue).cochee ? api.cocherUneEtape(id) : api.decocherUneEtape(id)
    },
    async () => {
      await lecture.relire()
      await relireLaFile()
    },
  )

  async function vouloir(qui: string, id: string, cochee: boolean): Promise<void> {
    enFileParPersonne.value = { ...enFileParPersonne.value, [qui]: { ...enFile.value, [id]: cochee } }
    await file.poser(`${PREFIXE}${id}`, { cochee } satisfies CocheVoulue, null)
  }

  /** Les coches sans compte rejoignent le compte, une fois par connexion. */
  async function fusionner(qui: string): Promise<void> {
    const lus = lusPourMoi.value
    // Sans le parcours publié, on ne sait pas quelles coches envoyer : on attend la prochaine ouverture.
    if (!lus || !publiees.value.size || lireCle(CLE_PARCOURS_FUSIONNE) === qui) return
    const compte = new Set(lus.ids)
    const manquantes = surLeTelephone.value.filter((id) => !compte.has(id) && publiees.value.has(id))
    for (const id of manquantes) await vouloir(qui, id, true)
    poserCle(CLE_PARCOURS_FUSIONNE, qui)
    if (manquantes.length) await file.partir()
  }

  // Le reflet du compte ne s'écrit qu'après la fusion : il effacerait les coches sans compte.
  watch(coches, (ids) => {
    const qui = personne.value
    if (qui && lusPourMoi.value && lireCle(CLE_PARCOURS_FUSIONNE) === qui) garderSurLeTelephone(ids)
  })

  async function assurer(): Promise<void> {
    surLeTelephone.value = idsLus(lireCle(CLE_PARCOURS))
    await Promise.all([session.assurer(), savoir.assurer()])
    const qui = personne.value
    if (!qui) {
      poserCle(CLE_PARCOURS_FUSIONNE, '')
      return
    }
    await relireLaFile()
    await lecture.rafraichir()
    await fusionner(qui)
  }

  /** Au retour du réseau et au retour sur l'écran : la coche d'un autre appareil y paraît (SC-006). */
  function suivre(): () => void {
    const relire = () => {
      if (document.visibilityState === 'visible' && personne.value) void assurer()
    }
    window.addEventListener('online', relire)
    document.addEventListener('visibilitychange', relire)
    return () => {
      window.removeEventListener('online', relire)
      document.removeEventListener('visibilitychange', relire)
    }
  }

  /** Le geste s'affiche tout de suite ; avec un compte, la file l'envoie quand elle peut. */
  async function basculer(id: string): Promise<void> {
    const voulue = !coches.value.has(id)
    const qui = personne.value
    if (!qui) {
      const ids = new Set(surLeTelephone.value)
      if (voulue) ids.add(id)
      else ids.delete(id)
      garderSurLeTelephone(ids)
      return
    }
    await vouloir(qui, id, voulue)
    await file.partir()
    await relireLaFile()
  }

  return {
    coches,
    etat,
    misesAJourAilleurs,
    oublierLaMiseAJour: () => (misesAJourAilleurs.value = false),
    assurer,
    suivre,
    basculer,
  }
}
