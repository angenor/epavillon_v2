/**
 * LA FICHE D'ÉVALUATION D'UNE PROPOSITION (A8), composée à partir des données
 * simulées — et les quatre écritures qu'elle permet.
 *
 * CE QUE CE FICHIER AJOUTE, ET RIEN DE PLUS. Le dossier, ses organisations, ses
 * intervenants, ses pièces, ses échanges et ses revues existent déjà dans
 * `mocks/proposals/`, `mocks/reviews.ts` et `mocks/criteria.ts` : on les
 * assemble, on ne les réécrit pas. Le travail propre à cet écran est ailleurs —
 * appliquer le VOILE de l'évaluation en aveugle, filtrer les échanges selon ce
 * que le lecteur a le droit de voir, et rejouer `refresh_proposal_score()` après
 * chaque note posée.
 *
 * LE VOILE EST APPLIQUÉ ICI, PAS DANS UN COMPOSANT. En évaluation en aveugle
 * (`calls_for_proposals.blind_review`), les revues des pairs ne sont PAS
 * envoyées à qui n'a pas encore soumis la sienne. Les masquer côté écran
 * laisserait les notes dans la réponse, donc dans l'onglet réseau du navigateur,
 * donc lisibles de qui les cherche — un voile qui ne cache rien.
 *
 * LES TROIS VISIBILITÉS DES ÉCHANGES SONT FILTRÉES À LA SOURCE, pour la même
 * raison : `private` n'est rendu qu'à son auteur, `committee` qu'à un membre du
 * comité, `submitter` à tous ceux qui accèdent au dossier. Un composant ne doit
 * jamais être le dernier rempart entre une note interne et le déposant.
 *
 * LES ÉCRITURES SONT DES ÉCRITURES DE SESSION, comme dans
 * `mocks/admin-proposals.ts` : elles modifient un tampon en mémoire que la
 * lecture suivante applique par-dessus les données simulées. Le tampon des
 * STATUTS et celui des AFFECTATIONS sont ceux de la liste (A7), importés et non
 * recréés : un dossier retenu depuis la fiche doit apparaître retenu dans la
 * liste, et un dossier confié depuis la liste doit apparaître dans le comité de
 * la fiche.
 *
 * LE PÉRIMÈTRE D'ADMINISTRATION N'EST PAS VÉRIFIÉ ICI : `useApi()` s'en charge
 * avant d'appeler ces fonctions (`assertEventInScope`), et l'API le refera.
 */

import type {
  CommitteeMemberProgress,
  DecisionPayload,
  DecisionResult,
  MyReview,
  OrganizationTrackRecord,
  PeerReview,
  PostCommentPayload,
  ProposalDocumentEntry,
  ProposalOrganizationEntry,
  ProposalSpeakerEntry,
  RecusalPayload,
  ReviewDeskPermissions,
  ReviewDeskScreen,
  SaveReviewPayload,
  SaveReviewResult,
} from '~/types/admin-review'
import type { ReviewCriterion } from '~/types/event/call'
import type { Person } from '~/types/identity'
import type { Proposal, ProposalComment, ProposalTransition } from '~/types/programme/proposal'
import type { Review, ReviewAssignment, ReviewScore } from '~/types/programme/review'
import type { CriterionId, Numeric, Uuid } from '~/types/shared'
import { PROPOSAL_COMMENT, PROPOSAL_TRANSITION, REVIEW, REVIEW_ASSIGNMENT } from './ids'
import { events } from './event'
import { callsForProposals } from './calls'
import { maxWeightedScoreOf, reviewCriteria } from './criteria'
import { organizations } from './org'
import { people } from './people'
import {
  allProposals,
  proposalAssets,
  proposalComments,
  proposalDocuments,
  proposalOrganizations,
  proposalSpeakers,
  proposalTransitions,
} from './proposals'
import { proposalHistory } from './proposals/history'
import { proposalReads } from './proposal-reads'
import { reviewAssignments, reviewScores, reviews } from './reviews'
import { allSessions } from './sessions'
import { effectivePermissions } from './permissions'
import {
  changeProposalStatus,
  sessionProposalStatus,
  sessionReviewAssignments,
} from './admin-proposals'
import { proposalDashboard } from './views'
import { progressState } from '~/utils/review-scoring'

