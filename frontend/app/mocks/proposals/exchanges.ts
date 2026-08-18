/**
 * Données simulées des échanges autour d'un dossier :
 * `programme.proposal_comments` et `programme.proposal_transitions`.
 *
 * LA VISIBILITÉ D'UN COMMENTAIRE EST UN ÉTAT EXPLICITE, pas un tableau de
 * destinataires : `committee` (le comité seul), `submitter` (visible du
 * soumissionnaire), `private` (note personnelle de son auteur). La v1 stockait
 * des identifiants de destinataires dans un tableau sans contrainte : personne
 * ne savait plus qui voyait quoi.
 *
 * LES DEMANDES DE CORRECTION (`is_change_request`) pilotent l'état
 * `changes_requested`. Un fil résolu (`resolved_at`) ne bloque plus le dossier ;
 * un fil ouvert, si. Les trois dossiers en correction du jeu de données portent
 * chacun leur demande, formulée en français et adressée au déposant.
 *
 * LE JOURNAL DES TRANSITIONS est ce que lit l'onglet « Historique » : qui a fait
 * quoi, quand, et pourquoi quand la base l'exige. Il n'est pas exhaustif ici —
 * il couvre les dossiers dont les écrans du jalon montrent le parcours.
 */

import type { ProposalComment, ProposalTransition } from '~/types/programme/proposal'
import { PERSON, PROPOSAL, PROPOSAL_COMMENT, PROPOSAL_TRANSITION } from '../ids'

// ---------------------------------------------------------------------------
// Commentaires
// ---------------------------------------------------------------------------

