/**
 * LA FICHE D'ÉVALUATION D'UNE PROPOSITION (A8) — ce que l'écran reçoit, ce
 * qu'il renvoie quand un membre du comité note, se déporte, écrit ou décide.
 *
 * CE FICHIER NE REDÉCLARE AUCUNE TABLE. Le dossier est `Proposal`, la revue est
 * `Review`, la note par critère est `ReviewScore`, le critère est
 * `ReviewCriterion`, l'échange est `ProposalComment` : tout cela vit déjà dans
 * `types/programme/*.ts` et `types/event/call.ts`, dérivé du SQL. Ce qui est ici
 * est ce que la base ne porte pas — l'assemblage que l'écran demande, le voile
 * de l'évaluation en aveugle, et le résultat des écritures.
 *
 * TROIS RÈGLES DU MODÈLE COMMANDENT CE CONTRAT, et aucune n'est un choix
 * d'interface :
 *
 *  · L'ÉVALUATION EN AVEUGLE est une donnée de l'appel
 *    (`calls_for_proposals.blind_review`), pas une préférence d'affichage. Quand
 *    elle est active et que la personne n'a pas soumis sa revue, les revues de
 *    ses pairs NE SONT PAS ENVOYÉES : elles ne sont ni masquées en CSS ni
 *    filtrées par un composant. Le voile se lève à la seconde où sa propre revue
 *    part — c'est le sens de `blind_veiled` et de `veiled_count`.
 *  · LA GRILLE APPARTIENT À L'APPEL (`event.review_criteria`) : poids, note
 *    maximale et caractère éliminatoire varient d'un appel à l'autre. Aucune
 *    constante d'interface ne les redit, et `max_weighted_score` vient de
 *    `event.max_weighted_score()`.
 *  · LA MACHINE À ÉTATS EST UNE DONNÉE (`proposal_transitions_allowed`) : les
 *    actions de décision de l'en-tête en sont dérivées, avec leur permission et
 *    leur exigence de motif, jamais réécrites dans un composant.
 */

import type { CallForProposals, ReviewCriterion } from './event/call'
import type { EventEdition } from './event/edition'
import type { Person } from './identity'
import type { Asset } from './media'
import type { Organization } from './org'
import type {
  CommentVisibility,
  Proposal,
  ProposalComment,
  ProposalDocument,
  ProposalHistoryEntry,
  ProposalOrganization,
  ProposalSpeaker,
  ProposalStatus,
  ProposalTransition,
} from './programme/proposal'
import type { Review, ReviewAssignment, ReviewRecommendation, ReviewScore } from './programme/review'
import type { ScheduleThemeBadge } from './views'
import type {
  CriterionId,
  IsoDateTime,
  Numeric,
  OrganizationId,
  PersonId,
  ProposalId,
  Url,
  Uuid,
} from './shared'

// ---------------------------------------------------------------------------
// Le dossier, à gauche
// ---------------------------------------------------------------------------

/**
 * CE QU'UNE ORGANISATION A DÉJÀ FAIT AVEC L'IFDD — le sous-ensemble de
 * `analytics.mv_organization_scorecard` (`130_analytics.sql` § 5) qui répond à
 * la question que se pose un membre du comité devant un dossier : cette
 * organisation est-elle une inconnue, une habituée, ou quelqu'un dont trois
 * dossiers ont déjà été écartés ?
 *
 * SEULES LES COLONNES UTILES ICI SONT REPRISES. La projection en porte une
 * quarantaine — membres, articles, octets stockés, score de confiance — qui
 * appartiennent à la fiche d'organisation (A11) et n'ont rien à faire dans un
 * panneau latéral d'évaluation.
 *
 * `ratio_acceptation` est NUL et non zéro quand rien n'a jamais été déposé :
 * c'est la règle portée par la vue elle-même, et un zéro se lirait « jamais
 * retenue », ce qui est un contresens.
 */
export interface OrganizationTrackRecord {
  organization_id: OrganizationId
  propositions_deposees: number
  propositions_acceptees: number
  propositions_rejetees: number
  /** Éditions distinctes sur lesquelles l'organisation a déposé. */
  evenements_couverts: number
  sessions_realisees: number
  /** Moyenne des notes obtenues, toutes éditions confondues. */
  note_moyenne_obtenue: Numeric | null
  /** Acceptées / déposées. Nul si rien n'a jamais été déposé. */
  ratio_acceptation: Numeric | null
  /** Dernier dépôt connu, pour distinguer une habituée d'une revenante. */
  derniere_proposition: IsoDateTime | null
}