const organizationById = new Map(organizations.map((o) => [o.id, o]))
const personById = new Map(people.map((p) => [p.id, p]))
const assetById = new Map(proposalAssets.map((a) => [a.id, a]))

// ---------------------------------------------------------------------------
// Écritures de session
// ---------------------------------------------------------------------------

/** Revues créées ou modifiées pendant cette visite, par (dossier, personne). */
const sessionReviews = new Map<string, Review>()
/** Notes par critère des revues ci-dessus, par identifiant de revue. */
const sessionScores = new Map<Uuid, ReviewScore[]>()
/** Déports déclarés pendant cette visite, par (dossier, personne). */
const sessionRecusals = new Map<string, { recused_at: string; reason: string }>()
/** Messages écrits pendant cette visite. */
const sessionComments: ProposalComment[] = []
/** Transitions journalisées pendant cette visite. */
const sessionTransitions: ProposalTransition[] = []
/** Dossiers ouverts pendant cette visite — l'équivalent de `record_proposal_read`. */
const sessionReads = new Set<string>()

let reviewSeq = 900
let commentSeq = 900
let transitionSeq = 900
let assignmentSeq = 950

const keyOf = (proposalId: Uuid, personId: Uuid): string => `${proposalId}:${personId}`

// ---------------------------------------------------------------------------
// Lectures composées
// ---------------------------------------------------------------------------

/**
 * Les affectations d'un dossier — celles de la base, celles ajoutées par la
 * liste (A7) et les déports déclarés depuis la fiche.
 *
 * LES DÉPORTS NE SONT PAS RETIRÉS de la liste, ils sont MARQUÉS. La colonne
 * `recused_at` existe pour tracer l'impartialité du comité : effacer
 * l'affectation ferait disparaître la déclaration en même temps que l'obligation.
 */
function assignmentsOf(proposalId: Uuid): ReviewAssignment[] {
  return [...reviewAssignments, ...sessionReviewAssignments()]
    .filter((assignment) => assignment.proposal_id === proposalId)
    .map((assignment) => {
      const recusal = sessionRecusals.get(keyOf(proposalId, assignment.reviewer_id))
      if (!recusal) return assignment
      return { ...assignment, recused_at: recusal.recused_at, recusal_reason: recusal.reason }
    })
}

/** Les revues d'un dossier, tampon de session appliqué par-dessus la base. */
function reviewsOf(proposalId: Uuid): Review[] {
  const base = reviews.filter((review) => review.proposal_id === proposalId)
  const merged = base.map((review) => sessionReviews.get(keyOf(proposalId, review.reviewer_id)) ?? review)
  const known = new Set(merged.map((review) => review.reviewer_id))

  for (const [key, review] of sessionReviews) {
    if (key.startsWith(`${proposalId}:`) && !known.has(review.reviewer_id)) merged.push(review)
  }
  return merged
}

/** Les notes par critère d'une revue, tampon compris. */
function scoresOf(reviewId: Uuid): ReviewScore[] {
  return sessionScores.get(reviewId) ?? reviewScores.filter((score) => score.review_id === reviewId)
}

/**
 * CE QUE CE LECTEUR A LE DROIT DE VOIR DANS LE FIL.
 *
 * `private` n'appartient qu'à son auteur — y compris pour un autre membre du
 * comité. `committee` suppose d'en être : quelqu'un qui n'a ni le droit de noter
 * ni celui de décider n'a pas à lire les délibérations. `submitter` est le fil
 * partagé, visible de tous ceux qui accèdent au dossier.
 */
function commentsVisibleTo(
  proposalId: Uuid,
  viewerId: Uuid | null,
  isCommittee: boolean,
): ProposalComment[] {
  return [...proposalComments, ...sessionComments]
    .filter((comment) => comment.proposal_id === proposalId && comment.deleted_at === null)
    .filter((comment) => {
      if (comment.visibility === 'private') return comment.author_id === viewerId
      if (comment.visibility === 'committee') return isCommittee
      return true
    })
    .sort((a, b) => a.created_at.localeCompare(b.created_at))
}

/**
 * `programme.refresh_proposal_score()` — la part « agrégats du dossier ».
 *
 * SEULES LES REVUES SOUMISES COMPTENT, et une note de zéro sur un critère
 * ÉLIMINATOIRE lève `is_knocked_out` pour le dossier entier. Ce sont les deux
 * règles que la fonction applique en base, et les afficher autrement ferait
 * diverger l'en-tête de la liste dont il vient.
 */