export const proposalComments = [
  // --- Demandes de correction ouvertes -------------------------------------
  {
    id: PROPOSAL_COMMENT(1),
    proposal_id: PROPOSAL.numeriqueResponsable,
    parent_id: null,
    author_id: PERSON.bakayoko,
    visibility: 'submitter',
    body: "Le dossier ne précise pas quelles administrations ont été inventoriées, ni sur quelle période. Merci de compléter la présentation détaillée sur ces deux points, et d'indiquer si les résultats peuvent être cités publiquement. Sans cette précision, le comité ne peut pas se prononcer sur la faisabilité.",
    is_change_request: true,
    resolved_at: null,
    resolved_by: null,
    edited_at: null,
    deleted_at: null,
    created_at: '2026-08-10T08:30:00Z',
  },
  {
    id: PROPOSAL_COMMENT(2),
    proposal_id: PROPOSAL.numeriqueResponsable,
    parent_id: PROPOSAL_COMMENT(1),
    author_id: PERSON.moreau,
    visibility: 'submitter',
    body: "Bien reçu. Les deux administrations concernées doivent donner leur accord pour être nommées ; je reviens vers vous avant la fin de la semaine.",
    is_change_request: false,
    resolved_at: null,
    resolved_by: null,
    edited_at: null,
    deleted_at: null,
    created_at: '2026-08-11T14:05:00Z',
  },
  {
    id: PROPOSAL_COMMENT(3),
    proposal_id: PROPOSAL.dechetsPlastiques,
    parent_id: null,
    author_id: PERSON.perretAdmin,
    visibility: 'submitter',
    body: "L'annexe photographique n'a pas pu être analysée : le fichier a été mis en quarantaine par l'antivirus. Merci de la redéposer, si possible en PDF plutôt qu'en archive compressée. Par ailleurs, la durée demandée est de 90 minutes pour cinq intervenants : le format tiendra difficilement.",
    is_change_request: true,
    resolved_at: null,
    resolved_by: null,
    edited_at: '2026-08-12T15:10:00Z',
    deleted_at: null,
    created_at: '2026-08-12T15:00:00Z',
  },
  {
    id: PROPOSAL_COMMENT(4),
    proposal_id: PROPOSAL.ecolesResilientes,
    parent_id: null,
    author_id: PERSON.bakayoko,
    visibility: 'submitter',
    body: "Le dossier est solide sur les mesures thermiques, mais l'effet sur la fréquentation scolaire est annoncé sans être documenté. Merci de préciser la source de ces données, ou de retirer cette affirmation de la présentation.",
    is_change_request: true,
    resolved_at: null,
    resolved_by: null,
    edited_at: null,
    deleted_at: null,
    created_at: '2026-08-13T09:20:00Z',
  },

  // --- Demande de correction déjà résolue ----------------------------------
  {
    id: PROPOSAL_COMMENT(5),
    proposal_id: PROPOSAL.adaptationCotiere,
    parent_id: null,
    author_id: PERSON.nkoDiop,
    visibility: 'submitter',
    body: "Merci de confirmer la présence physique des quatre intervenants annoncés : deux d'entre eux n'apparaissent pas dans la liste des accréditations transmise par la délégation.",
    is_change_request: true,
    resolved_at: '2026-07-12T09:30:00Z',
    resolved_by: PERSON.nkoDiop,
    edited_at: null,
    deleted_at: null,
    created_at: '2026-07-08T11:00:00Z',
  },
  {
    id: PROPOSAL_COMMENT(6),
    proposal_id: PROPOSAL.adaptationCotiere,
    parent_id: PROPOSAL_COMMENT(5),
    author_id: PERSON.sowFall,
    visibility: 'submitter',
    body: "Les accréditations des quatre intervenants sont désormais déposées auprès du point focal national. Justificatifs joints au dossier.",
    is_change_request: false,
    resolved_at: null,
    resolved_by: null,
    edited_at: null,
    deleted_at: null,
    created_at: '2026-07-11T16:20:00Z',
  },

  // --- Échanges internes au comité, invisibles du soumissionnaire ----------
  {
    id: PROPOSAL_COMMENT(7),
    proposal_id: PROPOSAL.captageAir,
    parent_id: null,
    author_id: PERSON.lemoine,
    visibility: 'committee',
    body: "Aucun lien avec un territoire francophone, aucun intervenant du Sud. Je mets zéro à la pertinence en connaissance de cause : c'est un critère éliminatoire, et il est fait pour ces cas-là.",
    is_change_request: false,
    resolved_at: null,
    resolved_by: null,
    edited_at: null,
    deleted_at: null,
    created_at: '2026-07-25T10:40:00Z',
  },
  {
    id: PROPOSAL_COMMENT(8),
    proposal_id: PROPOSAL.captageAir,
    parent_id: PROPOSAL_COMMENT(7),
    author_id: PERSON.duchesne,
    visibility: 'committee',
    body: "D'accord sur le fond. Il faudra une motivation écrite au déposant : c'est sa troisième proposition, et deux ont déjà été écartées.",
    is_change_request: false,
    resolved_at: null,
    resolved_by: null,
    edited_at: null,
    deleted_at: null,
    created_at: '2026-07-25T13:15:00Z',
  },
  {
    id: PROPOSAL_COMMENT(9),
    proposal_id: PROPOSAL.assuranceParametrique,
    parent_id: null,
    author_id: PERSON.benAmor,
    visibility: 'committee',
    body: "Excellent dossier, mais attention au chevauchement thématique avec la session sur les budgets sensibles au genre : les deux traitent du décaissement. À placer sur des créneaux éloignés.",
    is_change_request: false,
    resolved_at: null,
    resolved_by: null,
    edited_at: null,
    deleted_at: null,
    created_at: '2026-08-01T09:00:00Z',
  },

  // --- Note personnelle, visible de son seul auteur -------------------------
  {
    id: PROPOSAL_COMMENT(10),
    proposal_id: PROPOSAL.interpretation,
    parent_id: null,
    author_id: PERSON.perretAdmin,
    visibility: 'private',
    body: "Relancer Hélène Lemoine : sa revue manque depuis trois semaines et le dossier est porté par l'IFDD lui-même.",
    is_change_request: false,
    resolved_at: null,
    resolved_by: null,
    edited_at: null,
    deleted_at: null,
    created_at: '2026-08-05T07:45:00Z',
  },

  // --- Le dossier de la fiche d'évaluation (A8) : les trois visibilités -----
  //
  // COP31-00020 est le dossier sur lequel se règle l'écran d'évaluation : il est
  // en cours de revue, co-organisé, porte un déport et une revue encore
  // attendue. Il lui manquait ses échanges — sans eux, la distinction entre ce
  // que le comité s'écrit et ce qui est adressé au déposant n'avait aucune
  // donnée à montrer, alors qu'elle est le risque principal de cet écran.
  {
    id: PROPOSAL_COMMENT(12),
    proposal_id: PROPOSAL.budgetsGenre,
    parent_id: null,
    author_id: PERSON.duchesne,
    visibility: 'committee',
    body: "Le sujet tient, mais les trois pays annoncés ne sont pas nommés et le calendrier budgétaire visé n'est pas décrit. Avant de demander une correction, vérifions si le guide annoncé existe déjà : s'il reste à écrire, la session promet un livrable qu'elle ne produira pas.",
    is_change_request: false,
    resolved_at: null,
    resolved_by: null,
    edited_at: null,
    deleted_at: null,
    created_at: '2026-08-04T10:20:00Z',
  },
  {
    id: PROPOSAL_COMMENT(13),
    proposal_id: PROPOSAL.budgetsGenre,
    parent_id: PROPOSAL_COMMENT(12),
    author_id: PERSON.rasoanaivo,
    visibility: 'committee',
    body: "Le guide existe en version de travail, je l'ai vu circuler en juin. Reste la question du second intervenant : sans quelqu'un d'une direction du budget, la session parlera de finances publiques sans les financiers.",
    is_change_request: false,
    resolved_at: null,
    resolved_by: null,
    edited_at: null,
    deleted_at: null,
    created_at: '2026-08-05T16:40:00Z',
  },
  {
    // PARTAGÉ AVEC LE DÉPOSANT, et ce n'est pas une demande de correction : une
    // précision peut se demander sans renvoyer le dossier, ce que la v1 ne
    // savait pas faire — tout échange y rouvrait le dossier.
    id: PROPOSAL_COMMENT(14),
    proposal_id: PROPOSAL.budgetsGenre,
    parent_id: null,
    author_id: PERSON.perretAdmin,
    visibility: 'submitter',
    body: "Le comité examine votre dossier cette semaine. Pourriez-vous préciser si un représentant d'une direction du budget sera présent au panel ? Cette information n'appelle pas de nouveau dépôt : une réponse ici suffit.",
    is_change_request: false,
    resolved_at: null,
    resolved_by: null,
    edited_at: null,
    deleted_at: null,
    created_at: '2026-08-07T08:15:00Z',
  },
  {
    id: PROPOSAL_COMMENT(15),
    proposal_id: PROPOSAL.budgetsGenre,
    parent_id: PROPOSAL_COMMENT(14),
    author_id: PERSON.ngoBassong,
    visibility: 'submitter',
    body: "Oui : la directrice du budget du ministère des finances camerounais a confirmé sa venue par courriel le 2 août. Je joins la confirmation au dossier dès que l'accréditation est déposée.",
    is_change_request: false,
    resolved_at: null,
    resolved_by: null,
    edited_at: null,
    deleted_at: null,
    created_at: '2026-08-08T11:30:00Z',
  },
  {
    // NOTE PERSONNELLE : visible de son seul auteur, y compris des autres
    // membres du comité. Elle est ici pour que la fiche d'évaluation ait de quoi
    // éprouver la troisième visibilité sur le dossier qu'elle montre.
    id: PROPOSAL_COMMENT(16),
    proposal_id: PROPOSAL.budgetsGenre,
    parent_id: null,
    author_id: PERSON.lemoine,
    visibility: 'private',
    body: "Reprendre la note méthodologique avant de noter : le marqueur genre y est décrit autrement que dans la présentation du dossier.",
    is_change_request: false,
    resolved_at: null,
    resolved_by: null,
    edited_at: null,
    deleted_at: null,
    created_at: '2026-08-09T07:05:00Z',
  },

  // --- Commentaire supprimé, conservé en base ------------------------------
  {
    id: PROPOSAL_COMMENT(11),
    proposal_id: PROPOSAL.creditsVolontaires,
    parent_id: null,
    author_id: PERSON.rasoanaivo,
    visibility: 'committee',
    body: '',
    is_change_request: false,
    resolved_at: null,
    resolved_by: null,
    edited_at: null,
    deleted_at: '2026-07-27T11:00:00Z',
    created_at: '2026-07-26T18:30:00Z',
  },
] satisfies ProposalComment[]

