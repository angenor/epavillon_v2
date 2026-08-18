/**
 * `programme.proposal_transitions_allowed` — la machine à états des dossiers,
 * recopiée fidèlement de `docs/database/070_programme_proposals.sql` § 1.
 *
 * ELLE EST UNE DONNÉE, PAS DU CODE, et c'est tout l'intérêt : le back-office ne
 * décide pas lui-même qu'un dossier déposé peut passer en évaluation, il le LIT.
 * Ajouter un chemin dans la base ajoute une action dans l'écran, sans qu'aucune
 * condition Vue soit touchée. La v1 tenait ce graphe dans ses contrôleurs, ce
 * qui laissait passer des transitions impossibles dès qu'un écran oubliait une
 * vérification.
 *
 * TROIS COLONNES QUI COMMANDENT L'INTERFACE :
 *   · `required_permission` — l'action ne s'affiche que si la personne l'a ;
 *   · `allowed_for_owner`   — le soumissionnaire peut la déclencher lui-même ;
 *   · `requires_reason`     — le trigger REFUSE la transition sans motif, donc
 *                             le dialogue exige le champ avant d'envoyer.
 */

import type { ProposalTransitionRule } from '~/types/programme/proposal'

export const proposalTransitionsAllowed: ProposalTransitionRule[] = [
  { from_status: 'draft', to_status: 'submitted', required_permission: 'programme.proposal.submit', allowed_for_owner: true, requires_reason: false },
  { from_status: 'draft', to_status: 'withdrawn', required_permission: null, allowed_for_owner: true, requires_reason: false },
  { from_status: 'submitted', to_status: 'under_review', required_permission: 'programme.proposal.decide', allowed_for_owner: false, requires_reason: false },
  { from_status: 'submitted', to_status: 'changes_requested', required_permission: 'programme.review.write', allowed_for_owner: false, requires_reason: true },
  { from_status: 'submitted', to_status: 'withdrawn', required_permission: null, allowed_for_owner: true, requires_reason: true },
  { from_status: 'under_review', to_status: 'changes_requested', required_permission: 'programme.review.write', allowed_for_owner: false, requires_reason: true },
  { from_status: 'under_review', to_status: 'accepted', required_permission: 'programme.proposal.decide', allowed_for_owner: false, requires_reason: false },
  { from_status: 'under_review', to_status: 'rejected', required_permission: 'programme.proposal.decide', allowed_for_owner: false, requires_reason: true },
  { from_status: 'under_review', to_status: 'withdrawn', required_permission: null, allowed_for_owner: true, requires_reason: true },
  { from_status: 'changes_requested', to_status: 'submitted', required_permission: 'programme.proposal.submit', allowed_for_owner: true, requires_reason: false },
  { from_status: 'changes_requested', to_status: 'withdrawn', required_permission: null, allowed_for_owner: true, requires_reason: false },
  { from_status: 'changes_requested', to_status: 'rejected', required_permission: 'programme.proposal.decide', allowed_for_owner: false, requires_reason: true },
  { from_status: 'accepted', to_status: 'cancelled', required_permission: 'programme.proposal.decide', allowed_for_owner: false, requires_reason: true },
  { from_status: 'rejected', to_status: 'under_review', required_permission: 'programme.proposal.decide', allowed_for_owner: false, requires_reason: true },
]
