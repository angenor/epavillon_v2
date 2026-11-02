/**
 * Une seule rotation du jeton à la fois **entre onglets**.
 *
 * Les onglets partagent les cookies : deux qui rafraîchissent ensemble
 * présentent le même jeton, et c'est ainsi que le commanditaire a été déconnecté
 * de partout le 02/11 (toutes les lignes temps réel reçoivent `reauth` au même
 * instant, puisqu'elles portent le même jeton d'accès). Le verrou du navigateur
 * (`navigator.locks`) les met en file ; celui qui passe après une rotation
 * réussie ailleurs ne tourne pas : les cookies neufs valent déjà pour lui, il
 * rejoue sa requête. Sans verrou (navigateur ancien), la marque seule évite la
 * plupart des doublons, et la tolérance de l'API (ADR-024) couvre le reste.
 */
import type { IssueDeRotation } from './rotation'

export const VERROU_ROTATION = 'epavillon:rotation'
export const MARQUE_ROTATION = 'epavillon:rotation-reussie'
/** Rotation réussie dans un autre onglet depuis moins que cela : ses cookies valent pour tous. */
export const ROTATION_FRAICHE_MS = 10_000
/** Au-delà, l'onglet cesse d'attendre le verrou et tourne lui-même. */
export const ATTENTE_MAX_MS = 15_000

const ONGLET = Math.random().toString(36).slice(2)

export interface Marque {
  quand: number
  onglet: string
}

/** Ce que l'utilitaire demande au navigateur ; injecté pour se tester sans lui. */
export interface Onglets {
  /** Identifiant de cet onglet. */
  onglet: string
  /** `navigator.locks.request`, absent si le navigateur ne l'a pas. */
  verrou?: <T>(nom: string, signal: AbortSignal, travail: () => Promise<T>) => Promise<T>
  lireMarque: () => Marque | null
  poserMarque: (marque: Marque) => void
  maintenant: () => number
}

export async function tournerEntreOnglets<R extends { issue: IssueDeRotation }>(
  tourner: () => Promise<R>,
  dejaTournee: R,
  onglets: Onglets,
): Promise<R> {
  // Une marque de cet onglet ne dit rien : c'est son propre cookie qui vient d'échouer.
  const fraiche = (): boolean => {
    const marque = onglets.lireMarque()
    return marque !== null
      && marque.onglet !== onglets.onglet
      && onglets.maintenant() - marque.quand < ROTATION_FRAICHE_MS
  }
  const travail = async (): Promise<R> => {
    if (fraiche()) return dejaTournee
    const rotation = await tourner()
    if (rotation.issue === 'renouvelee') onglets.poserMarque({ quand: onglets.maintenant(), onglet: onglets.onglet })
    return rotation
  }

  if (!onglets.verrou) return travail()

  const abandon = new AbortController()
  const delai = setTimeout(() => abandon.abort(), ATTENTE_MAX_MS)
  try {
    return await onglets.verrou(VERROU_ROTATION, abandon.signal, () => {
      clearTimeout(delai)
      return travail()
    })
  } catch (erreur) {
    // Attente trop longue (onglet figé qui tient le verrou) : on tourne quand même.
    if (abandon.signal.aborted) return travail()
    throw erreur
  } finally {
    clearTimeout(delai)
  }
}

/** Les onglets du navigateur courant. Le stockage peut manquer (navigation privée) : rien n'échoue. */
export function ongletsDuNavigateur(): Onglets {
  const locks = typeof navigator !== 'undefined' ? navigator.locks : undefined
  return {
    onglet: ONGLET,
    verrou: locks
      ? <T>(nom: string, signal: AbortSignal, travail: () => Promise<T>) =>
          new Promise<T>((resolve, reject) => {
            // Le verrou est tenu tant que la promesse rendue au rappel n'est pas réglée.
            locks.request(nom, { signal }, () => travail().then(resolve, reject)).catch(reject)
          })
      : undefined,
    lireMarque: () => {
      try {
        const brute = localStorage.getItem(MARQUE_ROTATION)
        const marque = brute ? (JSON.parse(brute) as Partial<Marque>) : null
        return typeof marque?.quand === 'number' && typeof marque.onglet === 'string'
          ? { quand: marque.quand, onglet: marque.onglet }
          : null
      } catch {
        return null
      }
    },
    poserMarque: (marque) => {
      try {
        localStorage.setItem(MARQUE_ROTATION, JSON.stringify(marque))
      } catch {
        // Sans stockage, le verrou seul sérialise ; l'API tolère le reste.
      }
    },
    maintenant: () => Date.now(),
  }
}
