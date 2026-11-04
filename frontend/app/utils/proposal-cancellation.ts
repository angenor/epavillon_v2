import type { ProposalTracking } from '~/types/organization-workspace'
import type { ProposalHistoryEntry, ProposalTransition } from '~/types/programme/proposal'
import type { IsoDateTime, PersonId } from '~/types/shared'
import { isEditionOver } from './proposal-edit.ts'

/**
 * L'ANNULATION D'UNE ACTIVITÉ PAR SON ORGANISATION (04/11).
 *
 * Possible tant qu'aucune séance n'a commencé, sur un dossier retenu d'une
 * édition pas encore finie. L'API tranche de toute façon : l'écran ne fait que
 * ne pas proposer un geste qu'elle refuserait.
 */
export function canOrganizationCancel(tracking: ProposalTracking, at: number = Date.now()): boolean {
  if (tracking.proposal.status !== 'accepted') return false
  if (isEditionOver(tracking.edition, at)) return false
  return tracking.sessions.every(
    (tracked) => tracked.session.status === 'cancelled' || Date.parse(tracked.session.starts_at) > at,
  )
}

export interface ProposalCancellation {
  at: IsoDateTime
  actorId: PersonId | null
  /** Le nom que l'historique donne à l'auteur ; nul s'il ne le dit pas. */
  actorLabel: string | null
  reason: string | null
  /** Annulé après avoir été retenu : l'activité a quitté le programme, inscrits et équipe prévenus. */
  afterAcceptance: boolean
}

/** Qui a annulé, quand et pourquoi : le journal des transitions, nommé par l'historique. */
export function proposalCancellation(
  transitions: ProposalTransition[],
  history: ProposalHistoryEntry[],
): ProposalCancellation | null {
  const last = [...transitions]
    .sort((a, b) => a.occurred_at.localeCompare(b.occurred_at))
    .reverse()
    .find((transition) => transition.to_status === 'cancelled')
  if (!last) return null
  const named = history
    .filter((entry) => entry.field === 'status' && entry.new_value === 'cancelled')
    .sort((a, b) => b.occurred_at.localeCompare(a.occurred_at))[0]
  return {
    at: last.occurred_at,
    actorId: last.actor_id,
    actorLabel: named?.actor_label ?? null,
    reason: last.reason?.trim() || null,
    afterAcceptance: last.from_status === 'accepted',
  }
}
