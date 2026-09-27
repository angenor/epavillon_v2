/**
 * La saisie de « Proposer un terme » gardée pendant la connexion (récit 7, scénario 3) :
 * elle ne revient que sur l'écran qui l'a gardée, et dans l'heure, comme le retour
 * après connexion.
 */
export interface SaisieDeTerme {
  terme: string
  contexte: string
}

const VALIDE_MS = 60 * 60 * 1000

export function saisieAGarder(chemin: string, saisie: SaisieDeTerme, maintenant: number): string {
  return JSON.stringify({ chemin, ...saisie, a: maintenant })
}

/** Le chemin compare sans la requête : la recherche du lexique la réécrit à chaque frappe. */
export function saisieALire(brut: string | null, chemin: string, maintenant: number): SaisieDeTerme | null {
  if (!brut) return null
  try {
    const lu = JSON.parse(brut) as Record<string, unknown>
    const { terme, contexte, a } = lu
    if (typeof terme !== 'string' || typeof contexte !== 'string' || typeof a !== 'number') return null
    if (typeof lu.chemin !== 'string' || lu.chemin.split('?')[0] !== chemin.split('?')[0]) return null
    return maintenant - a >= 0 && maintenant - a < VALIDE_MS ? { terme, contexte } : null
  } catch {
    return null
  }
}
