/**
 * La recherche du lexique et de la FAQ, sur le téléphone, sans réseau (R4).
 *
 * L'index se prépare une fois, à la lecture du savoir ; chaque frappe ne fait
 * ensuite que comparer. Rangs, du meilleur au moins bon : égalité, début du champ
 * ou d'un de ses mots, sous-chaîne (ou tous les mots saisis en début de mots du
 * champ), puis approché — chaque mot saisi trouve un mot de l'index à une distance
 * de Damerau-Levenshtein d'au plus 1, 2 pour un mot de huit lettres ou plus.
 */
import { normaliserTerme } from './lexique.ts'

/** Du plus au moins parlant : un terme ou une question, une traduction, un corps de réponse. */
export type NatureDuChamp = 'nom' | 'traduction' | 'corps'

export interface ACherche<T> {
  valeur: T
  /** Lexique : terme, sigle, variantes. FAQ : la question. */
  noms: readonly (string | null)[]
  traductions?: readonly (string | null)[]
  /** FAQ : la réponse. */
  corps?: readonly (string | null)[]
}

export type Rang = 1 | 2 | 3 | 4

export interface Trouve<T> {
  valeur: T
  rang: Rang
  nature: NatureDuChamp
}

export interface ResultatsFlous<T> {
  trouves: Trouve<T>[]
  /** Rien aux rangs 1 à 3, mais un résultat approché : « Vous cherchiez peut-être ». */
  vousCherchiezPeutEtre: boolean
  /** Un résultat vient d'une traduction : « Aussi dans les traductions françaises ». */
  aussiDansLesTraductions: boolean
}

const NATURES: readonly NatureDuChamp[] = ['nom', 'traduction', 'corps']
const BIT: Record<NatureDuChamp, number> = { nom: 1, traduction: 2, corps: 4 }

interface Champ {
  texte: string
  nature: NatureDuChamp
}

interface Fiche<T> {
  valeur: T
  champs: Champ[]
}

export interface IndexFlou<T> {
  fiches: Fiche<T>[]
  /** Chaque mot de l'index, et par fiche les natures des champs où il paraît. */
  mots: Map<string, Map<number, number>>
}

export function preparerIndex<T>(entrees: readonly ACherche<T>[]): IndexFlou<T> {
  const mots = new Map<string, Map<number, number>>()
  const fiches = entrees.map((entree, i) => {
    const champs: Champ[] = []
    for (const nature of NATURES) {
      const textes = nature === 'nom' ? entree.noms : nature === 'traduction' ? entree.traductions : entree.corps
      for (const brut of textes ?? []) {
        const texte = brut ? normaliserTerme(brut) : ''
        if (!texte) continue
        champs.push({ texte, nature })
        for (const mot of texte.split(' ')) {
          let parFiche = mots.get(mot)
          if (!parFiche) mots.set(mot, (parFiche = new Map()))
          parFiche.set(i, (parFiche.get(i) ?? 0) | BIT[nature])
        }
      }
    }
    return { valeur: entree.valeur, champs }
  })
  return { fiches, mots }
}

// Deux lignes réutilisées : la recherche ne crée rien à chaque mot comparé.
let ligneA = new Int32Array(64)
let ligneB = new Int32Array(64)
let ligneC = new Int32Array(64)

/** Distance de Damerau-Levenshtein restreinte, abandonnée dès que `seuil` est dépassé. */
export function distanceBornee(a: string, b: string, seuil: number): number {
  if (Math.abs(a.length - b.length) > seuil) return seuil + 1
  const n = b.length
  if (ligneA.length <= n) {
    ligneA = new Int32Array(n + 1)
    ligneB = new Int32Array(n + 1)
    ligneC = new Int32Array(n + 1)
  }
  let avantDerniere = ligneA
  let derniere = ligneB
  let courante = ligneC
  for (let j = 0; j <= n; j += 1) derniere[j] = j
  for (let i = 1; i <= a.length; i += 1) {
    courante[0] = i
    let minimum = i
    for (let j = 1; j <= n; j += 1) {
      const cout = a.charCodeAt(i - 1) === b.charCodeAt(j - 1) ? 0 : 1
      let d = Math.min((derniere[j] as number) + 1, (courante[j - 1] as number) + 1, (derniere[j - 1] as number) + cout)
      if (i > 1 && j > 1 && a[i - 1] === b[j - 2] && a[i - 2] === b[j - 1]) {
        d = Math.min(d, (avantDerniere[j - 2] as number) + 1)
      }
      courante[j] = d
      if (d < minimum) minimum = d
    }
    if (minimum > seuil) return seuil + 1
    const libre = avantDerniere
    avantDerniere = derniere
    derniere = courante
    courante = libre
  }
  return derniere[n] as number
}

