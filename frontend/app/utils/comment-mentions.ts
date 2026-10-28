import type { CommentPerson } from '~/types/programme/proposal'
import { normalizeLabelLike } from './slug.ts'

/** Une mention en cours de frappe : de l'arobase au curseur. */
export interface MentionQuery {
  start: number
  end: number
  query: string
}

const MAX_QUERY_LENGTH = 40

/**
 * La mention que le curseur est en train d'écrire, ou rien.
 *
 * L'arobase doit ouvrir un mot : collée à une lettre, c'est une adresse
 * électronique (`contact@ifdd.org`), pas un appel. Le nom peut contenir une
 * espace (« @Awa Bak ») mais pas deux de suite ni un retour à la ligne : on a
 * alors fini de nommer et l'on écrit la phrase.
 */
export function activeMention(text: string, caret: number): MentionQuery | null {
  const before = text.slice(0, caret)
  const at = before.lastIndexOf('@')
  if (at < 0) return null
  const previous = at > 0 ? before[at - 1] : ''
  if (previous && !/[\s([{"'«]/.test(previous)) return null
  const query = before.slice(at + 1)
  if (query.length > MAX_QUERY_LENGTH || /[\n@]/.test(query) || /^\s/.test(query) || /\s\s/.test(query)) {
    return null
  }
  return { start: at, end: caret, query }
}

/**
 * Les personnes dont un mot du nom commence par chaque mot saisi, accents et
 * casse ignorés : « bak » trouve « Awa Bakayoko », « a bak » aussi.
 */
export function matchMentionCandidates(people: CommentPerson[], query: string, limit = 8): CommentPerson[] {
  const words = normalizeLabelLike(query).split(' ').filter(Boolean)
  const matches = words.length === 0
    ? people
    : people.filter((person) => {
        const parts = normalizeLabelLike(person.display_name).split(' ')
        return words.every((word) => parts.some((part) => part.startsWith(word)))
      })
  return matches.slice(0, limit)
}

/** Remplace « @que » par « @Nom complet » suivi d'une espace, et rend la place du curseur. */
export function insertMention(
  text: string,
  mention: MentionQuery,
  displayName: string,
): { text: string; caret: number } {
  const inserted = `@${displayName} `
  const after = text.slice(mention.end).replace(/^ /, '')
  return {
    text: text.slice(0, mention.start) + inserted + after,
    caret: mention.start + inserted.length,
  }
}
