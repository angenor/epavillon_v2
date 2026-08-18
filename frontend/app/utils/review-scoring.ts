/**
 * NOTER UNE PROPOSITION — le calcul de la fiche d'évaluation (A8), en fonctions
 * pures.
 *
 * CE FICHIER REJOUE `programme.refresh_proposal_score()`, ET RIEN D'AUTRE. La
 * base calcule la note pondérée d'une revue comme la somme des `score × weight`,
 * la ramène sur 20 par `event.max_weighted_score()`, et lève `is_knocked_out`
 * dès qu'un critère éliminatoire reçoit zéro. L'écran doit afficher la même
 * chose EN DIRECT, avant l'enregistrement : sans cela, un membre du comité pose
 * six notes sans savoir ce qu'elles donnent, et découvre son total après coup.
 *
 * AUCUNE CONSTANTE DE GRILLE ICI. Le maximum sur 20, les poids, les notes
 * maximales et le caractère éliminatoire appartiennent à l'appel
 * (`event.review_criteria`) et varient d'un appel à l'autre : les figer dans un
 * utilitaire serait refaire, à un étage de plus, l'erreur des thématiques
 * codées en dur de la v1.
 *
 * FONCTIONS PURES : ni réseau, ni traduction, ni fuseau. Ce qui demande une
 * locale — le libellé d'un critère, celui d'une recommandation — est résolu par
 * l'appelant.
 */

import type { ReviewCriterion } from '~/types/event/call'
import type { CommitteeMemberProgress, ReviewProgressState } from '~/types/admin-review'
import type { ProposalStatus, ProposalTransitionRule } from '~/types/programme/proposal'
import type { Review, ReviewAssignment, ReviewRecommendation } from '~/types/programme/review'
import type { EffectivePermission } from '~/types/identity'
import type { Intent } from '~/types/ui'
import type { CriterionId, Numeric, Uuid } from '~/types/shared'

// ---------------------------------------------------------------------------
// La grille
// ---------------------------------------------------------------------------

/**
 * Note maximale atteignable sur cette grille — `event.max_weighted_score()`.
 * Somme des `max_score × weight`, jamais une constante : la grille de la COP31
 * plafonne à 40, celle d'un autre appel plafonnera ailleurs.
 */
export function maxWeightedScoreOfCriteria(criteria: ReviewCriterion[]): number {
  return criteria.reduce((total, criterion) => total + criterion.max_score * criterion.weight, 0)
}

/**
 * TOTAL PONDÉRÉ DES NOTES POSÉES.
 *
 * UN CRITÈRE NON NOTÉ NE COMPTE PAS COMME UN ZÉRO, et la nuance est le cœur du
 * calcul : zéro sur le critère éliminatoire disqualifie le dossier. Un total
 * calculé sur des cases vides afficherait donc, dès l'ouverture de la grille,
 * une proposition « éliminée » que personne n'a encore lue.
 */
export function weightedTotal(
  scores: Record<CriterionId, Numeric>,
  criteria: ReviewCriterion[],
): number {
  return criteria.reduce((total, criterion) => {
    const score = scores[criterion.id]
    return score === undefined || score === null ? total : total + score * criterion.weight
  }, 0)
}

/**
 * La même note ramenée sur 20 — l'échelle des équipes de la v1, conservée
 * exprès. Nulle quand la grille est vide : diviser par zéro n'affiche rien de
 * bon, et un appel sans critère n'a pas de note à donner.
 */
export function scoreOutOfTwenty(weighted: number, maxWeighted: number): number | null {
  if (maxWeighted <= 0) return null
  return Math.round((weighted / maxWeighted) * 20 * 100) / 100
}

/**
 * LES CRITÈRES ÉLIMINATOIRES NOTÉS ZÉRO. Rend les critères concernés, et non un
 * booléen : l'écran doit pouvoir NOMMER celui qui disqualifie — « Pertinence :
 * 0 » — plutôt qu'afficher un avertissement dont on cherche la cause.
 */
export function knockoutBreaches(
  scores: Record<CriterionId, Numeric>,
  criteria: ReviewCriterion[],
): ReviewCriterion[] {
  return criteria.filter((criterion) => criterion.is_knockout && scores[criterion.id] === 0)
}

/** La grille est-elle complète ? Une revue ne se dépose pas à moitié notée. */
export function missingScores(
  scores: Record<CriterionId, Numeric>,
  criteria: ReviewCriterion[],
): ReviewCriterion[] {
  return criteria.filter((criterion) => {
    const score = scores[criterion.id]
    return score === undefined || score === null
  })
}

/**
 * Les notes proposées pour un critère : de 0 à `max_score`, par pas d'un point.
 *
 * `max_score` est un `numeric(5,2)` et pourrait valoir 7,5 ; la grille réelle de
 * l'IFDD note sur cinq points entiers. On engendre donc les entiers jusqu'au
 * plafond — un demi-point ne se saisit pas au clavier dans un panneau qu'on
 * remplit à la souris, et la base accepterait la valeur si elle venait d'ailleurs.
 */
