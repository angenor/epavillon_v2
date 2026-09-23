/**
 * La bibliothèque : filtres, recherche et marques d'une ligne, sans Nuxt.
 *
 * Les libellés des types, des thématiques et des COP viennent du vocabulaire que
 * la liste porte (`DocumentLibrary.vocabulary`), jamais d'un fichier de traduction.
 */
import type {
  DocumentLibrary,
  LibraryDocument,
  LibraryVocabulary,
  VocabularyCop,
  VocabularyTerm,
} from '~/types/negotiation-documents'
import type { NomDEtat } from './etats.ts'
import { normalizeSearch } from '../proposal-list.ts'

export type Axe = 'types' | 'themes' | 'cops'

export const AXES: readonly Axe[] = ['types', 'themes', 'cops']

export type Filtres = Record<Axe, string[]>

export const aucunFiltre = (): Filtres => ({ types: [], themes: [], cops: [] })

export interface Critere {
  filtres: Filtres
  recherche: string
  /** Les documents dont le texte contient la recherche, lus en ligne ; nul sans cette lecture. */
  dansLeTexte: ReadonlySet<string> | null
}

function valeursDe(document: LibraryDocument, axe: Axe): string[] {
  if (axe === 'types') return [document.type]
  if (axe === 'themes') return document.themes
  return document.cop ? [document.cop] : []
}

const passe = (document: LibraryDocument, axe: Axe, choix: string[]): boolean =>
  !choix.length || valeursDe(document, axe).some((v) => choix.includes(v))

export function correspondALaRecherche(
  document: LibraryDocument,
  recherche: string,
  dansLeTexte: ReadonlySet<string> | null,
): boolean {
  const cherche = normalizeSearch(recherche)
  if (!cherche) return true
  if (dansLeTexte?.has(document.id)) return true
  return [document.title, document.summary, document.publisher].some((c) => !!c && normalizeSearch(c).includes(cherche))
}

/** Les documents retenus ; `sauf` ignore un axe, pour compter ce que ses valeurs donneraient. */
export function filtrer(documents: LibraryDocument[], critere: Critere, sauf?: Axe): LibraryDocument[] {
  return documents.filter(
    (d) =>
      correspondALaRecherche(d, critere.recherche, critere.dansLeTexte) &&
      AXES.every((axe) => axe === sauf || passe(d, axe, critere.filtres[axe])),
  )
}

/** « Afficher n documents » : ce que donnerait ce brouillon, les autres filtres gardés. */
export function compterAvec(documents: LibraryDocument[], critere: Critere, axe: Axe, brouillon: string[]): number {
  return filtrer(documents, { ...critere, filtres: { ...critere.filtres, [axe]: brouillon } }).length
}

export interface OptionDeFiltre {
  valeur: string
  libelle: string
  /** Les documents de cette valeur, les autres filtres et la recherche gardés. */
  compte: number
}

const parOrdre = (a: VocabularyTerm, b: VocabularyTerm) => a.sort_order - b.sort_order || a.label.localeCompare(b.label)

/** « COP30 — Belém » : la ville dit la COP à qui n'en retient pas le numéro. */
export const libelleDeCop = (cop: VocabularyCop): string => (cop.city ? `${cop.label} — ${cop.city}` : cop.label)

export function optionsDuFiltre(
  documents: LibraryDocument[],
  vocabulaire: LibraryVocabulary,
  critere: Critere,
  axe: Axe,
): OptionDeFiltre[] {
  const candidats = filtrer(documents, critere, axe)
  const compte = (valeur: string) => candidats.filter((d) => valeursDe(d, axe).includes(valeur)).length
  if (axe === 'cops') {
    return vocabulaire.cops.map((c) => ({ valeur: c.id, libelle: libelleDeCop(c), compte: compte(c.id) }))
  }
  const termes = axe === 'types' ? vocabulaire.types : vocabulaire.themes
  return [...termes].sort(parOrdre).map((t) => ({ valeur: t.code, libelle: t.label, compte: compte(t.code) }))
}

/** Le libellé d'un code ; nul pour un terme désactivé, que le vocabulaire servi ne cite plus. */
export function libelleDuTerme(termes: VocabularyTerm[], code: string): string | null {
  return termes.find((t) => t.code === code)?.label ?? null
}

/** Ce que l'API sert d'un réservé à qui n'a pas l'accès (FR-022). */
export const reserveMasque = (d: LibraryDocument): LibraryDocument => ({
  ...d,
  summary: null,
  themes: [],
  themes_hidden: true,
  external_url: null,
  accessible: false,
})

/**
 * Une liste gardée pour une autre personne — celle qui s'est déconnectée de ce
 * téléphone — ne montre pas ses réservés : on les lit comme l'API les servirait.
 */
export function pourUneAutrePersonne(bibliotheque: DocumentLibrary): DocumentLibrary {
  return {
    ...bibliotheque,
    documents: bibliotheque.documents.map((d) => (d.restricted && d.accessible ? reserveMasque(d) : d)),
  }
}

export const filtresActifs = (filtres: Filtres): boolean => AXES.some((axe) => filtres[axe].length > 0)

export type MarqueDeLigne =
  | 'telecharge'
  | 'a-jour'
  | 'nouveau'
  | 'reserve'
  | 'lien-externe'
  | 'lien-reseau'
  | 'remplace'
  | 'non-telecharge'

/** « Réseau nécessaire » et « Non téléchargé » portent le wifi barré de la maquette (04). */
export const ETAT_DE_LA_MARQUE: Record<MarqueDeLigne, NomDEtat> = {
  telecharge: 'telecharge',
  'a-jour': 'a-jour',
  nouveau: 'nouveau',
  reserve: 'reserve',
  'lien-externe': 'lien-externe',
  'lien-reseau': 'hors-connexion',
  remplace: 'remplace',
  'non-telecharge': 'hors-connexion',
}

export interface EtatSurLeTelephone {
  telecharge: boolean
  nouveau: boolean
  enLigne: boolean
}

/**
 * Les marques d'une ligne, d'après la maquette (03 · 01 et 04). « À jour » se tait
 * devant « Nouveau », qui le dit déjà. Hors connexion, seul compte ce qui s'ouvre :
 * « Nouveau » disparaît, « À jour » ne reste que sur un téléchargé, et le reste dit
 * qu'il attend le réseau.
 */
export function marquesDeLigne(document: LibraryDocument, etat: EtatSurLeTelephone): MarqueDeLigne[] {
  const fichier = document.source === 'file'
  const marques: MarqueDeLigne[] = []
  if (etat.telecharge) marques.push('telecharge')
  if (fichier && !document.superseded_by && (etat.enLigne ? !etat.nouveau : etat.telecharge)) marques.push('a-jour')
  if (document.superseded_by) marques.push('remplace')
  if (etat.enLigne && etat.nouveau) marques.push('nouveau')
  if (document.restricted) marques.push('reserve')
  if (!fichier) marques.push(etat.enLigne ? 'lien-externe' : 'lien-reseau')
  if (fichier && !etat.enLigne && !etat.telecharge) marques.push('non-telecharge')
  return marques
}
