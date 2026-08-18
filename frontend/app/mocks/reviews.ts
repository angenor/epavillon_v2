/**
 * Données simulées de l'évaluation : `programme.review_assignments`,
 * `programme.reviews` et `programme.review_scores`.
 *
 * LES NOTES NE SONT PAS RECOPIÉES, ELLES SONT CALCULÉES. Chaque revue est écrite
 * comme six notes sur cinq — pertinence, qualité, impact, innovation, inclusion,
 * faisabilité — et la note pondérée en découle par la grille de
 * `mocks/criteria.ts`, exactement comme le fait `refresh_proposal_score()` en
 * base. Écrire à la main un `weighted_score` à côté de ses notes serait la
 * garantie d'une divergence silencieuse le jour où une pondération change.
 *
 * LA COHÉRENCE EST VÉRIFIÉE À L'EXÉCUTION : en fin de fichier, la moyenne des
 * revues soumises est comparée aux agrégats portés par la proposition. Une
 * divergence lève une erreur au chargement du module, pendant le développement,
 * plutôt que de produire un écran qui affiche deux notes différentes pour le
 * même dossier.
 *
 * PÉRIMÈTRE, assumé et documenté. Les revues détaillées couvrent TREIZE
 * dossiers : les cinq en cours d'évaluation, les trois en correction, les trois
 * écartés, et deux dossiers retenus donnés en référence. Les quatorze autres
 * dossiers retenus portent leurs agrégats sans que le détail de leurs
 * trente-neuf revues soit écrit — sept cents notes à la main que nul écran ne
 * consulte, la fiche d'évaluation (A8) travaillant sur les dossiers en cours.
 * L'écart est consigné dans `docs/PROGRESSION.md`.
 *
 * ÉVALUATION EN AVEUGLE : l'appel simulé pose `blind_review: true` depuis le
 * 18/08, donc un membre du comité ne voit les notes de ses pairs qu'APRÈS avoir
 * soumis la sienne — c'est ce que la fiche d'évaluation (A8) doit montrer, et ce
 * que le modèle prend par défaut. Deux revues du jeu restent délibérément non
 * soumises (`submitted_at: null`) : sans elles, le voile ne se verrait jamais.
 * Le déport (`recused_at`) est représenté : un révisionniste déclare un lien
 * avec l'organisation porteuse et se retire, ce qui se lit dans l'écran
 * d'affectation comme dans la fiche.
 */

import type { Review, ReviewAssignment, ReviewScore } from '~/types/programme/review'
import type { Numeric } from '~/types/shared'
import { CRITERION, PERSON, PROPOSAL, REVIEW, REVIEW_ASSIGNMENT } from './ids'
import { maxWeightedScore, reviewCriteria } from './criteria'
import { allProposals } from './proposals'

// ---------------------------------------------------------------------------
// Fabrique d'une revue et de ses notes
// ---------------------------------------------------------------------------

/** Notes sur cinq, dans l'ordre de la grille : pertinence, qualité, impact,
 *  innovation, inclusion, faisabilité. */
type Scores = [number, number, number, number, number, number]

const criterionOrder = [
  CRITERION.relevance,
  CRITERION.quality,
  CRITERION.impact,
  CRITERION.innovation,
  CRITERION.inclusiveness,
  CRITERION.feasibility,
] as const

const weightOf = new Map(reviewCriteria.map((c) => [c.id, c.weight]))

function weightedOf(scores: Scores): Numeric {
  return criterionOrder.reduce((total, id, index) => total + scores[index]! * weightOf.get(id)!, 0)
}

interface ReviewFields {
  scores: Scores
  recommendation: Review['recommendation']
  strengths?: string | null
  weaknesses?: string | null
  privateNote?: string | null
  /** Nul : la revue est un brouillon et ne compte dans aucun agrégat. */
  submittedAt: string | null
  createdAt: string
  /** Commentaires attachés à un critère précis. */
  comments?: Partial<Record<'relevance' | 'quality' | 'impact' | 'innovation' | 'inclusiveness' | 'feasibility', string>>
}

const allScores: ReviewScore[] = []

