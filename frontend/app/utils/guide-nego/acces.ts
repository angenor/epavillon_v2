/**
 * L'état d'accès, tel que les écrans l'interrogent — **et ce qu'il devient sans
 * réseau**.
 *
 * Tout est pur ici, et c'est délibéré : les règles qui comptent se trompent
 * silencieusement, et un test doit pouvoir les prendre sans navigateur.
 *
 * LA RÈGLE QUI COMPTE : **hors connexion, on n'annonce jamais un accès qu'on n'a
 * pas obtenu** (FR-019), et on n'efface jamais celui qui a été lu (FR-034). Ce
 * sont deux moitiés de la même prudence — dire ce qu'on sait, avec l'heure où on
 * l'a su, et rien de plus.
 */
import type { AccessStateView } from '~/types/negotiation'

export const ACCES_VISITEUSE: AccessStateView = {
  admission_mode: 'code',
  state: 'visitor',
  networks: [],
  granted: null,
  request: null,
}

/**
 * **Un objet simple, jamais la réponse brute.** Ce que rend cette fonction part
 * dans IndexedDB et dans l'état du rendu serveur : une réponse entière y
 * garderait des champs dont l'écran n'a pas l'usage, et une instance de classe
 * ne s'y sérialiserait pas.
 */
export function etatDAcces(lu: AccessStateView | null): AccessStateView {
  if (!lu) return ACCES_VISITEUSE

  return {
    admission_mode: lu.admission_mode,
    state: lu.state,
    granted: lu.granted ?? null,
    networks: lu.networks ?? [],
    request: lu.request ?? null,
  }
}

/**
 * Les modules réservés sont-ils ouverts ?
 *
 * **Un seul état l'ouvre**, et c'est celui que l'API a dérivé du RBAC. Un écran
 * qui testerait « pas visiteuse » ouvrirait les modules à une personne dont
 * l'accès vient d'être retiré.
 */
export function accesOuvert(etat: AccessStateView | null): boolean {
  return etat?.state === 'granted'
}

/**
 * La saisie d'un code est-elle possible maintenant ?
 *
 * **Elle exige le réseau, et l'écran le dit** (FR-019). Rien n'est mis en file :
 * un accès n'est pas un signalement, on ne peut pas l'annoncer avant de l'avoir
 * obtenu. Et le mode « approbation seule » ne propose pas de code du tout
 * (FR-022) — c'est la demande qui prend sa place.
 */
export function saisiePossible(etat: AccessStateView | null, enLigne: boolean): boolean {
  if (!enLigne) return false
  return etat?.admission_mode !== 'approval'
}

/**
 * Ce que l'écran montre hors connexion : ce qui a été lu, avec l'heure de sa
 * lecture. **Jamais un écran vide**, et jamais non plus la prétention d'un accès
 * — `state` reste ce que la dernière lecture disait.
 */
export function accesLisibleHorsConnexion(
  etat: AccessStateView | null,
  luA: string | null,
): { etat: AccessStateView; luA: string | null } {
  return { etat: etat ?? ACCES_VISITEUSE, luA }
}

/**
 * La demande d'accès est-elle proposée ?
 *
 * **Dès que le mode exige une approbation** — « approbation seule » et « code
 * et approbation » —, et jamais quand l'accès est déjà ouvert ou qu'une demande
 * attend déjà : il n'y a alors rien à demander (FR-024).
 *
 * En mode « code seul », elle ne l'est pas non plus : l'entrée passe par le
 * code, et offrir une demande que personne ne traiterait laisserait attendre
 * une réponse qui ne viendrait pas.
 */
export function demandePossible(etat: AccessStateView | null, enLigne: boolean): boolean {
  if (!enLigne) return false
  if (!etat) return false
  if (etat.state === 'granted' || etat.state === 'pending') return false
  return etat.admission_mode !== 'code'
}

/**
 * Le parcours propose-t-il la saisie d'un code **dans ce mode** ?
 *
 * FR-022 : en « approbation seule », le champ disparaît du parcours et la
 * demande prend sa place. À la différence de `saisiePossible`, cette fonction
 * ne regarde pas le réseau : elle dit ce que le MODE offre, pas ce qui est
 * faisable à l'instant. Les confondre ferait disparaître le champ à chaque
 * tunnel, au lieu de le désactiver en le disant.
 */
export function codeOffertParLeMode(etat: AccessStateView | null): boolean {
  return etat?.admission_mode !== 'approval'
}
