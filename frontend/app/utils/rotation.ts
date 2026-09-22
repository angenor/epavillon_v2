/**
 * Ce que dit une tentative de rotation du jeton — **trois issues, pas deux**.
 *
 * Seule une réponse qui dit « session finie » déconnecte : 200 `expired`, ou 401
 * sur un rejeu. Tout le reste — erreur réseau, délai, 5xx, réponse illisible — dit
 * seulement que l'API ne répond pas. La confondre avec une fin de session effaçait
 * le témoin sur un réseau saturé : la session de quatre-vingt-dix jours était perdue
 * pour rien, dans la situation même pour laquelle Guide Négo existe.
 */
export type IssueDeRotation = 'renouvelee' | 'finie' | 'injoignable'

/** `null` : aucune réponse n'est arrivée. */
export function issueDeRotation(reponse: { status: number; corps: unknown } | null): IssueDeRotation {
  if (!reponse) return 'injoignable'
  if (reponse.status === 401) return 'finie'
  if (reponse.status !== 200) return 'injoignable'
  const statut = (reponse.corps as { status?: unknown } | null)?.status
  if (statut === 'renewed') return 'renouvelee'
  if (statut === 'expired') return 'finie'
  return 'injoignable'
}