export function scoreChoices(criterion: ReviewCriterion): number[] {
  return Array.from({ length: Math.floor(criterion.max_score) + 1 }, (_, index) => index)
}

// ---------------------------------------------------------------------------
// L'avancement du comité
// ---------------------------------------------------------------------------

/**
 * OÙ EN EST UNE AFFECTATION, en un seul calcul pour tout l'écran.
 *
 * L'ORDRE DES TESTS PORTE UNE RÈGLE : un déport l'emporte sur tout le reste —
 * une personne qui s'est retirée n'est ni en retard ni attendue —, et une revue
 * COMMENCÉE mais non soumise reste une revue manquante. C'est ce que fait la
 * vue `v_proposal_dashboard`, qui ne compte que `submitted_at IS NOT NULL`.
 */
export function progressState(
  assignment: ReviewAssignment,
  review: Review | null,
  now: number,
): ReviewProgressState {
  if (assignment.recused_at) return 'recused'
  if (review?.submitted_at) return 'submitted'
  if (assignment.due_at && Date.parse(assignment.due_at) < now) return 'overdue'
  if (review) return 'drafted'
  return 'pending'
}

/** Revues effectivement rendues, déports exclus — le numérateur du « 2/3 ». */
export function submittedCount(committee: CommitteeMemberProgress[]): number {
  return committee.filter((entry) => entry.state === 'submitted').length
}

/**
 * COMBIEN DE REVUES SONT ENCORE ATTENDUES.
 *
 * Le dénominateur est `required_reviews` — la règle de l'appel —, et non le
 * nombre de personnes affectées : confier un dossier à cinq membres quand deux
 * revues suffisent ne rend pas trois revues manquantes. À défaut d'appel, on
 * retombe sur les affectations non déportées, seule référence disponible.
 */
export function reviewsMissing(
  committee: CommitteeMemberProgress[],
  requiredReviews: number | null,
): number {
  const expected =
    requiredReviews ?? committee.filter((entry) => entry.state !== 'recused').length
  return Math.max(expected - submittedCount(committee), 0)
}

// ---------------------------------------------------------------------------
// Couleurs d'état
// ---------------------------------------------------------------------------

/**
 * L'INTENTION D'UNE RECOMMANDATION — la règle de couleur de la charte, appliquée
 * à un avis de comité et non à un état temporel.
 *
 * « Retenir avec modifications » est un AVERTISSEMENT et non un succès : le
 * dossier ne passe pas en l'état. « Neutre » est gris, parce qu'il ne tranche
 * rien — l'afficher en bleu le ferait lire comme un avis favorable.
 */
export function recommendationIntent(recommendation: ReviewRecommendation): Intent {
  const intents: Record<ReviewRecommendation, Intent> = {
    accept: 'success',
    accept_with_changes: 'warning',
    neutral: 'neutral',
    reject: 'danger',
  }
  return intents[recommendation]
}

/**
 * L'intention d'un état d'avancement. Le retard est un DANGER et non un
 * avertissement : c'est la seule ligne de la file du comité sur laquelle
 * quelqu'un doit agir aujourd'hui.
 */
export function progressIntent(state: ReviewProgressState): Intent {
  const intents: Record<ReviewProgressState, Intent> = {
    submitted: 'success',
    drafted: 'warning',
    pending: 'info',
    overdue: 'danger',
    recused: 'neutral',
  }
  return intents[state]
}

// ---------------------------------------------------------------------------
// Les décisions offertes
// ---------------------------------------------------------------------------

/** Une décision proposée par l'en-tête, dérivée de la machine à états. */
export interface DecisionOption {
  to_status: ProposalStatus
  requires_reason: boolean
}

/**
 * LES DÉCISIONS POSSIBLES SUR CE DOSSIER, lues dans
 * `programme.proposal_transitions_allowed` et filtrées par ce que la personne a
 * le droit de faire, SUR CETTE ÉDITION.
 *
 * TROIS CHOSES SONT ÉCARTÉES, et chacune pour une raison différente :
 *  · les transitions réservées au SOUMISSIONNAIRE (`allowed_for_owner` sans
 *    permission requise) — retirer un dossier n'appartient pas au comité ;
 *  · celles dont la permission manque à cette personne sur cette édition — c'est
 *    la règle métier n° 8 prise par l'autre bout ;
 *  · aucune autre : si la base ouvre un chemin, l'écran l'offre. Ajouter une
 *    ligne en base ajoute une action, sans toucher à ce fichier.
 */
export function decisionOptions(
  status: ProposalStatus,
  rules: ProposalTransitionRule[],
  granted: EffectivePermission[] | null | undefined,
  eventId: Uuid | null,
): DecisionOption[] {
  return rules
    .filter((rule) => rule.from_status === status)
    .filter((rule) => rule.required_permission !== null)
    .filter((rule) => hasPermission(granted, rule.required_permission!, eventId))
    .map((rule) => ({ to_status: rule.to_status, requires_reason: rule.requires_reason }))
}