function refreshedAggregates(
  proposalReviews: Review[],
  criteria: ReviewCriterion[],
): Pick<Proposal, 'average_score' | 'weighted_score' | 'review_count' | 'is_knocked_out'> {
  const submitted = proposalReviews.filter((review) => review.submitted_at !== null)
  const knockoutIds = new Set(criteria.filter((criterion) => criterion.is_knockout).map((c) => c.id))
  const average = (values: number[]): number | null =>
    values.length === 0
      ? null
      : Math.round((values.reduce((sum, value) => sum + value, 0) / values.length) * 100) / 100

  return {
    average_score: average(submitted.map((review) => review.score_out_of_20 ?? 0)),
    weighted_score: average(submitted.map((review) => review.weighted_score ?? 0)),
    review_count: submitted.length,
    is_knocked_out: submitted.some((review) =>
      scoresOf(review.id).some((score) => score.score === 0 && knockoutIds.has(score.criterion_id)),
    ),
  }
}

/**
 * L'HISTORIQUE DE PARTICIPATION D'UNE ORGANISATION —
 * `analytics.mv_organization_scorecard` rejouée sur les seules colonnes dont la
 * fiche d'évaluation a besoin.
 *
 * POURQUOI CETTE PROJECTION ET PAS UN COMPTE À LA VOLÉE : la question que se
 * pose un membre du comité — « cette organisation a-t-elle déjà tenu ce qu'elle
 * annonce ? » — se répond sur TOUTES les éditions, pas sur celle qu'il regarde.
 * La projection est faite pour cela, et la calculer ici comme elle est calculée
 * là-bas évite deux réponses différentes à la même question.
 */
export function organizationTrackRecord(organizationId: Uuid): OrganizationTrackRecord | null {
  const own = allProposals.filter(
    (proposal) => proposal.organization_id === organizationId && proposal.deleted_at === null,
  )
  if (own.length === 0) return null

  const deposees = own.filter((proposal) => proposal.submitted_at !== null)
  const acceptees = own.filter((proposal) => proposal.status === 'accepted')
  const rejetees = own.filter((proposal) => proposal.status === 'rejected')
  const notes = own
    .map((proposal) => proposal.average_score)
    .filter((score): score is number => score !== null)

  return {
    organization_id: organizationId,
    propositions_deposees: deposees.length,
    propositions_acceptees: acceptees.length,
    propositions_rejetees: rejetees.length,
    evenements_couverts: new Set(deposees.map((proposal) => proposal.event_id)).size,
    sessions_realisees: allSessions.filter(
      (session) => session.organization_id === organizationId && session.status === 'completed',
    ).length,
    note_moyenne_obtenue:
      notes.length === 0
        ? null
        : Math.round((notes.reduce((sum, score) => sum + score, 0) / notes.length) * 100) / 100,
    // NUL et non zéro quand rien n'a jamais été déposé : c'est la règle de la
    // vue, et un zéro se lirait « jamais retenue ».
    ratio_acceptation:
      deposees.length === 0
        ? null
        : Math.round((acceptees.length / deposees.length) * 10000) / 10000,
    derniere_proposition:
      deposees
        .map((proposal) => proposal.submitted_at as string)
        .sort((a, b) => b.localeCompare(a))[0] ?? null,
  }
}

// ---------------------------------------------------------------------------
// L'écran
// ---------------------------------------------------------------------------

/**
 * TOUT L'ÉCRAN EN UNE RÉPONSE.
 *
 * L'OUVERTURE POSE UN ACCUSÉ DE LECTURE, comme le ferait
 * `programme.record_proposal_read()` : c'est ce qui alimente le « lu par 3
 * membres du comité » de la liste et l'indicateur « non consulté ». La réponse
 * porte l'état d'AVANT (`first_visit`), sans quoi l'écran ne pourrait jamais
 * dire « vous ouvrez ce dossier pour la première fois ».
 */
