import type { EventEdition } from '~/types/event/edition'
import type { AttachedImage } from '~/types/media'
import type { ProposalStatus } from '~/types/programme/proposal'
import type { ProposalTracking, WorkspaceAction, WorkspaceOverview } from '~/types/organization-workspace'
import { foldText } from './fold-text.ts'
import { isEditionOver } from './proposal-edit.ts'

/**
 * L'ESPACE ORGANISATION, UNE ÉDITION À LA FOIS (28/10).
 *
 * Toutes éditions mêlées, les comptes rendus attendus de la CdP30 reprise de la
 * v1 remplissaient « à faire » et repoussaient les dossiers en cours sous la
 * ligne de flottaison. Chaque onglet porte donc ses dossiers et SES actions ;
 * une édition terminée ne garde qu'un nombre, celui des comptes rendus attendus.
 */
export interface WorkspaceEditionTab {
  edition: EventEdition
  /** Le logo de l'édition, préféré à la vignette carrée. */
  logo: AttachedImage | null
  thumbnail: AttachedImage | null
  proposals: ProposalTracking[]
  actions: WorkspaceAction[]
  over: boolean
  /** Comptes rendus attendus, comptés à part quand l'édition est terminée. */
  missingReports: number
}

export function workspaceEditionTabs(overview: WorkspaceOverview, at: number = Date.now()): WorkspaceEditionTab[] {
  const tabs = new Map<string, WorkspaceEditionTab>()
  const editionOfProposal = new Map<string, string>()

  for (const tracking of overview.proposals) {
    editionOfProposal.set(tracking.proposal.id, tracking.edition.id)
    const existing = tabs.get(tracking.edition.id)
    if (existing) {
      existing.proposals.push(tracking)
      existing.logo ??= tracking.edition_logo
      existing.thumbnail ??= tracking.edition_thumbnail
      continue
    }
    tabs.set(tracking.edition.id, {
      edition: tracking.edition,
      logo: tracking.edition_logo,
      thumbnail: tracking.edition_thumbnail,
      proposals: [tracking],
      actions: [],
      over: isEditionOver(tracking.edition, at),
      missingReports: 0,
    })
  }

  // L'édition de l'appel ouvert a son onglet même sans dossier : c'est là qu'on dépose.
  const callEdition = overview.call_edition
  if (callEdition && !tabs.has(callEdition.id)) {
    tabs.set(callEdition.id, {
      edition: callEdition,
      logo: null,
      thumbnail: null,
      proposals: [],
      actions: [],
      over: isEditionOver(callEdition, at),
      missingReports: 0,
    })
  }

  const ordered = [...tabs.values()].sort((a, b) => b.edition.starts_at.localeCompare(a.edition.starts_at))

  const organizationWide: WorkspaceAction[] = []
  for (const action of overview.actions) {
    const editionId = action.proposal_id ? editionOfProposal.get(action.proposal_id) : undefined
    const tab = editionId ? tabs.get(editionId) : undefined
    if (!tab) {
      organizationWide.push(action)
      continue
    }
    if (tab.over && action.kind === 'session_report_missing') tab.missingReports += 1
    else tab.actions.push(action)
  }

  // Une demande d'adhésion n'appartient à aucune édition : elle suit l'onglet vivant.
  const living = ordered.filter((tab) => !tab.over)
  for (const tab of living.length > 0 ? living : ordered.slice(0, 1)) tab.actions.push(...organizationWide)

  return ordered
}

/** L'onglet ouvert à l'arrivée : l'édition demandée, sinon la plus récente non terminée. */
export function defaultEditionTab(tabs: WorkspaceEditionTab[], requested?: string | null): WorkspaceEditionTab | null {
  return (
    (requested ? tabs.find((tab) => tab.edition.slug === requested) : undefined) ??
    tabs.find((tab) => !tab.over) ??
    tabs[0] ??
    null
  )
}

export type WorkspaceProposalFilter = 'all' | 'todo' | 'review' | 'accepted' | 'closed' | 'report'

export const WORKSPACE_PROPOSAL_FILTERS: WorkspaceProposalFilter[] = ['all', 'todo', 'review', 'accepted', 'closed', 'report']

const REVIEW: ProposalStatus[] = ['submitted', 'under_review']
const CLOSED: ProposalStatus[] = ['rejected', 'withdrawn', 'cancelled']

/** Ce que le filtre et la recherche ont besoin de savoir des actions, par dossier. */
export interface ProposalActionIndex {
  pending: Set<string>
  missingReports: Map<string, number>
}

export function proposalActionIndex(actions: WorkspaceAction[]): ProposalActionIndex {
  const pending = new Set<string>()
  const missingReports = new Map<string, number>()
  for (const action of actions) {
    if (!action.proposal_id) continue
    if (action.kind === 'session_report_missing') {
      missingReports.set(action.proposal_id, (missingReports.get(action.proposal_id) ?? 0) + 1)
    } else {
      pending.add(action.proposal_id)
    }
  }
  return { pending, missingReports }
}

export function matchesProposalFilter(
  tracking: ProposalTracking,
  filter: WorkspaceProposalFilter,
  index: ProposalActionIndex,
): boolean {
  const { id, status } = tracking.proposal
  switch (filter) {
    case 'all':
      return true
    case 'todo':
      return index.pending.has(id) || index.missingReports.has(id)
    case 'review':
      return REVIEW.includes(status)
    case 'accepted':
      return status === 'accepted'
    case 'closed':
      return CLOSED.includes(status)
    case 'report':
      return index.missingReports.has(id)
  }
}

/** Titre, dans toutes ses langues, et numéro de dossier ; sans accents ni casse. */
export function matchesProposalQuery(tracking: ProposalTracking, query: string): boolean {
  const wanted = foldText(query.trim())
  if (!wanted) return true
  const titles = Object.values(tracking.proposal.title ?? {}).filter((value): value is string => typeof value === 'string')
  return foldText([...titles, tracking.proposal.reference_code].join(' ')).includes(wanted)
}
