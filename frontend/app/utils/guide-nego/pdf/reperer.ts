/**
 * Retrouver sur la page du PDF un passage de la recherche (R7, ADR-022). Le passage
 * vient de la forme lisible ; la page, des éléments de la couche de texte de pdf.js.
 */
import { normaliserLeCherche, normaliserPourReperer, occurrences, type Position, type TexteNormalise } from './normaliser.ts'

export interface PageDeTexte {
  page: number
  chaines: readonly string[]
}

/** De la première lettre à la position qui suit la dernière. */
export interface Intervalle {
  debut: Position
  fin: Position
}

export interface PassageCherche {
  /** La page que l'index de la forme lisible donne au passage. */
  page: number
  /** Les mots qui l'entourent dans la forme lisible : l'extrait de la recherche. */
  contexte: { avant: string; apres: string }
  expression: string
  /**
   * Le rang de l'expression parmi ses occurrences sur la page de la forme lisible. Une
   * note n'en a pas : le passage cité n'est qu'un texte.
   */
  rang?: { occurrence: number; total: number }
}

export type Reperage =
  | { issue: 'trouve'; page: number; courant: Intervalle; autres: Intervalle[] }
  | { issue: 'ambigu'; page: number; occurrences: Intervalle[] }
  | { issue: 'introuvable' }

/** La page de l'index, puis la suivante, puis la précédente : les deux textes ne coupent pas les pages au même endroit. */
export function pagesAInterroger(page: number, total: number): number[] {
  return [page, page + 1, page - 1].filter((p) => p >= 1 && p <= total)
}

function intervalle({ table }: TexteNormalise, debut: number, longueur: number): Intervalle {
  const derniere = table[debut + longueur - 1] as Position
  return { debut: table[debut] as Position, fin: { element: derniere.element, caractere: derniere.caractere + 1 } }
}

/**
 * Le contexte d'abord, puis l'expression si elle est seule sur la page, puis son rang si
 * les deux textes en comptent autant sur la page de l'index. Chaque page se lit
 * en deux temps : telle quelle, puis sans puces, numéros ni appels de note — le second
 * temps seul perdrait « Autres³⁷ ». Jamais un autre endroit marqué plein (FR-015).
 */
export function repererPassage(pages: readonly PageDeTexte[], cherche: PassageCherche): Reperage {
  const expression = normaliserLeCherche(cherche.expression)
  if (!expression) return { issue: 'introuvable' }
  const avant = normaliserLeCherche(cherche.contexte.avant)
  const avecContexte = avant + expression + normaliserLeCherche(cherche.contexte.apres)

  const lectures: Array<{ page: number; normalise: TexteNormalise }> = []
  for (const numero of pagesAInterroger(cherche.page, Number.POSITIVE_INFINITY)) {
    const page = pages.find((p) => p.page === numero)
    if (!page) continue
    for (const sansMarques of [false, true]) {
      lectures.push({ page: numero, normalise: normaliserPourReperer(page.chaines, { sansMarques }) })
    }
  }

  if (avecContexte.length > expression.length) {
    for (const { page, normalise } of lectures) {
      const [seule, ...autresContextes] = occurrences(normalise.texte, avecContexte)
      if (seule === undefined || autresContextes.length) continue
      const debut = seule + avant.length
      const autres = occurrences(normalise.texte, expression)
        .filter((i) => i !== debut)
        .map((i) => intervalle(normalise, i, expression.length))
      return { issue: 'trouve', page, courant: intervalle(normalise, debut, expression.length), autres }
    }
  }

  for (const { page, normalise } of lectures) {
    const trouvees = occurrences(normalise.texte, expression)
    if (!trouvees.length) continue
    const liste = trouvees.map((i) => intervalle(normalise, i, expression.length))
    if (liste.length === 1) return { issue: 'trouve', page, courant: liste[0] as Intervalle, autres: [] }
    // Mesuré sur les 43 ambigus du guide : les deux ordres de lecture concordent, le rang les place tous.
    const rang = cherche.rang
    if (rang && page === cherche.page && liste.length === rang.total) {
      const autres = liste.filter((_, i) => i !== rang.occurrence)
      return { issue: 'trouve', page, courant: liste[rang.occurrence] as Intervalle, autres }
    }
    return { issue: 'ambigu', page, occurrences: liste }
  }
  return { issue: 'introuvable' }
}

const MOTS_DE_TETE = 8

/**
 * Le passage cité d'une note, sur sa seule page : entier, puis par ses huit premiers mots
 * (10 sur 10 à l'essai). Répété, il se place à sa première occurrence ; introuvable, la
 * note va en tête de page (FR-032).
 */
export function repererLaNote(page: PageDeTexte, passage: string | null): Intervalle | null {
  const mots = passage?.trim().split(/\s+/) ?? []
  if (!passage || !mots[0]) return null
  const essais = mots.length > MOTS_DE_TETE ? [passage, mots.slice(0, MOTS_DE_TETE).join(' ')] : [passage]
  for (const expression of essais) {
    const r = repererPassage([page], { page: page.page, contexte: { avant: '', apres: '' }, expression })
    if (r.issue === 'trouve') return r.courant
    if (r.issue === 'ambigu') return r.occurrences[0] ?? null
  }
  return null
}
