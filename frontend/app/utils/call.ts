/**
 * État d'un appel à propositions, tel que la BASE le calcule.
 *
 * Deux fonctions SQL portent cette logique et font foi (`060_events.sql` § 5) :
 *
 *   · `event.effective_deadline(call)` = `COALESCE(extended_until, closes_at)`.
 *     La prolongation est conservée À CÔTÉ de l'échéance annoncée, et non à sa
 *     place : la v1 écrasait la première, et plus personne ne savait dire à une
 *     organisation ce qui lui avait été annoncé au départ.
 *   · `event.is_call_open(call)` = statut `open` ET instant compris entre
 *     l'ouverture et l'échéance effective. LES DEUX conditions, pas l'une des
 *     deux : un appel dont la fenêtre est ouverte mais que l'équipe n'a pas
 *     encore fait passer à `open` n'accepte aucun dossier, et un appel resté à
 *     `open` après son échéance n'en accepte plus.
 *
 * Elles sont reproduites ici parce que la page publique doit rendre l'encart
 * SANS attendre un aller-retour, et surtout parce que le compte à rebours a
 * besoin de l'échéance en clair. Elles ne remplacent pas le contrôle : la
 * recevabilité d'un dépôt se tranche en base, jamais dans un navigateur.
 */

import type { CallForProposals } from '~/types/event/call'
import type { IsoDateTime } from '~/types/shared'

/**
 * Les trois états que l'encart d'appel doit rendre — ce sont ceux du prompt, et
 * ils ne se confondent pas avec le statut de la table : `draft`, `under_review`
 * et `published` disent où en est l'ÉQUIPE, `upcoming`/`open`/`closed` disent ce
 * qu'une organisation peut faire aujourd'hui.
 */
export type CallPhase = 'upcoming' | 'open' | 'closed'

/** `event.effective_deadline()` — prolongation comprise. */
export function effectiveDeadline(call: CallForProposals): IsoDateTime {
  return call.extended_until ?? call.closes_at
}

/** L'appel a-t-il été prolongé ? L'échéance d'origine reste alors affichable. */
export function wasExtended(call: CallForProposals): boolean {
  return call.extended_until !== null
}

/**
 * @param at instant d'évaluation — passé explicitement pour que le rendu serveur
 *           et le rendu client ne puissent pas diverger sur un cas limite.
 */
export function callPhase(call: CallForProposals, at: number = Date.now()): CallPhase {
  // Un appel annulé n'est pas « à venir » : il ne s'ouvrira pas. Il se range
  // avec ce qui est clos, et l'écran en donne le motif.
  if (call.status === 'cancelled') return 'closed'

  const opens = Date.parse(call.opens_at)
  const deadline = Date.parse(effectiveDeadline(call))

  if (Number.isFinite(opens) && at < opens) return 'upcoming'
  if (Number.isFinite(deadline) && at > deadline) return 'closed'

  // La fenêtre est ouverte : reste le statut, qui seul dit si l'équipe a
  // effectivement lancé la campagne. C'est le ET de `is_call_open()`.
  return call.status === 'open' ? 'open' : 'closed'
}

/** Le dépôt est-il possible maintenant ? — `event.is_call_open()`. */
export function isCallOpen(call: CallForProposals, at: number = Date.now()): boolean {
  return callPhase(call, at) === 'open'
}
