import { ecrireGarde, lireGarde } from '~/utils/guide-nego/garde'

export type SourceLecture = 'reseau' | 'garde' | 'aucune'

export interface EtatLecture<T> {
  valeur: T | null
  /** Instant ISO de la dernière lecture réussie ; nul si rien n'a jamais été lu. */
  luA: string | null
  source: SourceLecture
  /** Vrai dès qu'un verdict est possible : la garde a répondu, ou le réseau. */
  pret: boolean
  /** Vrai tant que la lecture du réseau n'a pas abouti ou échoué. */
  enCours: boolean
}

const DELAI_MS = 5000

// Les lectures en vol, par clé : une promesse ne va pas dans un `useState`.
const enVol = new Map<string, Promise<void>>()

function avecDelai<T>(promesse: Promise<T>): Promise<T> {
  return new Promise((resolve, reject) => {
    const minuterie = setTimeout(() => reject(new Error('délai dépassé')), DELAI_MS)
    promesse.then(resolve, reject).finally(() => clearTimeout(minuterie))
  })
}

/**
 * Lit par le réseau, garde ce qui a été lu, et rend la garde quand le réseau manque.
 *
 * La garde vaut AUSSITÔT : l'application n'attend pas un réseau saturé pour s'ouvrir.
 * La réponse du réseau s'applique à son arrivée. Un échec ne lève jamais — il laisse
 * la garde en place, avec son heure. `lire` reçoit ce qui est gardé, pour compléter une
 * réponse partielle.
 */
export function useGnLecture<T>(cle: string, lire: (garde: T | null) => Promise<T>) {
  const etat = useState<EtatLecture<T>>(`gn-lecture-${cle}`, () => ({
    valeur: null,
    luA: null,
    source: 'aucune',
    pret: false,
    enCours: false,
  }))
  const connexion = useGnConnexion()

  function rafraichir(): Promise<void> {
    const courante = enVol.get(cle)
    if (courante) return courante
    const lecture = lireUneFois().finally(() => enVol.delete(cle))
    enVol.set(cle, lecture)
    return lecture
  }

  /** Après une écriture : une lecture partie avant elle rendrait l'état d'avant. */
  async function relire(): Promise<void> {
    await enVol.get(cle)
    await rafraichir()
  }

  async function lireUneFois(): Promise<void> {
    etat.value.enCours = true

    if (!etat.value.pret) {
      const garde = await lireGarde<T>(cle)
      if (garde && etat.value.source === 'aucune') {
        etat.value = { ...etat.value, valeur: garde.valeur, luA: garde.lu_a, source: 'garde', pret: true }
        connexion.noterLecture(garde.lu_a)
      }
    }

    try {
      const valeur = await avecDelai(lire(etat.value.valeur))
      const luA = new Date().toISOString()
      etat.value = { valeur, luA, source: 'reseau', pret: true, enCours: false }
      connexion.noterReussite(luA)
      await ecrireGarde({ cle, valeur, lu_a: luA, empreinte: null })
    } catch {
      etat.value = { ...etat.value, pret: true, enCours: false }
      connexion.noterEchec()
    }
  }

  return { etat, rafraichir, relire }
}
