/**
 * Le verrou d'un module réservé : **ouvre-t-il, et sinon, où mène-t-il ?**
 *
 * Tout est pur ici, et c'est délibéré : la règle qui compte — celle qui décide
 * si un module s'ouvre — se trompe silencieusement, et un test doit pouvoir la
 * prendre sans navigateur.
 *
 * # LE VERROU SUIT `me/access`, ET RIEN D'AUTRE
 *
 * FR-032. `accesOuvert` ne regarde que `state`, dérivé du RBAC par l'API. Un
 * client qui **prétendrait** avoir l'accès — un `granted` bricolé dans la garde
 * locale, un état recopié d'une lecture d'hier — n'ouvre rien : l'état fait foi,
 * et il ne se déduit d'aucun autre champ.
 *
 * # OÙ MÈNE LE VERROU DÉPEND DU MODE, PAS DE L'ÉCRAN
 *
 * Un écran qui enverrait toujours vers la saisie du code proposerait, en mode
 * « approbation seule », un champ qui n'ouvre rien (FR-022). Et une personne
 * qui attend déjà une réponse n'a pas à ressaisir quoi que ce soit : on la
 * ramène à sa demande.
 */
import type { AccessStateView } from '~/types/negotiation'
import { accesOuvert, codeOffertParLeMode } from './acces.ts'

/** Les quatre suites possibles, dans l'ordre où on les rencontre. */
export type SortieDuVerrou = 'compte' | 'attente' | 'demande' | 'code'

/**
 * Ce que le verrou propose de faire.
 *
 * **Le compte passe avant tout** (FR-031) : saisir un code ou demander l'accès
 * sans session n'ouvrirait rien, et le refus viendrait de l'API sans que l'écran
 * ait rien expliqué.
 */
export function sortieDuVerrou(etat: AccessStateView | null, connectee: boolean): SortieDuVerrou {
  if (!connectee) return 'compte'
  if (etat?.request?.status === 'pending') return 'attente'
  if (!codeOffertParLeMode(etat)) return 'demande'
  return 'code'
}

/** L'adresse de cette suite. « Attente » et « demande » mènent au même écran,
 *  qui sait lequel des deux états afficher. */
export function adresseDeLaSortie(sortie: SortieDuVerrou): string {
  switch (sortie) {
    case 'compte':
      return '/guide-nego/compte'
    case 'attente':
    case 'demande':
      return '/guide-nego/demande'
    case 'code':
      return '/guide-nego/code'
  }
}

/**
 * Le module est-il ouvert ?
 *
 * Redit ici pour que le verrou n'ait qu'un seul point d'entrée, et parce que la
 * règle se lit mieux à côté de ce qu'elle commande. La décision reste celle
 * d'`accesOuvert` : un seul état ouvre, celui que l'API a dérivé du RBAC.
 */
export function moduleOuvert(etat: AccessStateView | null): boolean {
  return accesOuvert(etat)
}
