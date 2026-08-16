/**
 * Données simulées de `event.review_criteria` — la grille d'évaluation.
 *
 * Six critères, repris à l'identique de `event.seed_default_criteria()` : codes,
 * libellés, notes maximales, pondérations. Les mocks doivent rester
 * substituables par la base réelle sans qu'un écran change.
 *
 * LE CRITÈRE ÉLIMINATOIRE est `relevance` : une note de zéro sur la pertinence
 * disqualifie le dossier quelle que soit la moyenne générale
 * (`Proposal.is_knocked_out`). La fiche d'évaluation (A8) doit le dire AVANT que
 * le membre du comité pose la note, pas après.
 *
 * NOTE MAXIMALE ATTEIGNABLE : somme de `max_score × weight`, soit
 * 5 × (2 + 1,5 + 1,5 + 1 + 1 + 1) = 40. C'est ce que calcule
 * `event.max_weighted_score()`, et ce qui permet de ramener une note à l'échelle
 * sur 20 familière aux équipes : `score_out_of_20 = weighted_score × 20 / 40`.
 * Aucune constante d'interface ne doit refaire ce calcul à la main.
 */

import type { ReviewCriterion } from '~/types/event/call'
import { CALL, CRITERION } from './ids'

export const reviewCriteria = [
  {
    id: CRITERION.relevance,
    call_id: CALL.cop31,
    code: 'relevance',
    label: {
      fr: "Pertinence au regard des priorités de l'IFDD",
      en: 'Relevance to IFDD priorities',
    },
    description: {
      fr: "L'activité sert-elle les priorités de la Francophonie sur le climat : adaptation, accès aux financements, transition juste, renforcement des capacités des délégations ?",
    },
    max_score: 5,
    weight: 2.0,
    // ÉLIMINATOIRE : une note nulle disqualifie, quelle que soit la moyenne.
    is_knockout: true,
    sort_order: 10,
  },
  {
    id: CRITERION.quality,
    call_id: CALL.cop31,
    code: 'quality',
    label: {
      fr: 'Qualité et clarté de la proposition',
      en: 'Quality and clarity of the proposal',
    },
    description: {
      fr: "Objectifs formulés, déroulé lisible, intervenants identifiés, articulation avec l'agenda de la conférence.",
    },
    max_score: 5,
    weight: 1.5,
    is_knockout: false,
    sort_order: 20,
  },
  {
    id: CRITERION.impact,
    call_id: CALL.cop31,
    code: 'impact',
    label: { fr: 'Impact et retombées attendues', en: 'Expected impact and outcomes' },
    description: {
      fr: "Ce que l'activité laisse derrière elle : engagement, publication, partenariat, suite donnée après la conférence.",
    },
    max_score: 5,
    weight: 1.5,
    is_knockout: false,
    sort_order: 30,
  },
  {
    id: CRITERION.innovation,
    call_id: CALL.cop31,
    code: 'innovation',
    label: { fr: 'Caractère innovant', en: 'Innovation' },
    description: {
      fr: "Approche, format ou solution technique qui n'a pas déjà été présentée lors des éditions précédentes.",
    },
    max_score: 5,
    weight: 1.0,
    is_knockout: false,
    sort_order: 40,
  },
  {
    id: CRITERION.inclusiveness,
    call_id: CALL.cop31,
    code: 'inclusiveness',
    label: {
      fr: 'Inclusion (genre, jeunesse, sociétés civiles)',
      en: 'Inclusiveness',
    },
    description: {
      fr: "Place faite aux femmes, aux jeunes délégations, aux organisations de terrain et aux peuples autochtones, dans le panel comme dans le sujet.",
    },
    max_score: 5,
    weight: 1.0,
    is_knockout: false,
    sort_order: 50,
  },
  {
    id: CRITERION.feasibility,
    call_id: CALL.cop31,
    code: 'feasibility',
    label: { fr: 'Faisabilité logistique', en: 'Logistical feasibility' },
    description: {
      fr: "Compatibilité avec les moyens du pavillon : durée, salle, interprétation, présence effective des intervenants annoncés, accès à la zone bleue.",
    },
    max_score: 5,
    weight: 1.0,
    is_knockout: false,
    sort_order: 60,
  },
] satisfies ReviewCriterion[]

/**
 * Note maximale atteignable sur cet appel — équivalent de
 * `event.max_weighted_score(call_id)`. Calculée, jamais écrite en dur : changer
 * une pondération ci-dessus doit suffire.
 */
export const maxWeightedScore = reviewCriteria.reduce(
  (total, criterion) => total + criterion.max_score * criterion.weight,
  0,
)
