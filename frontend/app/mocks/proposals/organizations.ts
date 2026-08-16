/**
 * Données simulées de `programme.proposal_organizations` — la co-organisation.
 *
 * PLUSIEURS ORGANISATIONS PEUVENT PORTER UNE ACTIVITÉ : un porteur principal
 * (`lead`), des co-organisateurs, des partenaires, des soutiens. Sans cette
 * table, les co-organisateurs restaient dans le texte de présentation —
 * invisibles des filtres, des statistiques et du décompte par organisation. La
 * v1 en est morte : impossible de dire combien d'activités une organisation
 * avait réellement portées.
 *
 * LA LIGNE `lead` EST TENUE EN COHÉRENCE AVEC `proposals.organization_id` par un
 * trigger : elle n'est jamais saisie à part. Elle est donc DÉRIVÉE ici des
 * quarante dossiers, et non recopiée à la main — recopier serait s'exposer à une
 * divergence que la base, elle, ne connaît pas.
 *
 * Cinq dossiers sont co-organisés. L'un d'eux porte une co-organisation NON
 * CONFIRMÉE : annoncer un partenaire engage un tiers, et le back-office doit
 * l'afficher « en attente » tant que l'intéressé n'a pas confirmé.
 */

import type { ProposalOrganization } from '~/types/programme/proposal'
import { ORG, PERSON, PROPOSAL } from '../ids'
import { acceptedProposals } from './accepted'
import { draftProposals } from './drafts'
import { reviewedProposals } from './reviewed'
import { submittedProposals } from './submitted'

/** Ligne de porteur principal, telle que la pose le trigger. */
const leadRows: ProposalOrganization[] = [
  ...draftProposals,
  ...submittedProposals,
  ...reviewedProposals,
  ...acceptedProposals,
].map((p) => ({
  proposal_id: p.id,
  organization_id: p.organization_id,
  role: 'lead',
  confirmed_at: p.submitted_at ?? p.created_at,
  sort_order: 0,
  added_by: p.submitted_by,
  added_at: p.created_at,
}))

/** Co-organisateurs, partenaires et soutiens, déclarés par le porteur. */
const partnerRows = [
  // 1. Adaptation côtière — un co-organisateur institutionnel, un partenaire
  //    scientifique.
  {
    proposal_id: PROPOSAL.adaptationCotiere,
    organization_id: ORG.anteb,
    role: 'co_organizer',
    confirmed_at: '2026-06-08T10:00:00Z',
    sort_order: 10,
    added_by: PERSON.sowFall,
    added_at: '2026-06-04T09:00:00Z',
  },
  {
    proposal_id: PROPOSAL.adaptationCotiere,
    organization_id: ORG.imre,
    role: 'partner',
    confirmed_at: '2026-06-09T14:30:00Z',
    sort_order: 20,
    added_by: PERSON.sowFall,
    added_at: '2026-06-04T09:05:00Z',
  },

  // 3. Mini-réseaux solaires — un partenaire privé apporte les données
  //    d'exploitation.
  {
    proposal_id: PROPOSAL.miniReseaux,
    organization_id: ORG.verdeo,
    role: 'partner',
    confirmed_at: '2026-06-13T08:20:00Z',
    sort_order: 10,
    added_by: PERSON.kabore,
    added_at: '2026-06-06T11:15:00Z',
  },

  // 5. Pertes et préjudices — quatre organisations, la configuration la plus
  //    riche du jeu : deux co-organisateurs, un partenaire, un soutien.
  {
    proposal_id: PROPOSAL.pertesPrejudices,
    organization_id: ORG.roac,
    role: 'co_organizer',
    confirmed_at: '2026-06-16T09:40:00Z',
    sort_order: 10,
    added_by: PERSON.josephPierre,
    added_at: '2026-06-10T15:20:00Z',
  },
  {
    proposal_id: PROPOSAL.pertesPrejudices,
    organization_id: ORG.cofemac,
    role: 'co_organizer',
    confirmed_at: '2026-06-17T11:05:00Z',
    sort_order: 20,
    added_by: PERSON.josephPierre,
    added_at: '2026-06-10T15:25:00Z',
  },
  {
    proposal_id: PROPOSAL.pertesPrejudices,
    organization_id: ORG.cudcm,
    role: 'partner',
    confirmed_at: '2026-06-18T08:00:00Z',
    sort_order: 30,
    added_by: PERSON.josephPierre,
    added_at: '2026-06-10T15:30:00Z',
  },
  {
    proposal_id: PROPOSAL.pertesPrejudices,
    organization_id: ORG.ifdd,
    role: 'sponsor',
    confirmed_at: '2026-06-18T12:00:00Z',
    sort_order: 40,
    added_by: PERSON.duchesne,
    added_at: '2026-06-12T09:00:00Z',
  },

  // 6. Transition juste — co-organisation jeunesse.
  {
    proposal_id: PROPOSAL.transitionJuste,
    organization_id: ORG.ujfc,
    role: 'co_organizer',
    confirmed_at: '2026-06-19T16:45:00Z',
    sort_order: 10,
    added_by: PERSON.ngoBassong,
    added_at: '2026-06-13T10:10:00Z',
  },

  // 11. Mangroves — un co-organisateur confirmé, un soutien QUI NE L'A PAS
  //     ENCORE CONFIRMÉ : le back-office doit l'afficher « en attente ».
  {
    proposal_id: PROPOSAL.mangroves,
    organization_id: ORG.roac,
    role: 'co_organizer',
    confirmed_at: '2026-06-29T09:15:00Z',
    sort_order: 10,
    added_by: PERSON.ngoBassong,
    added_at: '2026-06-24T14:00:00Z',
  },
  {
    proposal_id: PROPOSAL.mangroves,
    organization_id: ORG.imre,
    role: 'sponsor',
    confirmed_at: null,
    sort_order: 20,
    added_by: PERSON.ngoBassong,
    added_at: '2026-06-24T14:05:00Z',
  },

  // 14. Accès au Fonds vert — deux co-organisateurs de terrain aux côtés de
  //     l'IFDD.
  {
    proposal_id: PROPOSAL.accesFondsVert,
    organization_id: ORG.fhrc,
    role: 'co_organizer',
    confirmed_at: '2026-07-01T13:20:00Z',
    sort_order: 10,
    added_by: PERSON.duchesne,
    added_at: '2026-06-28T09:30:00Z',
  },
  {
    proposal_id: PROPOSAL.accesFondsVert,
    organization_id: ORG.anteb,
    role: 'co_organizer',
    confirmed_at: '2026-07-02T08:45:00Z',
    sort_order: 20,
    added_by: PERSON.duchesne,
    added_at: '2026-06-28T09:35:00Z',
  },
] satisfies ProposalOrganization[]

export const proposalOrganizations: ProposalOrganization[] = [...leadRows, ...partnerRows]
