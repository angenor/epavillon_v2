/**
 * Données simulées de l'annulation d'une activité par son organisation (04/11),
 * `POST /proposals/{id}/cancel`.
 *
 * Mêmes refus que l'API, pour que la fiche se teste hors ligne : dossier retenu
 * seulement, motif obligatoire, aucune séance commencée. L'annulation vit le
 * temps de la session, par-dessus le jeu écrit à la main : l'espace organisation
 * l'applique au dossier, à son journal et à son historique.
 */

import type { CancelProposalPayload, ProposalHistoryEntry, ProposalTransition } from '~/types/programme/proposal'
import type { ApiErrorCode } from '~/types/api-error'
import type { Uuid } from '~/types/shared'
import { ApiRequestError } from '~/utils/api-error'
import { PROPOSAL_TRANSITION } from '../ids'
import { people } from '../people'
import { editedProposal } from '../proposal-edit'
import { allSessions } from '../sessions'
import { allProposals } from './index'

interface Cancellation {
  transition: ProposalTransition
  history: ProposalHistoryEntry
}

const cancellations = new Map<Uuid, Cancellation>()

function refuse(status: number, code: ApiErrorCode, message: string, field?: string): never {
  throw new ApiRequestError({ code, message, field }, status)
}

/** L'annulation faite pendant la session, ou rien. */
export function sessionCancellation(proposalId: Uuid): Cancellation | null {
  return cancellations.get(proposalId) ?? null
}

export function cancelProposalByOrganization(
  proposalId: Uuid,
  personId: Uuid,
  payload: CancelProposalPayload,
  at: number = Date.now(),
): ProposalTransition {
  const stored = allProposals.find((proposal) => proposal.id === proposalId)
  if (!stored) refuse(404, 'NOT_FOUND', 'Ce dossier est introuvable.')
  const status = cancellations.has(proposalId) ? 'cancelled' : (editedProposal(proposalId)?.status ?? stored.status)
  if (status !== 'accepted') {
    refuse(409, 'CONFLICT', 'Seule une activité retenue peut être annulée par son organisation.')
  }
  const reason = payload.reason.trim()
  if (!reason) refuse(422, 'VALIDATION_FAILED', "Indiquez le motif de l'annulation.", 'reason')
  const started = allSessions.some(
    (session) => session.proposal_id === proposalId && session.status !== 'cancelled' && Date.parse(session.starts_at) <= at,
  )
  if (started) {
    refuse(409, 'CONFLICT', 'Une séance de cette activité a déjà commencé : elle ne peut plus être annulée.')
  }

  const occurred_at = new Date(at).toISOString()
  const transition: ProposalTransition = {
    id: PROPOSAL_TRANSITION(9000 + cancellations.size),
    proposal_id: proposalId,
    from_status: 'accepted',
    to_status: 'cancelled',
    actor_id: personId,
    reason,
    occurred_at,
  }
  cancellations.set(proposalId, {
    transition,
    history: {
      occurred_at,
      actor_id: personId,
      actor_label: people.find((person) => person.id === personId)?.display_name ?? null,
      action: 'update',
      field: 'status',
      old_value: 'accepted',
      new_value: 'cancelled',
    },
  })
  return transition
}
