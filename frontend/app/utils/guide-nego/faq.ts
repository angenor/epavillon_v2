/**
 * La FAQ sur le téléphone : où mène une source, la date de vérification lisible, et
 * la lecture comptée une fois par entrée, par jour et par téléphone (R12).
 */
import type { FaqEntry, KnowledgeSource } from '../../types/negotiation-savoir.ts'

export type Destination = { interne: string } | { externe: string } | null

/** Un document s'ouvre dans le lecteur à la page citée (FR-014) ; une référence, à son adresse. */
export function destinationDe(source: KnowledgeSource): Destination {
  if (source.document_id) {
    const page = source.page_from === undefined ? '' : `?page=${source.page_from}`
    return { interne: `/guide-nego/ressources/documents/${source.document_id}/lire${page}` }
  }
  return source.external_url ? { externe: source.external_url } : null
}

/** `verified_on` est un jour, sans heure : lu en UTC, il ne glisse pas d'un jour selon le fuseau. */
export function jourDeVerification(jour: string, locale: string): string {
  return new Intl.DateTimeFormat(locale, { day: 'numeric', month: 'long', year: 'numeric', timeZone: 'UTC' }).format(
    new Date(`${jour}T00:00:00Z`),
  )
}

/** Les liens arrivent entiers : seuls restent ceux que le téléphone a. */
export function liees(entree: Pick<FaqEntry, 'id' | 'related_ids'>, toutes: readonly FaqEntry[]): FaqEntry[] {
  return entree.related_ids.flatMap((id) => (id === entree.id ? [] : (toutes.find((e) => e.id === id) ?? [])))
}

interface LecturesDuJour {
  jour: string
  ids: string[]
}

/**
 * Faut-il compter cette ouverture ? Rend aussi la valeur à garder : la liste du jour,
 * remise à zéro au changement de jour. Une garde illisible vaut une garde vide.
 */
export function lectureACompter(gardee: string | null, id: string, jour: string): { compter: boolean; garder: string } {
  let lues: LecturesDuJour = { jour, ids: [] }
  try {
    const lu: unknown = gardee ? JSON.parse(gardee) : null
    if (lu && typeof lu === 'object' && 'jour' in lu && 'ids' in lu && lu.jour === jour && Array.isArray(lu.ids)) {
      lues = { jour, ids: lu.ids.filter((x): x is string => typeof x === 'string') }
    }
  } catch {
    // garde illisible : on repart du jour
  }
  if (lues.ids.includes(id)) return { compter: false, garder: JSON.stringify(lues) }
  return { compter: true, garder: JSON.stringify({ jour, ids: [...lues.ids, id] }) }
}