function review(n: number, proposal_id: string, reviewer_id: string, fields: ReviewFields): Review {
  const id = REVIEW(n)
  const weighted = weightedOf(fields.scores)
  const commentKeys = ['relevance', 'quality', 'impact', 'innovation', 'inclusiveness', 'feasibility'] as const

  criterionOrder.forEach((criterion_id, index) => {
    allScores.push({
      review_id: id,
      criterion_id,
      score: fields.scores[index]!,
      comment: fields.comments?.[commentKeys[index]!] ?? null,
    })
  })

  return {
    id,
    proposal_id,
    reviewer_id,
    recommendation: fields.recommendation,
    // Un brouillon ne porte pas encore de note consolidée.
    weighted_score: fields.submittedAt ? weighted : null,
    score_out_of_20: fields.submittedAt ? (weighted * 20) / maxWeightedScore : null,
    strengths: fields.strengths ?? null,
    weaknesses: fields.weaknesses ?? null,
    private_note: fields.privateNote ?? null,
    submitted_at: fields.submittedAt,
    created_at: fields.createdAt,
    updated_at: fields.submittedAt ?? fields.createdAt,
  }
}

// ---------------------------------------------------------------------------
// Les revues
// ---------------------------------------------------------------------------

export const reviews = [
  // --- COP31-00001 · Financer l'adaptation côtière (retenue) ---------------
  review(1, PROPOSAL.adaptationCotiere, PERSON.lemoine, {
    scores: [5, 5, 5, 4, 4, 4],
    recommendation: 'accept',
    strengths:
      "Le montage régional répond à un obstacle identifié depuis des années sans être traité. Les trois administrations sont autour de la table, ce qui est rare à ce stade.",
    weaknesses: "Le déroulé de 90 minutes est serré pour quatre intervenants et un temps d'échange.",
    submittedAt: '2026-07-14T10:20:00Z',
    createdAt: '2026-07-10T09:00:00Z',
    comments: {
      relevance: "Adaptation côtière et accès au financement : deux priorités de l'IFDD dans une seule session.",
      feasibility: "Prévoir un modérateur ferme sur le temps de parole.",
    },
  }),
  review(2, PROPOSAL.adaptationCotiere, PERSON.duchesne, {
    scores: [5, 5, 4, 5, 4, 4],
    recommendation: 'accept',
    strengths: "L'angle financier est traité avec des instruments réels, pas avec des intentions.",
    weaknesses: "Le livrable annoncé — une feuille de route — devra être écrit pendant la session, pas après.",
    privateNote: "À rapprocher de la session sur l'accès direct au Fonds vert : mêmes interlocuteurs.",
    submittedAt: '2026-07-16T08:45:00Z',
    createdAt: '2026-07-12T14:30:00Z',
  }),
  review(3, PROPOSAL.adaptationCotiere, PERSON.benAmor, {
    scores: [5, 4, 5, 5, 5, 4],
    recommendation: 'accept',
    strengths: "Panel équilibré entre société civile, administration et bailleur ; parité respectée.",
    weaknesses: "La dimension sédimentaire mériterait un appui scientifique explicite dans le panel.",
    submittedAt: '2026-07-18T11:00:00Z',
    createdAt: '2026-07-15T10:15:00Z',
  }),

  // --- COP31-00005 · Pertes et préjudices (retenue, mieux notée) -----------
  review(10, PROPOSAL.pertesPrejudices, PERSON.lemoine, {
    scores: [5, 5, 5, 4, 5, 4],
    recommendation: 'accept',
    strengths:
      "Sujet central de l'édition, porté par ceux qui déposent effectivement les dossiers. Le livrable est adressé au conseil du fonds avant la clôture.",
    weaknesses: "Rien de substantiel ; veiller à ce que le conseil du fonds soit réellement représenté.",
    submittedAt: '2026-07-20T09:30:00Z',
    createdAt: '2026-07-17T13:00:00Z',
  }),
  review(11, PROPOSAL.pertesPrejudices, PERSON.duchesne, {
    scores: [5, 5, 4, 5, 5, 4],
    recommendation: 'accept',
    strengths: "Part des dossiers réellement déposés : c'est ce qui manque à toutes les sessions sur ce sujet.",
    weaknesses: "La quantification des pertes non économiques mériterait un temps dédié.",
    submittedAt: '2026-07-21T15:10:00Z',
    createdAt: '2026-07-18T09:20:00Z',
  }),
  review(12, PROPOSAL.pertesPrejudices, PERSON.rasoanaivo, {
    scores: [5, 5, 4, 5, 5, 5],
    recommendation: 'accept',
    strengths: "Dossier complet, co-organisation réelle, public identifié. Rien à redire.",
    weaknesses: null,
    submittedAt: '2026-07-22T06:40:00Z',
    createdAt: '2026-07-19T07:00:00Z',
  }),

  // --- COP31-00020 · Budgets sensibles au genre (en évaluation) ------------
  review(20, PROPOSAL.budgetsGenre, PERSON.duchesne, {
    scores: [4, 4, 4, 3, 4, 3],
    recommendation: 'accept_with_changes',
    strengths: "Le constat organisationnel est juste et rarement formulé : le marqueur arrive après l'arbitrage.",
    weaknesses:
      "Les trois pays ne sont pas nommés et le calendrier budgétaire retenu n'est pas décrit. Le public visé — directions du budget — ne se déplacera pas pour un exposé général.",
    submittedAt: '2026-08-04T10:00:00Z',
    createdAt: '2026-07-30T11:20:00Z',
    comments: {
      impact: "Le guide annoncé existe-t-il déjà, ou reste-t-il à écrire ?",
      feasibility: "Une session de 90 minutes avec un seul intervenant confirmé : à consolider.",
    },
  }),
  review(21, PROPOSAL.budgetsGenre, PERSON.rasoanaivo, {
    scores: [4, 4, 4, 4, 4, 3],
    recommendation: 'accept',
    strengths: "Croisement genre et finances publiques, sujet peu représenté dans les éditions précédentes.",
    weaknesses: "Un deuxième intervenant, côté ministère des finances, rendrait la session bien plus crédible.",
    submittedAt: '2026-08-05T16:20:00Z',
    createdAt: '2026-08-02T08:00:00Z',
  }),

  // --- COP31-00021 · Cartographie participative (en évaluation) ------------
  review(30, PROPOSAL.cartographieCotonou, PERSON.benAmor, {
    scores: [3, 4, 4, 3, 4, 2],
    recommendation: 'neutral',
    strengths: "Le protocole de relevé communautaire est reproductible et documenté.",
    weaknesses:
      "L'intérêt pour des délégations en négociation reste indirect. La session tiendrait mieux dans un atelier technique que dans le programme principal.",
    submittedAt: '2026-08-06T09:00:00Z',
    createdAt: '2026-08-03T14:00:00Z',
  }),
  review(31, PROPOSAL.cartographieCotonou, PERSON.kabore, {
    scores: [3, 4, 4, 2, 4, 2],
    recommendation: 'accept_with_changes',
    strengths: "Correction effective du plan de prévention : le résultat est tangible.",
    weaknesses: "Un seul intervenant, aucun représentant des comités de quartier alors qu'ils ont fait le travail.",
    submittedAt: '2026-08-07T08:30:00Z',
    createdAt: '2026-08-04T10:45:00Z',
    comments: {
      inclusiveness: "Faire venir un membre d'un comité de quartier changerait la nature de la session.",
    },
  }),
  review(32, PROPOSAL.cartographieCotonou, PERSON.rasoanaivo, {
    scores: [3, 4, 4, 4, 4, 2],
    recommendation: 'accept',
    strengths: "Sujet parlant, illustrations disponibles, propos accessible à un public non spécialiste.",
    weaknesses: "La durée de 90 minutes semble longue pour le contenu annoncé.",
    submittedAt: '2026-08-07T12:15:00Z',
    createdAt: '2026-08-05T06:20:00Z',
  }),

  // --- COP31-00022 · Interprétation (une seule revue sur trois) ------------
  review(40, PROPOSAL.interpretation, PERSON.duchesne, {
    scores: [3, 3, 3, 2, 2, 3],
    recommendation: 'neutral',
    strengths: "Le relevé de terrain sur deux conférences donne une base factuelle au débat.",
    weaknesses:
      "Le dossier est porté par l'IFDD et n'annonce aucun intervenant extérieur confirmé. Sans le secrétariat de la CCNUCC autour de la table, la session tourne à l'entre-soi.",
    submittedAt: '2026-08-01T09:40:00Z',
    createdAt: '2026-07-28T15:00:00Z',
  }),
  review(41, PROPOSAL.interpretation, PERSON.lemoine, {
    // BROUILLON : commencé, jamais soumis. Ne compte dans aucun agrégat, et
    // c'est cette revue que le tableau de bord signale comme manquante.
    scores: [4, 3, 3, 2, 3, 3],
    recommendation: 'neutral',
    strengths: null,
    weaknesses: "À reprendre après avoir lu le relevé complet.",
    submittedAt: null,
    createdAt: '2026-08-08T18:20:00Z',
  }),

  // --- COP31-00023 · Reboisement urbain (en évaluation) --------------------
  review(50, PROPOSAL.reboisementUrbain, PERSON.benAmor, {
    scores: [3, 4, 4, 2, 3, 2],
    recommendation: 'accept_with_changes',
    strengths: "Trois ans de mesures instrumentées, ce qui est rare sur ce sujet.",
    weaknesses: "L'abaque promis n'est pas joint au dossier ; sans lui, la session reste descriptive.",
    submittedAt: '2026-08-05T04:30:00Z',
    createdAt: '2026-08-01T05:00:00Z',
  }),
  review(51, PROPOSAL.reboisementUrbain, PERSON.kabore, {
    scores: [3, 4, 4, 3, 3, 2],
    recommendation: 'accept',
    strengths: "Résultats nuancés, y compris là où la plantation n'a rien changé : c'est honnête et utile.",
    weaknesses: "Le lien avec les délégations francophones d'Afrique n'est pas fait, alors qu'il serait immédiat.",
    submittedAt: '2026-08-06T12:00:00Z',
    createdAt: '2026-08-02T09:15:00Z',
  }),

  // --- COP31-00024 · Assurance paramétrique (trois revues rendues) ---------
  review(60, PROPOSAL.assuranceParametrique, PERSON.duchesne, {
    scores: [4, 4, 4, 4, 4, 5],
    recommendation: 'accept',
    strengths:
      "Le dossier expose les non-déclenchements et les écarts de base, c'est-à-dire ce que les promoteurs de l'assurance indicielle taisent d'ordinaire.",
    weaknesses: "La méthode de contrôle de l'écart de base devra être présentée, pas seulement annoncée.",
    submittedAt: '2026-08-03T09:20:00Z',
    createdAt: '2026-07-29T10:00:00Z',
  }),
  review(61, PROPOSAL.assuranceParametrique, PERSON.benAmor, {
    scores: [4, 5, 4, 4, 3, 4],
    recommendation: 'accept',
    strengths: "Trois campagnes suivies, données de coopératives réelles, présentation claire.",
    weaknesses: "Aucune voix de producteur dans le panel annoncé.",
    submittedAt: '2026-08-04T14:40:00Z',
    createdAt: '2026-07-31T08:30:00Z',
    comments: {
      inclusiveness: "Inviter un responsable de coopérative ferait beaucoup pour la crédibilité de la session.",
    },
  }),
  review(62, PROPOSAL.assuranceParametrique, PERSON.rasoanaivo, {
    scores: [4, 4, 5, 4, 4, 4],
    recommendation: 'accept',
    strengths: "Sujet concret, transposable, avec un livrable utilisable par les régulateurs.",
    weaknesses: null,
    submittedAt: '2026-08-08T07:10:00Z',
    createdAt: '2026-08-05T09:00:00Z',
  }),

  // --- COP31-00025 · Numérique responsable (corrections demandées) ---------
  review(70, PROPOSAL.numeriqueResponsable, PERSON.kabore, {
    scores: [2, 3, 3, 2, 2, 3],
    recommendation: 'accept_with_changes',
    strengths: "Le constat sur la fabrication plutôt que sur l'usage est juste et pédagogique.",
    weaknesses: "Aucune administration nommée, aucune période indiquée : impossible d'apprécier la portée des résultats.",
    submittedAt: '2026-08-08T10:30:00Z',
    createdAt: '2026-08-04T13:00:00Z',
  }),
  review(71, PROPOSAL.numeriqueResponsable, PERSON.duchesne, {
    scores: [3, 3, 3, 2, 2, 3],
    recommendation: 'neutral',
    strengths: "Méthode d'inventaire réutilisable.",
    weaknesses: "Le lien avec les priorités climatiques de la Francophonie reste ténu ; le sujet vaudrait mieux en webinaire.",
    submittedAt: '2026-08-09T15:45:00Z',
    createdAt: '2026-08-06T09:30:00Z',
  }),

  // --- COP31-00026 · Déchets plastiques (corrections demandées) -----------
  review(80, PROPOSAL.dechetsPlastiques, PERSON.kabore, {
    scores: [2, 3, 2, 3, 3, 2],
    recommendation: 'accept_with_changes',
    strengths: "Comparaison de quatre filières sur douze sites : le travail de terrain est réel.",
    weaknesses:
      "Le bilan carbone annoncé n'est pas documenté dans le dossier, et l'annexe déposée est illisible. Cinq intervenants pour 90 minutes, c'est trop.",
    submittedAt: '2026-08-11T08:50:00Z',
    createdAt: '2026-08-07T14:20:00Z',
  }),

  // --- COP31-00027 · Écoles résilientes (corrections demandées) -----------
  review(90, PROPOSAL.ecolesResilientes, PERSON.benAmor, {
    scores: [3, 4, 4, 3, 4, 2],
    recommendation: 'accept_with_changes',
    strengths: "Mesures thermiques sérieuses, solutions passives peu coûteuses et transposables.",
    weaknesses: "L'effet annoncé sur la fréquentation scolaire n'est appuyé par aucune source.",
    submittedAt: '2026-08-10T11:20:00Z',
    createdAt: '2026-08-06T16:00:00Z',
    comments: {
      impact: "Retirer l'affirmation ou en donner la source : en l'état, elle affaiblit le reste du dossier.",
    },
  }),
  review(91, PROPOSAL.ecolesResilientes, PERSON.rasoanaivo, {
    scores: [3, 4, 4, 4, 4, 3],
    recommendation: 'accept',
    strengths: "Sujet parlant pour le grand public comme pour les ministères de l'éducation.",
    weaknesses: "Les coûts par établissement mériteraient d'être donnés en séance.",
    submittedAt: '2026-08-12T06:30:00Z',
    createdAt: '2026-08-08T09:40:00Z',
  }),

  // --- COP31-00017 · Captage direct (écartée, note éliminatoire) -----------
  review(100, PROPOSAL.captageAir, PERSON.lemoine, {
    // ZÉRO SUR LE CRITÈRE ÉLIMINATOIRE : la proposition est disqualifiée quelle
    // que soit la moyenne. L'écran doit le montrer, sans quoi la décision est
    // incompréhensible.
    scores: [0, 3, 2, 4, 1, 3],
    recommendation: 'reject',
    strengths: "Exposé technique correct sur l'état de l'art.",
    weaknesses:
      "Aucun territoire ni acteur francophone concerné, aucun intervenant du Sud, aucun apport pour des délégations en négociation.",
    privateNote: "Troisième dépôt de cette organisation, deux déjà écartés. Motivation écrite indispensable.",
    submittedAt: '2026-07-25T10:30:00Z',
    createdAt: '2026-07-22T09:00:00Z',
    comments: {
      relevance: "Hors périmètre du pavillon francophone.",
    },
  }),
  review(101, PROPOSAL.captageAir, PERSON.duchesne, {
    scores: [1, 3, 2, 4, 1, 3],
    recommendation: 'reject',
    strengths: "Le panorama technologique est à jour.",
    weaknesses: "Sujet éloigné des priorités et des moyens des pays francophones concernés.",
    submittedAt: '2026-07-25T13:00:00Z',
    createdAt: '2026-07-23T11:15:00Z',
  }),
  review(102, PROPOSAL.captageAir, PERSON.rasoanaivo, {
    scores: [0, 2, 2, 1, 1, 1],
    recommendation: 'reject',
    strengths: null,
    weaknesses: "Rien ne rattache cette proposition au programme du pavillon.",
    submittedAt: '2026-07-26T07:20:00Z',
    createdAt: '2026-07-24T08:00:00Z',
  }),

  // --- COP31-00018 · Crédits volontaires (écartée) -------------------------
  review(110, PROPOSAL.creditsVolontaires, PERSON.lemoine, {
    scores: [2, 3, 3, 2, 1, 2],
    recommendation: 'reject',
    strengths: "La plateforme fonctionne et le mécanisme de vérification est décrit.",
    weaknesses: "Démonstration d'un produit propriétaire ; aucune mise en perspective critique.",
    submittedAt: '2026-07-27T09:00:00Z',
    createdAt: '2026-07-24T14:00:00Z',
  }),
  review(111, PROPOSAL.creditsVolontaires, PERSON.duchesne, {
    scores: [2, 2, 3, 3, 1, 2],
    recommendation: 'reject',
    strengths: "Sujet d'actualité.",
    weaknesses: "Le pavillon n'est pas un espace de démonstration commerciale.",
    submittedAt: '2026-07-27T14:30:00Z',
    createdAt: '2026-07-25T10:00:00Z',
  }),
  review(112, PROPOSAL.creditsVolontaires, PERSON.benAmor, {
    scores: [2, 2, 2, 2, 2, 3],
    recommendation: 'reject',
    strengths: null,
    weaknesses: "Les critiques adressées aux marchés volontaires ne sont pas traitées.",
    submittedAt: '2026-07-28T08:15:00Z',
    createdAt: '2026-07-26T09:30:00Z',
  }),

  // --- COP31-00019 · Rétrospective (écartée) -------------------------------
  review(120, PROPOSAL.retrospectiveCop, PERSON.rasoanaivo, {
    scores: [2, 2, 2, 2, 2, 2],
    recommendation: 'reject',
    strengths: "Archives de qualité, montage soigné.",
    weaknesses: "Format rétrospectif sans prise sur les négociations en cours.",
    submittedAt: '2026-07-29T06:00:00Z',
    createdAt: '2026-07-27T07:30:00Z',
  }),
  review(121, PROPOSAL.retrospectiveCop, PERSON.kabore, {
    scores: [2, 3, 2, 1, 1, 2],
    recommendation: 'reject',
    strengths: null,
    weaknesses: "Le pavillon dispose de peu de créneaux ; celui-ci serait mieux employé ailleurs.",
    submittedAt: '2026-07-29T10:20:00Z',
    createdAt: '2026-07-27T13:00:00Z',
  }),
  review(122, PROPOSAL.retrospectiveCop, PERSON.duchesne, {
    scores: [2, 3, 2, 2, 1, 2],
    recommendation: 'reject',
    strengths: "Pourrait trouver sa place hors période de conférence.",
    weaknesses: "Aucun apport aux délégations pendant la conférence.",
    submittedAt: '2026-07-30T09:45:00Z',
    createdAt: '2026-07-28T08:20:00Z',
  }),
] satisfies Review[]

