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
import { pastEditionProposals } from './past-editions'
import { reviewedProposals } from './reviewed'
import { submittedProposals } from './submitted'

export { acceptedProposals } from './accepted'
export { draftProposals } from './drafts'
export { pastEditionProposals } from './past-editions'
export { reviewedProposals } from './reviewed'
export { submittedProposals } from './submitted'
export { proposalOrganizations } from './organizations'
export { proposalSpeakers } from './speakers'
export { proposalAssets, proposalDocuments } from './documents'
export { proposalComments, proposalTransitions } from './exchanges'

/**
 * Les quarante et un dossiers, dans l'ordre de leur numéro — les quarante de la
 * COP31, plus celui que le ROAC avait déposé pour la COP30.
 *
 * Le tri par `reference_code` place naturellement les éditions passées avant les
 * dossiers en cours : `COP30-…` précède `COP31-…`.
 */
export const allProposals: Proposal[] = [
  ...acceptedProposals,
  ...reviewedProposals,
  ...submittedProposals,
  ...draftProposals,
  ...pastEditionProposals,
].sort((a, b) => a.reference_code.localeCompare(b.reference_code))
