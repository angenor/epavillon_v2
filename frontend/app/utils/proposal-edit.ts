/**
 * QUI PEUT MODIFIER SON DOSSIER, ET JUSQU'À QUAND.
 *
 * LA RÈGLE VIENT DU COMMANDITAIRE, arrêtée le 17/08 : « tant que l'événement
 * n'est pas terminé, il peut modifier ». Ce n'est PAS l'état du dossier qui
 * ferme la porte, c'est la fin de l'ÉDITION — et cela change tout par rapport à
 * ce qu'on aurait déduit de la machine à états.
 *
 * DEUX CONSÉQUENCES QU'IL FAUT ASSUMER, et que l'écran dit plutôt que de les
 * taire :
 *
 *  1. UN DOSSIER DÉJÀ DÉPOSÉ RESTE MODIFIABLE. Le comité peut donc lire une
 *     version, puis une autre. Ce n'est tenable que parce que l'historique
 *     existe : `programme.proposal_history()` rend chaque modification, champ
 *     par champ, avec son auteur et sa date — le comité voit ce qui a bougé
 *     depuis sa lecture. Sans cette traçabilité, la règle serait dangereuse.
 *
 *  2. UN DOSSIER RETENU RESTE MODIFIABLE, mais **la séance programmée ne bouge
 *     pas avec lui**. Le modèle sépare volontairement la PROPOSITION (le
 *     dossier déposé, pièce contractuelle) de la SESSION (l'occurrence
 *     programmée, avec son créneau, sa salle, ses inscrits et ses rappels) —
 *     c'est la décision structurante n° 1 de `070_programme_proposals.sql`.
 *     Corriger son dossier ne réécrit donc PAS le programme publié ; l'écran
 *     l'annonce, faute de quoi une organisation croirait avoir changé l'heure
 *     de son activité en corrigeant une faute de frappe.
 *
 * UN DOSSIER CLOS N'EST PAS MODIFIABLE, et ce n'est pas une restriction ajoutée
 * à la règle : un dossier refusé, retiré ou annulé n'est plus en course, aucune
 * transition ne l'y ramène à l'initiative de l'organisation
 * (`proposal_transitions_allowed`), et le modifier n'aurait aucun effet.
 * Reprendre un dossier refusé, c'est en déposer un nouveau.
 */

import type { EventEdition } from '~/types/event/edition'
import type { Proposal, ProposalStatus } from '~/types/programme/proposal'

/** Pourquoi un dossier ne se modifie pas. `null` quand il se modifie. */
export type EditBlockedReason =
  /** L'édition est terminée : plus rien ne bouge, même le brouillon. */
  | 'edition_over'
  /** Le dossier est refusé, retiré ou annulé : il n'est plus en course. */
  | 'file_closed'

/** États qui ferment définitivement un dossier, du point de vue du déposant. */
const CLOSED_STATUSES: ProposalStatus[] = ['rejected', 'withdrawn', 'cancelled']

/**
 * L'édition est-elle terminée ?
 *
 * DEUX SIGNAUX, ET IL EN SUFFIT D'UN. Le statut `completed` est posé par
 * l'équipe et fait foi ; la date de fin prend le relais quand personne n'a
 * encore cliqué — une COP finie le vendredi n'attend pas le lundi pour l'être.
 * `archived` n'existe pas dans l'ENUM : les états sont `draft`, `announced`,
 * `ongoing`, `completed`, `cancelled`, `suspended`.
 */
export function isEditionOver(edition: EventEdition, at: number = Date.now()): boolean {
  if (edition.status === 'completed' || edition.status === 'cancelled') return true
  return Date.parse(edition.ends_at) < at
}

/**
 * Ce dossier peut-il être modifié par son organisation, maintenant ?
 *
 * Rend `null` quand oui, et le MOTIF quand non — l'écran doit pouvoir dire
 * pourquoi le bouton n'est pas là. Un bouton absent sans explication est une
 * porte fermée sans écriteau, et c'est ce qui produit les courriels à l'IFDD.
 */
export function proposalEditBlockedReason(
  proposal: Proposal,
  edition: EventEdition,
  at: number = Date.now(),
): EditBlockedReason | null {
  if (isEditionOver(edition, at)) return 'edition_over'
  if (CLOSED_STATUSES.includes(proposal.status)) return 'file_closed'
  return null
}

/** Raccourci de lecture pour les gabarits. */
export function canEditProposal(
  proposal: Proposal,
  edition: EventEdition,
  at: number = Date.now(),
): boolean {
  return proposalEditBlockedReason(proposal, edition, at) === null
}

/**
 * CE QUE LA MODIFICATION DÉCLENCHE À L'ENVOI, selon l'état de départ.
 *
 * · `draft`             → dépôt normal (`draft → submitted`), soumis à la
 *                         fenêtre de l'appel : c'est un PREMIER dépôt.
 * · `changes_requested` → RENVOI au comité (`changes_requested → submitted`).
 *                         Le SQL a été corrigé le 17/08 pour l'autoriser après
 *                         l'échéance : le comité demande ses corrections APRÈS
 *                         la clôture, et le refus laissait le dossier bloqué.
 * · tout autre état     → enregistrement SANS changement d'état. Aucune
 *                         transition n'existe vers soi-même, et le dossier n'a
 *                         pas à repartir au comité pour une correction de forme.
 */
export type EditOutcome = 'submit' | 'resubmit' | 'save_only'

export function editOutcomeOf(status: ProposalStatus): EditOutcome {
  if (status === 'draft') return 'submit'
  if (status === 'changes_requested') return 'resubmit'
  return 'save_only'
}
