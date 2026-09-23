/**
 * Quand les documents réservés s'effacent — et surtout quand ils ne s'effacent pas.
 *
 * Deux enchaînements, écrits une fois pour se prouver sans navigateur : la
 * déconnexion efface **avant** de fermer la session, et une relecture n'efface que
 * sur une réponse. Une API muette lève avant, et rien ne s'efface (reprise 1 de 0c).
 */

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