export function reviewDesk(proposalId: Uuid, personId: Uuid | null): ReviewDeskScreen | null {
  const base = allProposals.find((entry) => entry.id === proposalId)
  if (!base || base.deleted_at !== null) return null

  const edition = events.find((event) => event.id === base.event_id)
  if (!edition) return null

  // Le statut peut avoir changé pendant la visite, depuis cet écran ou depuis la
  // liste : les deux écrans partagent le même tampon.
  const proposal = { ...base, status: sessionProposalStatus(proposalId) ?? base.status }
  const call = callsForProposals.find((entry) => entry.id === proposal.call_id) ?? null
  const criteria = call
    ? reviewCriteria
        .filter((criterion) => criterion.call_id === call.id)
        .sort((a, b) => a.sort_order - b.sort_order)
    : []

  const now = Date.now()
  const assignments = assignmentsOf(proposalId)
  const proposalReviews = reviewsOf(proposalId)

  // --- Droits ---------------------------------------------------------------
  const granted = personId ? effectivePermissions(personId) : []
  const can = (code: string): boolean =>
    granted.some(
      (entry) =>
        entry.permission_code === code &&
        (entry.scope_type === 'global' ||
          (entry.scope_type === 'event' && entry.scope_id === proposal.event_id)),
    )

  const myAssignment = assignments.find((entry) => entry.reviewer_id === personId) ?? null
  const permissions: ReviewDeskPermissions = {
    can_review: can('programme.review.write'),
    can_decide: can('programme.proposal.decide'),
    can_assign: can('event.call.manage'),
    is_assigned: myAssignment !== null && myAssignment.recused_at === null,
    is_recused: myAssignment?.recused_at != null,
  }
  const isCommittee = permissions.can_review || permissions.can_decide || permissions.can_assign

  // --- Ma revue -------------------------------------------------------------
  const mine = personId
    ? (proposalReviews.find((review) => review.reviewer_id === personId) ?? null)
    : null
  const mineScores = mine ? scoresOf(mine.id) : []

  const myReview: MyReview = {
    review: mine,
    scores: Object.fromEntries(
      mineScores.map((score) => [score.criterion_id, score.score]),
    ) as Record<CriterionId, Numeric>,
    comments: Object.fromEntries(
      mineScores.filter((score) => score.comment).map((score) => [score.criterion_id, score.comment as string]),
    ) as Record<CriterionId, string>,
    assignment: myAssignment,
  }

  // --- Le voile de l'évaluation en aveugle ----------------------------------
  //
  // Il ne vise QUE celui qui va poser une note et ne l'a pas encore posée : un
  // administrateur qui décide sans noter n'est pas exposé à l'effet d'ancrage,
  // et lui masquer les notes rendrait la décision impossible.
  const blindReview = call?.blind_review ?? false
  const blindVeiled = blindReview && permissions.is_assigned && mine?.submitted_at == null

  const submittedPeers = proposalReviews.filter(
    (review) => review.submitted_at !== null && review.reviewer_id !== personId,
  )

  const peerReviews: PeerReview[] = blindVeiled
    ? []
    : submittedPeers.map((review) => ({
        review,
        scores: scoresOf(review.id),
        reviewer: personById.get(review.reviewer_id) ?? null,
        assignment: assignments.find((entry) => entry.reviewer_id === review.reviewer_id) ?? null,
      }))

  // --- L'avancement du comité ----------------------------------------------
  const committee: CommitteeMemberProgress[] = assignments
    .map((assignment) => {
      const review =
        proposalReviews.find((entry) => entry.reviewer_id === assignment.reviewer_id) ?? null
      return {
        assignment,
        person: personById.get(assignment.reviewer_id) ?? null,
        state: progressState(assignment, review, now),
        submitted_at: review?.submitted_at ?? null,
      }
    })
    .sort((a, b) => (a.person?.display_name ?? '').localeCompare(b.person?.display_name ?? '', 'fr'))

  // --- Le dossier -----------------------------------------------------------
  const organizationEntries: ProposalOrganizationEntry[] = proposalOrganizations
    .filter((link) => link.proposal_id === proposalId)
    .sort((a, b) => (a.role === 'lead' ? -1 : b.role === 'lead' ? 1 : a.sort_order - b.sort_order))
    .map((link) => ({
      link,
      organization: organizationById.get(link.organization_id) ?? null,
      track_record: organizationTrackRecord(link.organization_id),
    }))

  const speakerEntries: ProposalSpeakerEntry[] = proposalSpeakers
    .filter((speaker) => speaker.proposal_id === proposalId)
    .sort((a, b) => a.sort_order - b.sort_order)
    .map((speaker) => ({ speaker, person: personById.get(speaker.person_id) ?? null }))

  const documentEntries: ProposalDocumentEntry[] = proposalDocuments
    .filter((document) => document.proposal_id === proposalId)
    .sort((a, b) => a.sort_order - b.sort_order)
    .map((document) => {
      const asset = assetById.get(document.asset_id) ?? null
      return {
        document,
        asset,
        // `media.object_url()` rejouée : seul un objet `ready` est servi, et
        // l'adresse se compose depuis `(bucket, object_key)`. Une pièce en
        // quarantaine n'a donc pas d'URL — l'écran l'annonce au lieu de
        // proposer un lien mort.
        url: asset && asset.status === 'ready' ? `/mocks/documents/${asset.object_key}` : null,
      }
    })

  const comments = commentsVisibleTo(proposalId, personId, isCommittee)

  const participantIds = new Set<Uuid>([
    ...comments.map((comment) => comment.author_id),
    ...proposalReviews.map((review) => review.reviewer_id),
    ...assignments.map((assignment) => assignment.reviewer_id),
    proposal.submitted_by,
    ...(proposal.contact_person_id ? [proposal.contact_person_id] : []),
    ...(proposal.decided_by ? [proposal.decided_by] : []),
  ])
  const participants: Person[] = people.filter((person) => participantIds.has(person.id))

  // --- Lecture --------------------------------------------------------------
  const readKey = personId ? keyOf(proposalId, personId) : null
  const alreadyRead =
    personId !== null &&
    (sessionReads.has(readKey as string) ||
      proposalReads.some((read) => read.proposal_id === proposalId && read.person_id === personId))
  if (readKey) sessionReads.add(readKey)

  const readCount =
    proposalReads.filter((read) => read.proposal_id === proposalId).length +
    (personId && !alreadyRead ? 1 : 0)

  const dashboardRow = proposalDashboard().find((row) => row.id === proposalId)

  /**
   * LES AGRÉGATS SUIVENT LES REVUES DÉPOSÉES PENDANT LA VISITE.
   *
   * `proposals.average_score`, `weighted_score`, `review_count` et
   * `is_knocked_out` sont DÉNORMALISÉS : la base les recalcule à chaque note par
   * `refresh_proposal_score()`. Les données simulées, elles, sont figées — un
   * membre du comité qui vient de déposer sa revue verrait donc l'en-tête
   * annoncer « 3 revues rendues » à côté d'une moyenne calculée sur deux.
   *
   * Le recalcul n'a lieu QUE si une revue a été écrite pendant cette visite :
   * quatorze dossiers retenus portent leurs agrégats sans que le détail de leurs
   * revues soit écrit (voir l'en-tête de `mocks/reviews.ts`), et recalculer sur
   * un jeu vide les ramènerait tous à zéro.
   */
  const touched = [...sessionReviews.keys()].some((key) => key.startsWith(`${proposalId}:`))
  const scored = touched
    ? refreshedAggregates(proposalReviews, criteria)
    : {
        average_score: proposal.average_score,
        weighted_score: proposal.weighted_score,
        review_count: proposal.review_count,
        is_knocked_out: proposal.is_knocked_out,
      }

  return {
    proposal: { ...proposal, ...scored },
    edition,
    call,
    organizations: organizationEntries,
    speakers: speakerEntries,
    documents: documentEntries,
    themes: dashboardRow?.themes ?? [],
    transitions: [...proposalTransitions, ...sessionTransitions]
      .filter((transition) => transition.proposal_id === proposalId)
      .sort((a, b) => a.occurred_at.localeCompare(b.occurred_at)),
    history: proposalHistory(proposalId),
    criteria,
    max_weighted_score: call ? maxWeightedScoreOf(call.id) : 0,
    required_reviews: call?.required_reviews ?? null,
    blind_review: blindReview,
    blind_veiled: blindVeiled,
    veiled_count: blindVeiled ? submittedPeers.length : 0,
    my_review: myReview,
    peer_reviews: peerReviews,
    committee,
    comments,
    participants,
    permissions,
    rank: dashboardRow?.event_rank ?? 0,
    first_visit: personId !== null && !alreadyRead,
    read_count: readCount,
  }
}

