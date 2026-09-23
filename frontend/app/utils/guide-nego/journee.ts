/**
 * « Ma journée » : ses blocs, et le jour qu'elle affiche.
 *
 * **L'ordre des blocs est fixe** (FR-013) et vit ici, pas dans le gabarit : un test
 * le tient sans navigateur, et chaque étape qui remplit un bloc (1, 3a, 3b, 4, 5) le
 * trouve à sa place.
 */
import type { LibraryDocument } from '~/types/negotiation-documents'
import type { Progression, Recent } from './appareil-lecture.ts'
import { jourCivil } from './connexion.ts'

export const BLOCS_DE_MA_JOURNEE = [
  'prochaine-session',
  'changements',
  'trois-agendas',
  'documents',
  'lexique',
] as const

export type BlocDeMaJournee = (typeof BLOCS_DE_MA_JOURNEE)[number]

/**
 * « Jeudi 12 novembre » — **le jour seul, sans fuseau nommé**.
 *
 * Nommer le fuseau de l'appareil tromperait : son identifiant dit « Istanbul » à
 * Antalya, « Lome » et « Ndjamena » sans accent, « Douala » à Yaoundé, et le nom long
 * du moteur dit « heure moyenne de Greenwich » à Dakar. 0c n'affiche aucune heure
 * d'événement ; le fuseau nommé viendra en 3a, du lieu de l'édition.
 */
export function jourLisible(date: Date, locale: string): string {
  const texte = new Intl.DateTimeFormat(locale, { weekday: 'long', day: 'numeric', month: 'long' }).format(date)
  return texte.charAt(0).toLocaleUpperCase(locale) + texte.slice(1)
}

export function memeJour(a: Date, b: Date): boolean {
  return jourCivil(a) === jourCivil(b)
}

export interface DocumentRecent {
  document: LibraryDocument
  /** La page notée pour la version servie ; nulle si aucune, ou pour une autre version. */
  page: number | null
  /** Instant ISO de la dernière lecture : l'ouverture, ou la dernière page notée si elle est plus tardive. */
  lu: string
}

/**
 * Les derniers ouverts sur ce téléphone, croisés avec la bibliothèque gardée : un
 * document dépublié n'y est plus, il ne paraît plus. Un réservé fermé à la personne
 * ne paraît pas : sur un téléphone partagé, il dirait ce qu'une autre a lu.
 */
export function documentsRecents(
  recents: readonly Recent[],
  documents: readonly LibraryDocument[],
  progressionDe: (id: string, version: string) => Progression | null,
): DocumentRecent[] {
  const parId = new Map(documents.map((d) => [d.id, d]))
  return recents.flatMap((recent) => {
    const document = parId.get(recent.id)
    if (!document || (document.restricted && !document.accessible)) return []
    const progression = progressionDe(document.id, document.version)
    const lu = progression && progression.a > recent.a ? progression.a : recent.a
    return [{ document, page: progression?.page ?? null, lu }]
  })
}
