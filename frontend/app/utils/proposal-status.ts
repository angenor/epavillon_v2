import type { ProposalStatus } from '~/types/programme/proposal'
import type { Intent } from '~/types/ui'

/** La couleur d'état d'un dossier, au back-office : retenu vert, corrections jaune, refus rouge. */
export const PROPOSAL_STATUS_INTENT: Record<ProposalStatus, Intent> = {
  draft: 'neutral',
  submitted: 'info',
  under_review: 'info',
  changes_requested: 'warning',
  accepted: 'success',
  rejected: 'danger',
  withdrawn: 'neutral',
  cancelled: 'danger',
}
