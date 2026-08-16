/**
 * Schéma `programme`, partie 2 — évaluation des propositions.
 * Dérivé de `docs/database/070_programme_proposals.sql` § 5.
 *
 * La grille de critères, elle, appartient à l'appel : voir `ReviewCriterion`
 * dans `types/event/call.ts`. La note d'une revue est la somme pondérée des
 * notes par critère ; `refresh_proposal_score()` la consolide sur la proposition.
 */

import type {
  CriterionId,
  IsoDateTime,
  Numeric,
  PersonId,
  ProposalId,
  ReviewId,
  Uuid,
} from '../shared'

/**
 * Table `programme.review_assignments` — `070` § 5.
 * Répartition de la charge : qui doit évaluer quoi, pour quand.
 */
export interface ReviewAssignment {
  id: Uuid
  proposal_id: ProposalId
  reviewer_id: PersonId
  assigned_by: PersonId | null
  assigned_at: IsoDateTime
  due_at: IsoDateTime | null
  /** Déport volontaire : le révisionniste déclare un lien avec l'organisation
   *  porteuse et se retire. Traçabilité de l'impartialité du comité. */
  recused_at: IsoDateTime | null
  recusal_reason: string | null
}

/** Contrainte `ck` sur `programme.reviews.recommendation`. */
export type ReviewRecommendation = 'accept' | 'accept_with_changes' | 'neutral' | 'reject'

/**
 * Table `programme.reviews` — `070` § 5.
 * L'avis complet d'un membre du comité sur une proposition. Tant que
 * `submitted_at` est nul, la revue est un brouillon et ne compte pas dans les
 * agrégats de la proposition.
 */
export interface Review {
  id: ReviewId
  proposal_id: ProposalId
  reviewer_id: PersonId
  recommendation: ReviewRecommendation
  /** Calculée depuis les notes par critère. */
  weighted_score: Numeric | null
  /** La même note ramenée sur 20, échelle familière aux équipes de la v1. */
  score_out_of_20: Numeric | null
  strengths: string | null
  weaknesses: string | null
  /** Visible du seul comité, JAMAIS du soumissionnaire. */
  private_note: string | null
  submitted_at: IsoDateTime | null
  created_at: IsoDateTime
  updated_at: IsoDateTime
}

/**
 * Table `programme.review_scores` — `070` § 5.
 * C'est ici que se justifie une décision contestée. Le plafond dépend du critère
 * (`ReviewCriterion.max_score`) et non d'une constante d'interface.
 */
export interface ReviewScore {
  review_id: ReviewId
  criterion_id: CriterionId
  score: Numeric
  comment: string | null
}