// ---------------------------------------------------------------------------
// Écritures
// ---------------------------------------------------------------------------

/**
 * ENREGISTRER OU DÉPOSER UNE REVUE — puis rejouer
 * `programme.refresh_proposal_score()`.
 *
 * DEUX RÈGLES DE LA BASE SONT REPRODUITES ICI, et l'écran en dépend :
 *  · seules les revues SOUMISES comptent dans les agrégats du dossier. Une
 *    revue enregistrée en brouillon ne bouge ni la moyenne, ni le rang, ni le
 *    « 2/3 » ;
 *  · une note de zéro sur un critère ÉLIMINATOIRE lève `is_knocked_out` pour le
 *    dossier entier, quelle que soit la moyenne.
 */
export function saveReview(
  personId: Uuid,
  payload: SaveReviewPayload,
  at: number = Date.now(),
): SaveReviewResult {
  const proposal = allProposals.find((entry) => entry.id === payload.proposal_id)
  if (!proposal) throw new Error(`Dossier ${payload.proposal_id} introuvable.`)

  const call = callsForProposals.find((entry) => entry.id === proposal.call_id) ?? null
  const criteria = call ? reviewCriteria.filter((criterion) => criterion.call_id === call.id) : []
  const maxWeighted = call ? maxWeightedScoreOf(call.id) : 0

  const key = keyOf(payload.proposal_id, personId)
  const existing =
    sessionReviews.get(key) ??
    reviews.find((review) => review.proposal_id === payload.proposal_id && review.reviewer_id === personId) ??
    null

  const id = existing?.id ?? REVIEW(++reviewSeq)
  const now = new Date(at).toISOString()

  // La note pondérée est CALCULÉE, jamais reçue du formulaire : c'est le trigger
  // qui fait foi en base, et deux calculs séparés divergeraient au premier
  // changement de pondération.
  const weighted = criteria.reduce((total, criterion) => {
    const score = payload.scores[criterion.id]
    return score === undefined || score === null ? total : total + score * criterion.weight
  }, 0)

  const submittedAt = payload.submit ? (existing?.submitted_at ?? now) : existing?.submitted_at ?? null

  const review: Review = {
    id,
    proposal_id: payload.proposal_id,
    reviewer_id: personId,
    recommendation: payload.recommendation,
    weighted_score: submittedAt ? weighted : null,
    score_out_of_20:
      submittedAt && maxWeighted > 0 ? Math.round(((weighted * 20) / maxWeighted) * 100) / 100 : null,
    strengths: payload.strengths,
    weaknesses: payload.weaknesses,
    private_note: payload.private_note,
    submitted_at: submittedAt,
    created_at: existing?.created_at ?? now,
    updated_at: now,
  }

  sessionReviews.set(key, review)
  sessionScores.set(
    id,
    criteria
      .filter((criterion) => payload.scores[criterion.id] !== undefined)
      .map((criterion) => ({
        review_id: id,
        criterion_id: criterion.id,
        score: payload.scores[criterion.id] as number,
        comment: payload.comments[criterion.id]?.trim() || null,
      })),
  )

  // --- refresh_proposal_score() --------------------------------------------
  const submitted = reviewsOf(payload.proposal_id).filter((entry) => entry.submitted_at !== null)
  const knockoutIds = new Set(criteria.filter((criterion) => criterion.is_knockout).map((c) => c.id))
  const average = (values: number[]): number | null =>
    values.length === 0
      ? null
      : Math.round((values.reduce((sum, value) => sum + value, 0) / values.length) * 100) / 100

  return {
    review,
    proposal_weighted_score: average(submitted.map((entry) => entry.weighted_score ?? 0)),
    proposal_average_score: average(submitted.map((entry) => entry.score_out_of_20 ?? 0)),
    review_count: submitted.length,
    is_knocked_out: submitted.some((entry) =>
      scoresOf(entry.id).some((score) => score.score === 0 && knockoutIds.has(score.criterion_id)),
    ),
  }
}

