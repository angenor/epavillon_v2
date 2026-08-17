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
import { CALL, CRITERION, CRITERION_PAST } from './ids'

/**
 * `event.seed_default_criteria()` rejouée — la grille proposée à la création
 * d'un appel (`060_events.sql` § 6), libellés, notes et pondérations compris.
 *
 * Elle sert aux appels des ÉDITIONS PASSÉES, qui ont été menés avec la grille
 * par défaut : les recopier à la main aurait produit douze blocs identiques à
 * ceux de la COP31, et une divergence le jour où l'un d'eux serait retouché.
 * L'appel de la COP31, lui, garde ses descriptions propres — c'est le cas
 * qu'éprouve la fiche d'évaluation.
 */
export function seedDefaultCriteria(callId: string, callIndex: number): ReviewCriterion[] {
  const grid: Array<[string, { fr: string; en: string }, number, boolean]> = [
    ['relevance', { fr: "Pertinence au regard des priorités de l'IFDD", en: 'Relevance to IFDD priorities' }, 2.0, true],
    ['quality', { fr: 'Qualité et clarté de la proposition', en: 'Quality and clarity of the proposal' }, 1.5, false],
    ['impact', { fr: 'Impact et retombées attendues', en: 'Expected impact and outcomes' }, 1.5, false],
    ['innovation', { fr: 'Caractère innovant', en: 'Innovation' }, 1.0, false],
    ['inclusiveness', { fr: 'Inclusion (genre, jeunesse, sociétés civiles)', en: 'Inclusiveness' }, 1.0, false],
    ['feasibility', { fr: 'Faisabilité logistique', en: 'Logistical feasibility' }, 1.0, false],
  ]

  return grid.map(([code, label, weight, is_knockout], index) => ({
    id: CRITERION_PAST(callIndex, index + 1),
    call_id: callId,
    code,
    label,
    description: null,
    max_score: 5,
    weight,
    is_knockout,
    sort_order: (index + 1) * 10,
  }))
}

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

  // Appels des éditions passées : la grille par défaut, engendrée.
  ...seedDefaultCriteria(CALL.cop30, 1),
  ...seedDefaultCriteria(CALL.cop29, 2),
] satisfies ReviewCriterion[]

/**
 * Note maximale atteignable — équivalent de `event.max_weighted_score(call_id)`.
 * Calculée, jamais écrite en dur : changer une pondération ci-dessus doit
 * suffire.
 *
 * FILTRÉE PAR APPEL, et pas seulement pour la forme : depuis que les éditions
 * passées ont elles aussi leur grille, sommer `reviewCriteria` en entier
 * donnerait 120 au lieu de 40 et diviserait par trois toutes les notes ramenées
 * sur 20. La fonction SQL prend un `call_id` ; celle-ci aussi.
 */
export function maxWeightedScoreOf(callId: string): number {
  return reviewCriteria
    .filter((criterion) => criterion.call_id === callId)
    .reduce((total, criterion) => total + criterion.max_score * criterion.weight, 0)
}

/** Raccourci pour l'appel en cours (COP31) : 5 × (2 + 1,5 + 1,5 + 1 + 1 + 1) = 40. */
export const maxWeightedScore = maxWeightedScoreOf(CALL.cop31)
