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
 * la garde en place, avec son heure.
 */
export function useGnLecture<T>(cle: string, lire: () => Promise<T>) {
  const etat = useState<EtatLecture<T>>(`gn-lecture-${cle}`, () => ({
    valeur: null,
    luA: null,
    source: 'aucune',
    pret: false,
    enCours: false,
  }))
  const connexion = useGnConnexion()

  async function rafraichir(): Promise<void> {
    if (etat.value.enCours) return
    etat.value.enCours = true

    if (!etat.value.pret) {
      const garde = await lireGarde<T>(cle)
      if (garde && etat.value.source === 'aucune') {
        etat.value = { ...etat.value, valeur: garde.valeur, luA: garde.lu_a, source: 'garde', pret: true }
        connexion.noterLecture(garde.lu_a)
      }
    }

    try {
      const valeur = await avecDelai(lire())
      const luA = new Date().toISOString()
      etat.value = { valeur, luA, source: 'reseau', pret: true, enCours: false }
      connexion.noterReussite(luA)
      await ecrireGarde({ cle, valeur, lu_a: luA, empreinte: null })
    } catch {
      etat.value = { ...etat.value, pret: true, enCours: false }
      connexion.noterEchec()
    }
  }

  return { etat, rafraichir }
}