/**
 * SE DÉPORTER D'UN DOSSIER.
 *
 * L'AFFECTATION N'EST PAS SUPPRIMÉE, elle est datée et motivée : c'est ce que
 * `recused_at` et `recusal_reason` conservent, et c'est ce qu'on relit quand une
 * organisation conteste une décision. Une personne du comité qui n'était pas
 * affectée peut aussi se déporter — on crée alors l'affectation déportée, pour
 * que la déclaration existe quelque part.
 */
export function recuseFromProposal(
  personId: Uuid,
  payload: RecusalPayload,
  at: number = Date.now(),
): ReviewAssignment {
  const now = new Date(at).toISOString()
  const key = keyOf(payload.proposal_id, personId)
  sessionRecusals.set(key, { recused_at: now, reason: payload.reason })

  const existing = assignmentsOf(payload.proposal_id).find(
    (assignment) => assignment.reviewer_id === personId,
  )
  if (existing) return existing

  return {
    id: REVIEW_ASSIGNMENT(++assignmentSeq),
    proposal_id: payload.proposal_id,
    reviewer_id: personId,
    assigned_by: null,
    assigned_at: now,
    due_at: null,
    recused_at: now,
    recusal_reason: payload.reason,
  }
}

/**
 * ÉCRIRE SUR LE DOSSIER, AVEC UNE VISIBILITÉ EXPLICITE.
 *
 * UNE DEMANDE DE CORRECTION EST NÉCESSAIREMENT PARTAGÉE : le modèle ne
 * l'interdit pas, mais une demande que le déposant ne verrait pas bloquerait son
 * dossier sans qu'il sache pourquoi. La règle est donc appliquée ici, à la
 * source, plutôt que laissée à la vigilance de l'écran.
 */
