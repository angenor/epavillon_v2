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
 * Contrainte `ck_reviews_mode` : la grille pondérée, ou une note directe sur 20
 * quand le temps manque (arbitré le 16/09).
 */
export type ReviewMode = 'quick' | 'detailed'

/**
 * Table `programme.reviews` — `070` § 5.
 * L'avis d'un membre de l'équipe sur une proposition. Tant que `submitted_at`
 * est nul, la revue est un brouillon et ne compte pas dans les agrégats. Elle
 * n'est jamais un préalable à la décision.
 */
export interface Review {
  id: ReviewId
  proposal_id: ProposalId
  reviewer_id: PersonId
  mode: ReviewMode
  recommendation: ReviewRecommendation
  /** Calculée des critères en mode détaillé, déduite de la note sur 20 en mode rapide. */
  weighted_score: Numeric | null
  /** Calculée en mode détaillé, SAISIE en mode rapide. */
  score_out_of_20: Numeric | null
  /** Commentaire général, lu de l'équipe. */
  comment: string | null
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