/** Notes par critère, produites en même temps que les revues ci-dessus. */
export const reviewScores: ReviewScore[] = allScores

// ---------------------------------------------------------------------------
// Affectations
// ---------------------------------------------------------------------------

function assignment(
  n: number,
  proposal_id: string,
  reviewer_id: string,
  assigned_at: string,
  options: Partial<Pick<ReviewAssignment, 'due_at' | 'recused_at' | 'recusal_reason'>> = {},
): ReviewAssignment {
  return {
    id: REVIEW_ASSIGNMENT(n),
    proposal_id,
    reviewer_id,
    assigned_by: PERSON.bakayoko,
    assigned_at,
    due_at: options.due_at ?? '2026-08-20T23:59:59Z',
    recused_at: options.recused_at ?? null,
    recusal_reason: options.recusal_reason ?? null,
  }
}

export const reviewAssignments = [
  assignment(1, PROPOSAL.adaptationCotiere, PERSON.lemoine, '2026-06-24T08:10:00Z'),
  assignment(2, PROPOSAL.adaptationCotiere, PERSON.duchesne, '2026-06-24T08:10:00Z'),
  assignment(3, PROPOSAL.adaptationCotiere, PERSON.benAmor, '2026-06-24T08:10:00Z'),

  assignment(10, PROPOSAL.pertesPrejudices, PERSON.lemoine, '2026-06-30T09:00:00Z'),
  assignment(11, PROPOSAL.pertesPrejudices, PERSON.duchesne, '2026-06-30T09:00:00Z'),
  assignment(12, PROPOSAL.pertesPrejudices, PERSON.rasoanaivo, '2026-06-30T09:00:00Z'),

  assignment(20, PROPOSAL.budgetsGenre, PERSON.duchesne, '2026-07-27T10:00:00Z'),
  assignment(21, PROPOSAL.budgetsGenre, PERSON.rasoanaivo, '2026-07-27T10:00:00Z'),
  // DÉPORT : la personne siège au comité et travaille dans l'organisation
  // porteuse. Elle se retire, et la trace en reste.
  assignment(22, PROPOSAL.budgetsGenre, PERSON.kabore, '2026-07-27T10:00:00Z', {
    recused_at: '2026-07-28T08:15:00Z',
    recusal_reason:
      "Collaboration en cours avec la Coalition des femmes pour le climat en Afrique centrale sur un programme voisin.",
  }),
  // Remplaçante désignée après le déport : la revue n'est pas encore rendue.
  assignment(23, PROPOSAL.budgetsGenre, PERSON.lemoine, '2026-07-29T09:00:00Z', {
    due_at: '2026-08-25T23:59:59Z',
  }),

  assignment(30, PROPOSAL.cartographieCotonou, PERSON.benAmor, '2026-07-30T08:00:00Z'),
  assignment(31, PROPOSAL.cartographieCotonou, PERSON.kabore, '2026-07-30T08:00:00Z'),
  assignment(32, PROPOSAL.cartographieCotonou, PERSON.rasoanaivo, '2026-07-30T08:00:00Z'),

  assignment(40, PROPOSAL.interpretation, PERSON.duchesne, '2026-07-25T09:00:00Z'),
  assignment(41, PROPOSAL.interpretation, PERSON.lemoine, '2026-07-25T09:00:00Z', {
    due_at: '2026-08-10T23:59:59Z',
  }),
  assignment(42, PROPOSAL.interpretation, PERSON.benAmor, '2026-07-25T09:00:00Z', {
    due_at: '2026-08-10T23:59:59Z',
  }),

  assignment(50, PROPOSAL.reboisementUrbain, PERSON.benAmor, '2026-07-31T09:30:00Z'),
  assignment(51, PROPOSAL.reboisementUrbain, PERSON.kabore, '2026-07-31T09:30:00Z'),
  assignment(52, PROPOSAL.reboisementUrbain, PERSON.rasoanaivo, '2026-07-31T09:30:00Z'),

  assignment(60, PROPOSAL.assuranceParametrique, PERSON.duchesne, '2026-07-28T11:00:00Z'),
  assignment(61, PROPOSAL.assuranceParametrique, PERSON.benAmor, '2026-07-28T11:00:00Z'),
  assignment(62, PROPOSAL.assuranceParametrique, PERSON.rasoanaivo, '2026-07-28T11:00:00Z'),
  // Déport pour appartenance à l'organisation porteuse.
  assignment(63, PROPOSAL.assuranceParametrique, PERSON.kabore, '2026-07-28T11:00:00Z', {
    recused_at: '2026-07-28T15:40:00Z',
    recusal_reason: "Membre de l'organisation porteuse.",
  }),

  assignment(70, PROPOSAL.numeriqueResponsable, PERSON.kabore, '2026-08-03T08:00:00Z'),
  assignment(71, PROPOSAL.numeriqueResponsable, PERSON.duchesne, '2026-08-03T08:00:00Z'),
  assignment(72, PROPOSAL.numeriqueResponsable, PERSON.rasoanaivo, '2026-08-03T08:00:00Z'),

  assignment(80, PROPOSAL.dechetsPlastiques, PERSON.kabore, '2026-08-06T10:00:00Z'),
  assignment(81, PROPOSAL.dechetsPlastiques, PERSON.benAmor, '2026-08-06T10:00:00Z'),
  assignment(82, PROPOSAL.dechetsPlastiques, PERSON.lemoine, '2026-08-06T10:00:00Z'),

  assignment(90, PROPOSAL.ecolesResilientes, PERSON.benAmor, '2026-08-05T09:00:00Z'),
  assignment(91, PROPOSAL.ecolesResilientes, PERSON.rasoanaivo, '2026-08-05T09:00:00Z'),
  assignment(92, PROPOSAL.ecolesResilientes, PERSON.duchesne, '2026-08-05T09:00:00Z'),

  assignment(100, PROPOSAL.captageAir, PERSON.lemoine, '2026-06-30T09:10:00Z'),
  assignment(101, PROPOSAL.captageAir, PERSON.duchesne, '2026-06-30T09:10:00Z'),
  assignment(102, PROPOSAL.captageAir, PERSON.rasoanaivo, '2026-06-30T09:10:00Z'),

  assignment(110, PROPOSAL.creditsVolontaires, PERSON.lemoine, '2026-07-02T08:00:00Z'),
  assignment(111, PROPOSAL.creditsVolontaires, PERSON.duchesne, '2026-07-02T08:00:00Z'),
  assignment(112, PROPOSAL.creditsVolontaires, PERSON.benAmor, '2026-07-02T08:00:00Z'),

  assignment(120, PROPOSAL.retrospectiveCop, PERSON.rasoanaivo, '2026-07-06T07:00:00Z'),
  assignment(121, PROPOSAL.retrospectiveCop, PERSON.kabore, '2026-07-06T07:00:00Z'),
  assignment(122, PROPOSAL.retrospectiveCop, PERSON.duchesne, '2026-07-06T07:00:00Z'),
] satisfies ReviewAssignment[]

