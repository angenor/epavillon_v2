/**
 * Quand les documents réservés s'effacent — et surtout quand ils ne s'effacent pas.
 *
 * Deux enchaînements, écrits une fois pour se prouver sans navigateur : la
 * déconnexion efface **avant** de fermer la session, et une relecture n'efface que
 * sur une réponse. Une API muette lève avant, et rien ne s'efface (reprise 1 de 0c).
 */
import type { IssueDeRotation } from '../rotation.ts'

export interface EtapesDeDeconnexion {
  effacerLesReserves(): Promise<void>
  viderLaFile(): Promise<void>
  fermerLaSession(): Promise<void>
}

/** Un effacement qui échoue n'empêche pas de se déconnecter : c'est ce que la personne a demandé. */
export async function deconnecterDansLOrdre(etapes: EtapesDeDeconnexion): Promise<void> {
  await etapes.effacerLesReserves().catch(() => undefined)
  await etapes.viderLaFile()
  await etapes.fermerLaSession()
}

/**
 * Lit ; si la réponse dit l'accès ou la session perdus, efface les réservés, puis
 * rend la lecture. Un effacement qui échoue ne fait pas tomber la lecture.
 */
export async function relireEtEffacer<T>(
  lire: () => Promise<T>,
  perdu: (lu: T) => boolean,
  effacer: () => Promise<void>,
): Promise<T> {
  const lu = await lire()
  if (perdu(lu)) await effacer().catch(() => undefined)
  return lu
}

/**
 * Le jeton d'accès vit un quart d'heure, et une lecture publique faite sans lui ne
 * reçoit pas de 401 : elle passe pour anonyme, et les réservés d'une personne qui a
 * l'accès y paraissent fermés. Quand la réponse a cet air-là, le jeton tourne et la
 * lecture se refait une fois. Une rotation sans réponse ne conclut rien : elle lève,
 * et rien ne s'applique ni ne s'efface.
 */
export async function lireEnPersonne<T>(
  lire: () => Promise<T>,
  sembleAnonyme: (lu: T) => boolean,
  tourner: () => Promise<IssueDeRotation>,
): Promise<T> {
  const lu = await lire()
  if (!sembleAnonyme(lu)) return lu
  const issue = await tourner()
  if (issue === 'injoignable') throw new Error('rotation sans réponse')
  return issue === 'renouvelee' ? lire() : lu
}