/**
 * Une organisation associée au dossier, avec de quoi la nommer et la situer.
 *
 * `link` porte le RÔLE (porteur, co-organisateur, partenaire, soutien) et la
 * confirmation : une co-organisation annoncée engage un tiers, et tant que
 * `confirmed_at` est nul, le back-office doit l'afficher « en attente » plutôt
 * que de la compter comme acquise.
 */
export interface ProposalOrganizationEntry {
  link: ProposalOrganization
  organization: Organization | null
  /** Historique de participation ; nul pour une fiche qui n'a jamais rien déposé. */
  track_record: OrganizationTrackRecord | null
}

/** Un intervenant annoncé, avec la personne derrière la ligne d'activité. */
export interface ProposalSpeakerEntry {
  speaker: ProposalSpeaker
  person: Person | null
}

/**
 * Une pièce du dossier, avec l'objet stocké qu'elle désigne.
 *
 * `asset` porte l'état du fichier : seul `ready` est servi. Un objet en
 * quarantaine (`scan_verdict: 'infected'`) doit être ANNONCÉ comme tel et non
 * proposé au téléchargement — le comité doit savoir qu'une pièce manque à son
 * dossier, pas cliquer sur un lien mort.
 */
export interface ProposalDocumentEntry {
  document: ProposalDocument
  asset: Asset | null
  /**
   * Adresse de téléchargement, composée EN BASE par `media.object_url()` à
   * partir du couple `(bucket, object_key)` — jamais fabriquée par un composant.
   * Nulle quand l'objet n'est pas servi : quarantaine, purge, téléversement
   * inachevé. C'est cette nullité qui commande l'avertissement plutôt que le
   * bouton.
   */
  url: Url | null
}

// ---------------------------------------------------------------------------
// L'évaluation, à droite
// ---------------------------------------------------------------------------

/**
 * LA REVUE D'UN PAIR, telle que l'écran la montre — la revue elle-même, ses
 * notes par critère et le nom de son auteur.
 *
 * ELLE N'EST ENVOYÉE QUE SI LE LECTEUR Y A DROIT. En évaluation en aveugle, un
 * membre du comité qui n'a pas encore soumis la sienne ne reçoit rien : ni note,
 * ni recommandation, ni nom. Le décompte, lui, reste connu (`veiled_count`) —
 * savoir que deux revues existent n'ancre personne, lire leurs notes si.
 */
export interface PeerReview {
  review: Review
  scores: ReviewScore[]
  reviewer: Person | null
  /** L'affectation correspondante : son échéance, et le déport le cas échéant. */
  assignment: ReviewAssignment | null
}

/**
 * UNE AFFECTATION VUE PAR L'ÉCRAN — qui doit évaluer, pour quand, où il en est.
 *
 * `state` est calculé une fois ici plutôt que trois fois dans les composants :
 * l'en-tête l'affiche en avancement, le panneau en liste, la barre de retard en
 * alerte. Trois calculs séparés divergeraient sur le cas limite qui compte —
 * une revue commencée mais non soumise n'est PAS une revue rendue.
 */
export type ReviewProgressState = 'submitted' | 'drafted' | 'pending' | 'overdue' | 'recused'

export interface CommitteeMemberProgress {
  assignment: ReviewAssignment
  person: Person | null
  state: ReviewProgressState
  submitted_at: IsoDateTime | null
}

/**
 * LA REVUE DE LA PERSONNE CONNECTÉE, prête à être éditée.
 *
 * `scores` est indexé par critère : la grille est celle de l'appel, et une note
 * absente n'est pas une note à zéro — zéro sur un critère éliminatoire
 * DISQUALIFIE le dossier, ce qui n'est jamais ce qu'on veut dire en n'ayant pas
 * encore noté.
 */
export interface MyReview {
  review: Review | null
  scores: Record<CriterionId, Numeric>
  /** Commentaire attaché à un critère — `review_scores.comment`. */
  comments: Record<CriterionId, string>
  /** L'affectation qui m'a confié ce dossier ; nulle si je ne suis pas affecté. */
  assignment: ReviewAssignment | null
}

// ---------------------------------------------------------------------------
// Ce que la personne connectée a le droit de faire
// ---------------------------------------------------------------------------