// ---------------------------------------------------------------------------
// Contrôle de cohérence
//
// Ce que `refresh_proposal_score()` fait en base, vérifié ici : la moyenne des
// revues SOUMISES doit correspondre aux agrégats portés par la proposition, et
// une note nulle sur un critère éliminatoire doit lever `is_knocked_out`. Une
// divergence échoue au chargement du module, pendant le développement.
// ---------------------------------------------------------------------------

const knockoutCriteria = new Set(reviewCriteria.filter((c) => c.is_knockout).map((c) => c.id))

for (const proposalId of new Set(reviews.map((r) => r.proposal_id))) {
  const submitted = reviews.filter((r) => r.proposal_id === proposalId && r.submitted_at !== null)
  const target = allProposals.find((p) => p.id === proposalId)
  if (!target || submitted.length === 0) continue

  const average = (values: number[]) => values.reduce((a, b) => a + b, 0) / values.length
  const weighted = average(submitted.map((r) => r.weighted_score ?? 0))
  const outOf20 = average(submitted.map((r) => r.score_out_of_20 ?? 0))
  const knockedOut = allScores.some(
    (s) =>
      s.score === 0 &&
      knockoutCriteria.has(s.criterion_id) &&
      submitted.some((r) => r.id === s.review_id),
  )

  const mismatch =
    submitted.length !== target.review_count ||
    Math.abs(weighted - (target.weighted_score ?? -1)) > 0.001 ||
    Math.abs(outOf20 - (target.average_score ?? -1)) > 0.001 ||
    knockedOut !== target.is_knocked_out

  if (mismatch) {
    throw new Error(
      `Mocks incohérents pour ${target.reference_code} : les revues donnent ` +
        `${submitted.length} revue(s), ${weighted.toFixed(2)} / ${maxWeightedScore} et ${outOf20.toFixed(2)} / 20 ` +
        `(éliminé : ${knockedOut}), la proposition porte ` +
        `${target.review_count}, ${target.weighted_score} et ${target.average_score} (éliminé : ${target.is_knocked_out}).`,
    )
  }
}