// ---------------------------------------------------------------------------
// Journal des transitions
// ---------------------------------------------------------------------------

/** Raccourci d'écriture : une transition d'état datée. */
function transition(
  n: number,
  proposal_id: string,
  from_status: ProposalTransition['from_status'],
  to_status: ProposalTransition['to_status'],
  actor_id: string | null,
  occurred_at: string,
  reason: string | null = null,
): ProposalTransition {
  return {
    id: PROPOSAL_TRANSITION(n),
    proposal_id,
    from_status,
    to_status,
    actor_id,
    reason,
    occurred_at,
  }
}

export const proposalTransitions = [
  // Parcours complet d'un dossier retenu, correction comprise.
  transition(1, PROPOSAL.adaptationCotiere, null, 'draft', PERSON.sowFall, '2026-06-02T10:30:00Z'),
  transition(2, PROPOSAL.adaptationCotiere, 'draft', 'submitted', PERSON.sowFall, '2026-06-10T09:00:00Z'),
  transition(3, PROPOSAL.adaptationCotiere, 'submitted', 'under_review', PERSON.bakayoko, '2026-06-24T08:00:00Z'),
  transition(
    4,
    PROPOSAL.adaptationCotiere,
    'under_review',
    'changes_requested',
    PERSON.nkoDiop,
    '2026-07-08T11:00:00Z',
    "Accréditations des intervenants à confirmer.",
  ),
  transition(5, PROPOSAL.adaptationCotiere, 'changes_requested', 'under_review', PERSON.sowFall, '2026-07-11T16:25:00Z'),
  transition(
    6,
    PROPOSAL.adaptationCotiere,
    'under_review',
    'accepted',
    PERSON.bakayoko,
    '2026-07-28T14:00:00Z',
    "Sujet prioritaire, panel équilibré, montage régional inédit dans le programme.",
  ),

  // Dossier éliminé par le critère éliminatoire.
  transition(10, PROPOSAL.captageAir, null, 'draft', PERSON.moreau, '2026-06-12T14:00:00Z'),
  transition(11, PROPOSAL.captageAir, 'draft', 'submitted', PERSON.moreau, '2026-06-18T10:30:00Z'),
  transition(12, PROPOSAL.captageAir, 'submitted', 'under_review', PERSON.bakayoko, '2026-06-30T09:00:00Z'),
  transition(
    13,
    PROPOSAL.captageAir,
    'under_review',
    'rejected',
    PERSON.bakayoko,
    '2026-08-02T11:00:00Z',
    "Sujet sans lien avec les priorités du pavillon francophone. Note éliminatoire sur le critère de pertinence.",
  ),

  // Retrait à l'initiative de l'organisation.
  transition(20, PROPOSAL.energiesMarines, null, 'draft', PERSON.tranVanMinh, '2026-07-08T05:00:00Z'),
  transition(21, PROPOSAL.energiesMarines, 'draft', 'submitted', PERSON.tranVanMinh, '2026-07-15T04:30:00Z'),
  transition(
    22,
    PROPOSAL.energiesMarines,
    'submitted',
    'withdrawn',
    PERSON.tranVanMinh,
    '2026-08-09T03:15:00Z',
    "Financement du déplacement de la délégation non obtenu.",
  ),

  // Annulation par l'IFDD : le motif est exigé par la base.
  transition(30, PROPOSAL.salonSolutions, null, 'draft', PERSON.moreau, '2026-07-14T11:30:00Z'),
  transition(31, PROPOSAL.salonSolutions, 'draft', 'submitted', PERSON.moreau, '2026-07-20T08:00:00Z'),
  transition(
    32,
    PROPOSAL.salonSolutions,
    'submitted',
    'cancelled',
    PERSON.bakayoko,
    '2026-08-11T14:45:00Z',
    "Format incompatible avec les moyens du pavillon.",
  ),

  // Dossier EN COURS D'ÉVALUATION — celui que montre la fiche d'évaluation (A8).
  // Son journal s'arrête au passage en revue : c'est précisément l'état où une
  // décision reste à prendre, et la frise de l'écran doit pouvoir le rendre.
  transition(50, PROPOSAL.budgetsGenre, null, 'draft', PERSON.ngoBassong, '2026-06-24T11:00:00Z'),
  transition(51, PROPOSAL.budgetsGenre, 'draft', 'submitted', PERSON.ngoBassong, '2026-07-02T09:30:00Z'),
  transition(52, PROPOSAL.budgetsGenre, 'submitted', 'under_review', PERSON.bakayoko, '2026-07-27T10:00:00Z'),

  // Dossier en correction, encore ouvert.
  transition(40, PROPOSAL.ecolesResilientes, null, 'draft', PERSON.josephPierre, '2026-07-15T10:05:00Z'),
  transition(41, PROPOSAL.ecolesResilientes, 'draft', 'submitted', PERSON.josephPierre, '2026-07-24T16:10:00Z'),
  transition(42, PROPOSAL.ecolesResilientes, 'submitted', 'under_review', PERSON.perretAdmin, '2026-08-03T09:00:00Z'),
  transition(
    43,
    PROPOSAL.ecolesResilientes,
    'under_review',
    'changes_requested',
    PERSON.bakayoko,
    '2026-08-13T09:20:00Z',
    "Effet sur la fréquentation scolaire non documenté.",
  ),
] satisfies ProposalTransition[]
