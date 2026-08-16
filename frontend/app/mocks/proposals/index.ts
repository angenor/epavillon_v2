/**
 * Point d'entrée des propositions simulées. Ne contient AUCUNE donnée : il
 * ré-exporte et compose.
 *
 * Les écrans importent de préférence le fichier de statut dont ils ont besoin —
 * l'espace organisation lit les brouillons, la fiche d'évaluation lit les
 * dossiers en cours de revue. `allProposals` n'existe que pour les écrans qui
 * balaient réellement l'ensemble : le tableau de bord et la liste du back-office.
 */

import type { Proposal } from '~/types/programme/proposal'
import { acceptedProposals } from './accepted'
import { draftProposals } from './drafts'
import { reviewedProposals } from './reviewed'
import { submittedProposals } from './submitted'

export { acceptedProposals } from './accepted'
export { draftProposals } from './drafts'
export { reviewedProposals } from './reviewed'
export { submittedProposals } from './submitted'
export { proposalOrganizations } from './organizations'
export { proposalSpeakers } from './speakers'
export { proposalAssets, proposalDocuments } from './documents'
export { proposalComments, proposalTransitions } from './exchanges'

/** Les quarante dossiers, dans l'ordre de leur numéro de dossier. */
export const allProposals: Proposal[] = [
  ...acceptedProposals,
  ...reviewedProposals,
  ...submittedProposals,
  ...draftProposals,
].sort((a, b) => a.reference_code.localeCompare(b.reference_code))