const seuilPour = (mot: string): number => (mot.length >= 8 ? 2 : mot.length >= 3 ? 1 : 0)

/** Rang exact, début ou sous-chaîne d'un champ ; nul s'il ne tombe dans aucun des trois. */
function rangDuChamp(champ: string, requete: string, motsRequete: string[]): Rang | null {
  if (champ === requete) return 1
  if (champ.startsWith(requete) || champ.includes(` ${requete}`)) return 2
  if (champ.includes(requete)) return 3
  const motsChamp = champ.split(' ')
  return motsRequete.every((m) => motsChamp.some((c) => c.startsWith(m))) ? 3 : null
}

/** Pour chaque mot saisi, les fiches qui ont un mot proche, et la nature de leur champ. */
function approches<T>(index: IndexFlou<T>, motsRequete: string[]): Map<number, number> | null {
  let retenues: Map<number, number> | null = null
  for (const saisi of motsRequete) {
    const seuil = seuilPour(saisi)
    const ici = new Map<number, number>()
    for (const [mot, parFiche] of index.mots) {
      if (!(mot.startsWith(saisi) || (seuil > 0 && distanceBornee(saisi, mot, seuil) <= seuil))) continue
      for (const [fiche, natures] of parFiche) ici.set(fiche, (ici.get(fiche) ?? 0) | natures)
    }
    const precedentes: Map<number, number> | null = retenues
    if (precedentes === null) retenues = ici
    else {
      const communes = new Map<number, number>()
      for (const [fiche, natures] of ici) {
        const avant = precedentes.get(fiche)
        if (avant !== undefined) communes.set(fiche, avant & natures || avant | natures)
      }
      retenues = communes
    }
    if (retenues.size === 0) return retenues
  }
  return retenues
}

const natureDe = (bits: number): NatureDuChamp => NATURES.find((n) => bits & BIT[n]) ?? 'corps'

export function chercher<T>(index: IndexFlou<T>, saisie: string): ResultatsFlous<T> {
  const requete = normaliserTerme(saisie)
  if (!requete) return { trouves: [], vousCherchiezPeutEtre: false, aussiDansLesTraductions: false }
  const motsRequete = requete.split(' ')

  const classes: { trouve: Trouve<T>; longueur: number; ordre: number }[] = []
  const vues = new Set<number>()
  index.fiches.forEach((fiche, ordre) => {
    let meilleur: { rang: Rang; nature: NatureDuChamp; longueur: number } | null = null
    for (const champ of fiche.champs) {
      const rang = rangDuChamp(champ.texte, requete, motsRequete)
      if (rang === null) continue
      const mieux =
        !meilleur ||
        rang < meilleur.rang ||
        (rang === meilleur.rang && NATURES.indexOf(champ.nature) < NATURES.indexOf(meilleur.nature))
      if (mieux) meilleur = { rang, nature: champ.nature, longueur: champ.texte.length }
    }
    if (!meilleur) return
    vues.add(ordre)
    classes.push({ trouve: { valeur: fiche.valeur, rang: meilleur.rang, nature: meilleur.nature }, longueur: meilleur.longueur, ordre })
  })

  let approchees = 0
  for (const [ordre, natures] of approches(index, motsRequete) ?? []) {
    if (vues.has(ordre)) continue
    const fiche = index.fiches[ordre] as Fiche<T>
    const nature = natureDe(natures)
    const longueur = fiche.champs.find((c) => c.nature === nature)?.texte.length ?? 0
    classes.push({ trouve: { valeur: fiche.valeur, rang: 4, nature }, longueur, ordre })
    approchees += 1
  }

  classes.sort(
    (a, b) =>
      a.trouve.rang - b.trouve.rang ||
      NATURES.indexOf(a.trouve.nature) - NATURES.indexOf(b.trouve.nature) ||
      a.longueur - b.longueur ||
      a.ordre - b.ordre,
  )
  const trouves = classes.map((c) => c.trouve)
  return {
    trouves,
    vousCherchiezPeutEtre: approchees > 0 && approchees === trouves.length,
    aussiDansLesTraductions: trouves.some((t) => t.nature === 'traduction'),
  }
}
