import type { UnreadCommentCount } from '~/types/admin-proposals'
import type { CommentReader, CommentReceipts, ProposalComment } from '~/types/programme/proposal'
import type { ProposalId, Uuid } from '~/types/shared'

/** Ce que l'auteur lit sous son message : « Lu par 2 sur 3 », et l'organisation à part. */
export interface ReceiptSummary {
  /** Lecteurs du comité, un par personne, du premier au dernier. */
  committee: CommentReader[]
  /** Dénominateur du « sur M » ; jamais inférieur au nombre de lecteurs. */
  audience: number
  /** Première lecture côté organisation. */
  organisation: CommentReader | null
}

function earliestByPerson(readers: CommentReader[]): CommentReader[] {
  const byPerson = new Map<string, CommentReader>()
  for (const reader of readers) {
    const known = byPerson.get(reader.person_id)
    if (!known || reader.read_at < known.read_at) byPerson.set(reader.person_id, reader)
  }
  return [...byPerson.values()].sort((a, b) => a.read_at.localeCompare(b.read_at))
}

export function summarizeReceipts(receipts: CommentReceipts | null): ReceiptSummary | null {
  if (!receipts) return null
  const committee = earliestByPerson(receipts.readers.filter((reader) => reader.side === 'committee'))
  const organisation = earliestByPerson(receipts.readers.filter((reader) => reader.side === 'organisation'))
  return {
    committee,
    // Une personne sortie du comité après avoir lu reste un lecteur : « 3 sur 2 » serait absurde.
    audience: Math.max(receipts.audience_count, committee.length),
    organisation: organisation[0] ?? null,
  }
}

/** Compteurs de la liste (A7), réduits aux dossiers qui ont au moins un message non lu. */
export function unreadCommentMap(entries: UnreadCommentCount[] | null | undefined): Map<ProposalId, number> {
  const map = new Map<ProposalId, number>()
  for (const entry of entries ?? []) {
    if (entry.count > 0) map.set(entry.proposal_id, (map.get(entry.proposal_id) ?? 0) + entry.count)
  }
  return map
}

/**
 * Racines dans l'ordre d'écriture, chacune avec toutes ses réponses : une
 * réponse à une réponse (`parent_id` vers une réponse) reste dans son fil.
 */
export function groupThreads<T extends Pick<ProposalComment, 'id' | 'parent_id' | 'created_at'>>(
  comments: T[],
): { root: T; replies: T[] }[] {
  const byId = new Map(comments.map((comment) => [comment.id, comment]))
  const rootIdOf = (comment: T): Uuid => {
    let current = comment
    const seen = new Set<Uuid>()
    while (current.parent_id && byId.has(current.parent_id) && !seen.has(current.id)) {
      seen.add(current.id)
      current = byId.get(current.parent_id) as T
    }
    return current.id
  }
  const sorted = [...comments].sort((a, b) => a.created_at.localeCompare(b.created_at))
  const threads = new Map<Uuid, { root: T; replies: T[] }>()
  for (const comment of sorted) {
    if (comment.parent_id === null) threads.set(comment.id, { root: comment, replies: [] })
  }
  for (const comment of sorted) {
    if (comment.parent_id === null) continue
    threads.get(rootIdOf(comment))?.replies.push(comment)
  }
  return [...threads.values()]
}