/**
 * LES DROITS DE CET ÉCRAN, résolus UNE FOIS à la source plutôt que testés dans
 * six composants.
 *
 * Ils ne sont pas un contrôle de sécurité : l'API refera le contrôle, et une
 * action masquée reste refusée si l'URL est forgée. Ce qu'ils garantissent, c'est
 * que la notation et la décision ne se ressemblent pas à l'écran — un membre du
 * comité qui note ne doit pas voir un bouton « Retenir » qui lui sera refusé.
 */
export interface ReviewDeskPermissions {
  /** `programme.review.write` — noter, commenter, se déporter. */
  can_review: boolean
  /** `programme.proposal.decide` — retenir, demander des corrections, rejeter. */
  can_decide: boolean
  /** `event.call.manage` — affecter un membre du comité à ce dossier. */
  can_assign: boolean
  /** Suis-je AFFECTÉ à ce dossier ? Décorrélé de la permission : un membre du
   *  comité peut lire un dossier qu'on ne lui a pas confié, sans le noter. */
  is_assigned: boolean
  /** Me suis-je déporté ? La grille est alors close, et la trace demeure. */
  is_recused: boolean
}

// ---------------------------------------------------------------------------
// La réponse de l'écran
// ---------------------------------------------------------------------------

/**
 * TOUT L'ÉCRAN EN UNE RÉPONSE. Le dossier, son édition, son appel, sa grille,
 * ses organisations, ses intervenants, ses pièces, son historique, ses échanges,
 * l'avancement du comité, ma revue, et celles de mes pairs quand j'ai le droit
 * de les lire.
 *
 * UNE COMPOSITION, PAS QUINZE LECTURES. Ce que la page affiche vient de onze
 * tables ; les demander une par une depuis le composant, c'est le N+1 que la
 * v1 pratiquait, et c'est aussi la porte par laquelle une note interne finit
 * dans un écran qui ne devait pas la montrer. La composition appartiendra à
 * l'API (prompt B8), et le voile de l'aveugle avec elle.
 */
export interface ReviewDeskScreen {
  proposal: Proposal
  edition: EventEdition
  /** Nul pour une proposition hors appel, créée par l'IFDD. */
  call: CallForProposals | null

  // --- Le dossier, colonne de gauche ---------------------------------------
  organizations: ProposalOrganizationEntry[]
  speakers: ProposalSpeakerEntry[]
  documents: ProposalDocumentEntry[]
  /** Thématiques prêtes à afficher — `reference.term_badges()`, libellé et
   *  couleur venus de la base, jamais d'un fichier i18n. */
  themes: ScheduleThemeBadge[]
  /** Journal des changements d'état — `programme.proposal_transitions`. */
  transitions: ProposalTransition[]
  /** Historique champ par champ — `programme.proposal_history()`. */
  history: ProposalHistoryEntry[]

  // --- L'évaluation, colonne de droite -------------------------------------
  /** La grille de CET appel, dans l'ordre `sort_order`. */
  criteria: ReviewCriterion[]
  /** `event.max_weighted_score()` — le dénominateur de la conversion sur 20. */
  max_weighted_score: Numeric
  /** `calls_for_proposals.required_reviews` : le dénominateur du « 2/3 ». */
  required_reviews: number | null
  /** `calls_for_proposals.blind_review` : la règle de l'appel, telle quelle. */
  blind_review: boolean
  /**
   * LE VOILE EST-IL BAISSÉ POUR MOI ? Vrai quand l'appel est en aveugle, que je
   * suis affecté et que ma revue n'est pas soumise. Un administrateur qui décide
   * sans noter n'est pas concerné : l'effet d'ancrage vise celui qui va poser
   * une note, et masquer les notes à qui doit trancher rendrait la décision
   * impossible.
   */
  blind_veiled: boolean
  /** Revues soumises que le voile me cache. Compter n'ancre pas ; lire, si. */
  veiled_count: number
  my_review: MyReview
  /** Revues des pairs. VIDE quand `blind_veiled` : elles ne sont pas envoyées. */
  peer_reviews: PeerReview[]
  /** L'avancement nominatif du comité, déports compris. */
  committee: CommitteeMemberProgress[]