export function postProposalComment(
  personId: Uuid,
  payload: PostCommentPayload,
  at: number = Date.now(),
): ProposalComment {
  const comment: ProposalComment = {
    id: PROPOSAL_COMMENT(++commentSeq),
    proposal_id: payload.proposal_id,
    parent_id: payload.parent_id,
    author_id: personId,
    visibility: payload.is_change_request ? 'submitter' : payload.visibility,
    body: payload.body.trim(),
    is_change_request: payload.is_change_request,
    resolved_at: null,
    resolved_by: null,
    edited_at: null,
    deleted_at: null,
    created_at: new Date(at).toISOString(),
  }
  sessionComments.push(comment)
  return comment
}

/**
 * DÉCIDER — retenir, demander des corrections, rejeter.
 *
 * LA MACHINE À ÉTATS N'EST PAS RÉÉCRITE : la transition passe par
 * `changeProposalStatus()`, celle-là même qu'emploie l'action groupée de la
 * liste (A7), qui lit `proposal_transitions_allowed` comme le fait le trigger.
 * Le refus n'est pas une exception mais une réponse — transition impossible
 * depuis cet état, ou motif manquant —, et l'écran la rend comme telle.
 */
export function decideProposal(
  personId: Uuid | null,
  payload: DecisionPayload,
  at: number = Date.now(),
): DecisionResult {
  const proposal = allProposals.find((entry) => entry.id === payload.proposal_id)
  if (!proposal) return { status: 'transition_not_allowed' }

  const from = sessionProposalStatus(payload.proposal_id) ?? proposal.status

  const result = changeProposalStatus(
    {
      proposal_ids: [payload.proposal_id],
      to_status: payload.to_status,
      reason: payload.reason,
    },
    personId,
  )

  if (result.applied.length === 0) {
    return result.skipped[0]?.reason === 'reason_required'
      ? { status: 'reason_required' }
      : { status: 'transition_not_allowed' }
  }

  const now = new Date(at).toISOString()
  const transition: ProposalTransition = {
    id: PROPOSAL_TRANSITION(++transitionSeq),
    proposal_id: payload.proposal_id,
    from_status: from,
    to_status: payload.to_status,
    actor_id: personId,
    reason: payload.reason,
    occurred_at: now,
  }
  sessionTransitions.push(transition)

  return {
    status: 'applied',
    proposal: {
      ...proposal,
      status: payload.to_status,
      // `tg_guard_proposal_status()` date la décision et en retient l'auteur.
      decided_at: ['accepted', 'rejected'].includes(payload.to_status) ? now : proposal.decided_at,
      decided_by: ['accepted', 'rejected'].includes(payload.to_status) ? personId : proposal.decided_by,
      decision_reason: payload.reason ?? proposal.decision_reason,
    },
    transition,
  }
}
