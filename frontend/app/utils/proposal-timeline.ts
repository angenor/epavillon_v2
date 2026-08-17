/**
 * COMPOSITION DE LA FRISE D'AVANCEMENT D'UN DOSSIER.
 *
 * `UiStatusTimeline` ne déduit rien : elle reçoit ses étapes, dans l'ordre, avec
 * leur état et leur date. C'est ici qu'elles se composent, et cette fonction est
 * PURE — aucun accès au réseau, aucune traduction, aucun fuseau. Elle est donc
 * la même pour la carte de la liste et pour la fiche du dossier, qui n'affichent
 * pourtant pas la frise de la même façon.
 *
 * LES DATES VIENNENT DU JOURNAL, PAS DE LA PROPOSITION. `proposal_transitions`
 * est écrit par le trigger `tg_guard_proposal_status()` à chaque franchissement :
 * il porte l'instant, l'auteur et le motif. La ligne de la proposition ne garde,
 * elle, que `submitted_at` et `decided_at` — deux dates sur cinq étapes. Lire le
 * journal, c'est aussi rendre visible le chemin NON LINÉAIRE : un dossier
 * renvoyé pour correction, corrigé, puis retenu, a franchi l'évaluation deux
 * fois.
 *
 * LE GRAPHE DES ÉTATS N'EST PAS RÉIMPLÉMENTÉ ICI. Il vit dans la base
 * (`programme.proposal_transitions_allowed`) et nous n'en dérivons qu'un
 * AFFICHAGE : quatre jalons que tout dossier traverse, plus deux qui
 * n'apparaissent que s'ils ont eu lieu. Ajouter un état à l'ENUM ne casse pas
 * cette fonction — il tombe dans la décision, avec son libellé.
 */

import type { TimelineStep } from '~/types/ui'
import type { ProposalStatus } from '~/types/programme/proposal'
import type { ProposalTracking } from '~/types/organization-workspace'

/** Libellés déjà traduits — la frise ne connaît pas i18n. */
export interface ProposalTimelineLabels {
  draft: string
  submitted: string
  under_review: string
  changes_requested: string
  decision: string
  accepted: string
  rejected: string
  withdrawn: string
  cancelled: string
  scheduled: string
}

/** Les quatre issues qui closent le parcours. */
const FINAL_STATUSES: ProposalStatus[] = ['accepted', 'rejected', 'withdrawn', 'cancelled']

/**
 * Libellé de la dernière étape. Tant que le dossier est en cours, elle annonce
 * ce qui vient (« Décision ») ; une fois close, elle NOMME l'issue — « Retenu »,
 * « Non retenu ». Un dossier refusé dont la frise dirait encore « Décision »
 * laisserait chercher une réponse qui a déjà été donnée.
 */
function decisionLabel(status: ProposalStatus, labels: ProposalTimelineLabels): string {
  switch (status) {
    case 'accepted':
      return labels.accepted
    case 'rejected':
      return labels.rejected
    case 'withdrawn':
      return labels.withdrawn
    case 'cancelled':
      return labels.cancelled
    default:
      return labels.decision
  }
}

export function buildProposalTimeline(
  tracking: ProposalTracking,
  labels: ProposalTimelineLabels,
): TimelineStep[] {
  const { proposal, transitions } = tracking

  /** Premier franchissement VERS un état, s'il a eu lieu. */
  const firstArrivalAt = (status: ProposalStatus): string | null =>
    transitions.find((transition) => transition.to_status === status)?.occurred_at ?? null

  /** Dernier franchissement vers un état — celui qui compte pour la décision. */
  const lastArrival = (status: ProposalStatus) =>
    [...transitions].reverse().find((transition) => transition.to_status === status) ?? null

  const steps: TimelineStep[] = []
  const isFinal = FINAL_STATUSES.includes(proposal.status)

  // 1. Ouverture du dossier. Toujours franchie : la ligne existe.
  steps.push({
    value: 'draft',
    label: labels.draft,
    at: firstArrivalAt('draft') ?? proposal.created_at,
    state: proposal.status === 'draft' ? 'current' : 'done',
  })

  // 2. Dépôt.
  steps.push({
    value: 'submitted',
    label: labels.submitted,
    at: proposal.submitted_at,
    state:
      proposal.status === 'draft'
        ? 'upcoming'
        : proposal.status === 'submitted'
          ? 'current'
          : 'done',
  })

  // 3. Évaluation.
  //
  // ELLE EST FRANCHIE DÈS QU'UNE DÉCISION A ÉTÉ RENDUE, que le journal en porte
  // la trace ou non : un dossier retenu a forcément été évalué. Se fier à la
  // seule transition datée affichait « non concerné », barré, sur des dossiers
  // acceptés dont le journal ne remonte pas jusque-là. Seuls un retrait ou une
  // annulation peuvent l'avoir contournée.
  const reviewAt = firstArrivalAt('under_review')
  const wasDecided = proposal.status === 'accepted' || proposal.status === 'rejected'
  steps.push({
    value: 'under_review',
    label: labels.under_review,
    at: reviewAt,
    state:
      proposal.status === 'under_review'
        ? 'current'
        : reviewAt || wasDecided
          ? 'done'
          : isFinal
            ? 'skipped'
            : 'upcoming',
  })

  // 4. Corrections — l'étape n'existe QUE si le dossier est revenu en arrière.
  //    L'afficher toujours laisserait croire que tout dossier en passe par là.
  const changesAt = lastArrival('changes_requested')?.occurred_at ?? null
  if (changesAt) {
    steps.push({
      value: 'changes_requested',
      label: labels.changes_requested,
      at: changesAt,
      state: proposal.status === 'changes_requested' ? 'error' : 'done',
      detail: lastArrival('changes_requested')?.reason ?? null,
    })
  }

  // 5. Décision. Un refus porte TOUJOURS son motif : le guide en fait une règle,
  //    et le trigger l'exige (`requires_reason`).
  const decisionTransition = isFinal ? lastArrival(proposal.status) : null
  const decisionDetail = proposal.decision_reason ?? decisionTransition?.reason ?? null
  steps.push({
    value: 'decision',
    label: decisionLabel(proposal.status, labels),
    at: proposal.decided_at ?? decisionTransition?.occurred_at ?? null,
    state:
      proposal.status === 'accepted'
        ? 'done'
        : proposal.status === 'rejected' || proposal.status === 'cancelled'
          ? 'refused'
          : // Un retrait n'est pas un refus : c'est l'organisation qui a repris
            // sa parole, et la frise le barre plutôt que de l'accuser.
            proposal.status === 'withdrawn'
            ? 'skipped'
            : 'upcoming',
    detail: decisionDetail,
  })

  // 6. Programmation — seulement pour un dossier retenu : c'est la suite qu'il
  //    attend, et elle n'a aucun sens ailleurs.
  if (proposal.status === 'accepted') {
    const firstSession = tracking.sessions[0]
    steps.push({
      value: 'scheduled',
      label: labels.scheduled,
      at: firstSession?.session.starts_at ?? null,
      state: firstSession ? 'done' : 'upcoming',
    })
  }

  return steps
}
