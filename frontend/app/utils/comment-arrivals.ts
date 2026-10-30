import type { ProposalCommentView } from '~/types/programme/proposal'
import type { Uuid } from '~/types/shared'

/**
 * Les messages arrivés depuis la lecture précédente, écrits par quelqu'un
 * d'autre et pas encore lus : ce qui mérite une notification brève quand la
 * fenêtre des échanges est fermée.
 */
export function commentArrivals(
  known: ReadonlySet<Uuid>,
  comments: readonly ProposalCommentView[],
  viewerId: Uuid | null | undefined,
): ProposalCommentView[] {
  return comments.filter((comment) => !known.has(comment.id) && !comment.is_read && comment.author_id !== viewerId)
}