  // --- Échanges et droits ---------------------------------------------------
  /** Tous les échanges que CE lecteur a le droit de voir, les trois visibilités
   *  confondues, filtrés à la source. */
  comments: ProposalComment[]
  /** Auteurs des messages et des revues, pour ne pas résoudre les noms un par un. */
  participants: Person[]
  permissions: ReviewDeskPermissions
  /** Rang du dossier dans son édition, et le reste de l'en-tête : la ligne de
   *  `v_proposal_dashboard` est déjà calculée, on ne la refait pas. */
  rank: number
  /** Ai-je déjà ouvert ce dossier avant cette visite ? L'ouverture le pose
   *  (`programme.record_proposal_read`), la réponse dit l'état d'AVANT. */
  first_visit: boolean
  /** Membres du comité ayant ouvert le dossier — `proposal_reads`, collectif. */
  read_count: number
}

// ---------------------------------------------------------------------------
// Écritures
// ---------------------------------------------------------------------------

/**
 * ENREGISTREMENT ET DÉPÔT D'UNE REVUE — une seule charge utile, deux verbes.
 *
 * `submit: false` garde la revue en brouillon : elle ne compte dans aucun
 * agrégat (`refresh_proposal_score()` ne retient que `submitted_at IS NOT NULL`)
 * et reste invisible des pairs. `submit: true` la dépose, déclenche le recalcul
 * de la note du dossier et LÈVE LE VOILE de l'évaluation en aveugle.
 */
export interface SaveReviewPayload {
  proposal_id: ProposalId
  recommendation: ReviewRecommendation
  /** Note par critère. Une entrée absente est une note NON POSÉE, pas un zéro. */
  scores: Record<CriterionId, Numeric>
  comments: Record<CriterionId, string>
  strengths: string | null
  weaknesses: string | null
  /** Visible du seul comité, JAMAIS du soumissionnaire. */
  private_note: string | null
  submit: boolean
}

/**
 * Ce que l'API rend après une notation : la revue consolidée et les agrégats
 * recalculés du dossier, pour que l'en-tête change sans recharger la page.
 */
export interface SaveReviewResult {
  review: Review
  /** Note pondérée moyenne du dossier après recalcul. */
  proposal_weighted_score: Numeric | null
  proposal_average_score: Numeric | null
  review_count: number
  /** Un critère éliminatoire a-t-il reçu zéro, toutes revues soumises confondues ? */
  is_knocked_out: boolean
}

/**
 * DÉPORT — le révisionniste déclare un lien avec l'organisation et se retire.
 *
 * LE MOTIF EST OBLIGATOIRE et c'est le sujet : `review_assignments.recusal_reason`
 * existe pour tracer l'impartialité du comité. Un déport sans motif ne prouve
 * rien et ne se relit pas six mois plus tard, quand une organisation conteste.
 */
export interface RecusalPayload {
  proposal_id: ProposalId
  reason: string
}

/**
 * UN MESSAGE SUR LE DOSSIER, avec SA VISIBILITÉ — le champ le plus dangereux de
 * cet écran.
 *
 * `visibility` est un état explicite du modèle (`programme.comment_visibility`)
 * et non une case à cocher facultative : `committee` reste entre membres du
 * comité, `submitter` PART CHEZ LE DÉPOSANT, `private` n'est lu que de son
 * auteur. Se tromper est irrattrapable — un message lu ne se retire pas —, d'où
 * la confirmation exigée par l'écran au premier envoi partagé.
 *
 * `is_change_request` n'est possible que sur un message partagé : une demande de
 * correction que le déposant ne verrait pas bloquerait son dossier sans qu'il
 * sache pourquoi.
 */
export interface PostCommentPayload {
  proposal_id: ProposalId
  parent_id: Uuid | null
  visibility: CommentVisibility
  body: string
  is_change_request: boolean
}

/**
 * DÉCISION SUR LE DOSSIER — la transition d'état, prise depuis l'en-tête.
 *
 * `reason` est exigé par le trigger `tg_guard_proposal_status()` pour les
 * transitions dont `requires_reason` est vrai : le rejet, l'annulation, la
 * demande de correction. L'écran le sait en lisant
 * `proposal_transitions_allowed`, il ne le devine pas.
 */
export interface DecisionPayload {
  proposal_id: ProposalId
  to_status: ProposalStatus
  reason: string | null
}

/**
 * Ce que rend une décision : le dossier dans son nouvel état et la ligne de
 * journal qui vient d'être écrite. Le refus, lui, n'est pas une exception mais
 * une réponse — l'écran le rend comme telle.
 */
export type DecisionResult =
  | { status: 'applied'; proposal: Proposal; transition: ProposalTransition }
  | { status: 'transition_not_allowed' }
  | { status: 'reason_required' }
