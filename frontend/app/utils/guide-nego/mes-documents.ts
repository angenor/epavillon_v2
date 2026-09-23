/**
 * « Mes documents » : ce qui est gardé sur ce téléphone, et les favoris du compte.
 *
 * Une copie ne porte pas son titre : elle se croise avec la bibliothèque gardée, qui
 * se lit sans réseau. Une copie que la bibliothèque ne nomme plus sera effacée au
 * prochain rapprochement ; elle ne s'affiche pas, mais sa place compte.
 */
import type { LibraryDocument } from '~/types/negotiation-documents'
import type { Copie } from './copies.ts'

type CopieLue = Pick<Copie, 'id' | 'gardee_a' | 'octets'>

export interface DocumentGarde<C extends CopieLue = CopieLue> {
  document: LibraryDocument
  copie: C
}

/** Le plus récemment gardé d'abord. */
export function surLeTelephone<C extends CopieLue>(copies: readonly C[], documents: readonly LibraryDocument[]): DocumentGarde<C>[] {
  const parId = new Map(documents.map((d) => [d.id, d]))
  return copies
    .flatMap((copie) => {
      const document = parId.get(copie.id)
      return document ? [{ document, copie }] : []
    })
    .sort((a, b) => b.copie.gardee_a.localeCompare(a.copie.gardee_a))
}

export const placeDesCopies = (copies: readonly Pick<Copie, 'octets'>[]): number =>
  copies.reduce((total, c) => total + c.octets, 0)

/** Dans l'ordre de la bibliothèque ; un favori dépublié ne s'affiche pas. */
export const favorisConnus = (ids: ReadonlySet<string>, documents: readonly LibraryDocument[]): LibraryDocument[] =>
  documents.filter((d) => ids.has(d.id))
