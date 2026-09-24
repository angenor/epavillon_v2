/**
 * Le texte de PDFium (forme lisible, index de recherche) et celui de la couche de pdf.js
 * ne se découpent ni ne s'écrivent pareil (R7) : on les ramène à une même forme avant de
 * comparer, en gardant pour chaque lettre l'élément et le caractère d'où elle vient.
 */
import { plierLettre } from '../repli.ts'

export interface Position {
  element: number
  caractere: number
}

export interface TexteNormalise {
  texte: string
  /** `table[i]` : d'où vient la lettre `texte[i]`. */
  table: Position[]
}

// Blancs, tirets et guillemets tombent : pdf.js recolle ses éléments avec ou sans espace,
// garde ou non le trait d'union d'une fin de ligne, isole un guillemet sur son élément.
const ECARTEES = new Set(['-', '"'])

// Ce que PDFium omet et que pdf.js garde, seul sur son élément : une puce (Wingdings
// compris, en zone privée ; « o », la puce de second niveau de Word), un numéro de
// liste, un appel de note.
const MARQUE = /^\s*(?:[▪•◦‣·■□●○o--]|\d{1,3}\.?|[a-z]\))\s*$/u

export function normaliserPourReperer(chaines: readonly string[], options: { sansMarques?: boolean } = {}): TexteNormalise {
  let texte = ''
  const table: Position[] = []
  chaines.forEach((chaine, element) => {
    if (options.sansMarques && MARQUE.test(chaine)) return
    for (let caractere = 0; caractere < chaine.length; caractere += 1) {
      const lettre = chaine[caractere] as string
      if (/\s/u.test(lettre)) continue
      for (const c of plierLettre(lettre)) {
        if (ECARTEES.has(c)) continue
        texte += c
        table.push({ element, caractere })
      }
    }
  })
  return { texte, table }
}

export const normaliserLeCherche = (texte: string): string => normaliserPourReperer([texte]).texte

/** Les débuts de chaque occurrence, sans chevauchement. */
export function occurrences(texte: string, cherche: string): number[] {
  const trouvees: number[] = []
  for (let i = texte.indexOf(cherche); i !== -1; i = texte.indexOf(cherche, i + cherche.length)) trouvees.push(i)
  return trouvees
}
