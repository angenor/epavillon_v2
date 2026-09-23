/**
 * La recherche dans un document, sur le téléphone et sans réseau (FR-041).
 *
 * Sans accents ni casse : « progres collectifs » trouve « progrès collectifs ». La
 * comparaison se fait sur un texte replié, mais chaque position renvoie au texte
 * d'origine — c'est lui qu'on surligne, lettre pour lettre.
 */
import type { Block, DocumentReading, Span } from '~/types/negotiation-documents'
import { blocsDeLaPage, sectionDeLaPage } from './forme-lisible.ts'

/** Où se lit un texte dans un bloc : ses segments, ou, pour une origine, sa légende et son texte. */
export type Champ = 'spans' | 'caption' | 'text'

export interface Occurrence {
  page: number
  /** Le rang du bloc dans `blocsDeLaPage(page)`. */
  bloc: number
  champ: Champ
  /** Positions dans le texte du champ, segments bout à bout. */
  debut: number
  fin: number
}

export interface Passage extends Occurrence {
  /** L'étiquette imprimée de la page. */
  etiquette: string
  section: string | null
  extrait: { avant: string; trouve: string; apres: string }
}

// « CO₂ » se tape « co2 », « Paris–Nairobi » « paris-nairobi » : NFKD ramène exposants
// et indices, la table le reste.
const REMPLACEMENTS: Record<string, string> = { œ: 'oe', æ: 'ae', '’': "'", '–': '-', '—': '-', '‑': '-', '‐': '-' }

/**
 * Le texte replié, et pour chacune de ses lettres la position de la lettre d'origine.
 * Les blancs se réduisent à une espace : une double espace d'extraction ne cache rien.
 */
export function replier(texte: string): { replie: string; origine: number[] } {
  let replie = ''
  const origine: number[] = []
  for (let i = 0; i < texte.length; i += 1) {
    const lettre = texte[i] as string
    if (/\s/u.test(lettre)) {
      if (!replie.endsWith(' ')) {
        replie += ' '
        origine.push(i)
      }
      continue
    }
    const basse = lettre.toLowerCase()
    const pliee = REMPLACEMENTS[basse] ?? basse.normalize('NFKD').replace(/\p{M}/gu, '')
    for (const c of pliee) {
      replie += c
      origine.push(i)
    }
  }
  return { replie, origine }
}

export const expressionRepliee = (expression: string): string => replier(expression.trim()).replie

/** Une lettre seule trouverait tout le document : on attend la deuxième. */
export const LONGUEUR_MINIMALE = 2

const texteDe = (spans: Span[] | undefined): string => (spans ?? []).map((s) => s.text).join('')

function champsDe(bloc: Block): Array<[Champ, string]> {
  if (bloc.kind === 'origin') {
    return [
      ['caption', texteDe(bloc.caption)],
      ['text', texteDe(bloc.text)],
    ]
  }
  return [['spans', texteDe(bloc.spans)]]
}

const LONGUEUR_D_EXTRAIT = 48

function extraitDe(texte: string, debut: number, fin: number): Passage['extrait'] {
  let depuis = Math.max(0, debut - LONGUEUR_D_EXTRAIT)
  let jusqua = Math.min(texte.length, fin + LONGUEUR_D_EXTRAIT)
  // Coupé à un mot entier : un extrait qui s'ouvre sur « ogrès » ne se lit pas.
  if (depuis > 0) depuis = texte.indexOf(' ', depuis) + 1 || depuis
  if (jusqua < texte.length) jusqua = texte.lastIndexOf(' ', jusqua) > fin ? texte.lastIndexOf(' ', jusqua) : jusqua
  return {
    avant: (depuis > 0 ? '… ' : '') + texte.slice(depuis, debut),
    trouve: texte.slice(debut, fin),
    apres: texte.slice(fin, jusqua) + (jusqua < texte.length ? ' …' : ''),
  }
}

interface ChampReplie {
  page: number
  etiquette: string
  section: string | null
  bloc: number
  champ: Champ
  texte: string
  replie: string
  origine: number[]
}

// Replié une fois par document : chaque frappe ne cherche plus que dans l'index.
const index = new WeakMap<DocumentReading, ChampReplie[]>()

function indexer(lecture: DocumentReading): ChampReplie[] {
  const deja = index.get(lecture)
  if (deja) return deja
  const champs: ChampReplie[] = []
  for (const page of lecture.pages) {
    // La section de la page, pas celle du passage : un passage au-dessus d'un titre porte la suivante.
    const section = sectionDeLaPage(lecture.outline, page.index, 3)?.title ?? null
    blocsDeLaPage(page).forEach((bloc, rang) => {
      for (const [champ, texte] of champsDe(bloc)) {
        if (texte) champs.push({ page: page.index, etiquette: page.label, section, bloc: rang, champ, texte, ...replier(texte) })
      }
    })
  }
  index.set(lecture, champs)
  return champs
}

/** Toutes les occurrences, dans l'ordre du document. Moins de deux lettres ne cherchent rien. */
export function chercherDansLeDocument(lecture: DocumentReading, expression: string): Passage[] {
  const cherche = expressionRepliee(expression)
  if (cherche.length < LONGUEUR_MINIMALE) return []
  const passages: Passage[] = []
  for (const { page, etiquette, section, bloc, champ, texte, replie, origine } of indexer(lecture)) {
    for (let i = replie.indexOf(cherche); i !== -1; i = replie.indexOf(cherche, i + cherche.length)) {
      const debut = origine[i] as number
      const fin = (origine[i + cherche.length - 1] as number) + 1
      passages.push({ page, bloc, champ, debut, fin, etiquette, section, extrait: extraitDe(texte, debut, fin) })
    }
  }
  return passages
}

/** « Vous êtes ici » : le premier passage de la page en cours, s'il y en a un. */
export function passageDeLaPage(passages: Passage[], page: number): number {
  return passages.findIndex((p) => p.page === page)
}

/** Un segment, éventuellement coupé par un surlignage. */
export interface SegmentSurligne extends Span {
  surlignage?: 'courant' | 'autre'
  /** Le rang du segment d'origine : les morceaux d'un même terme restent un seul terme. */
  source: number
}

export interface Surlignage {
  debut: number
  fin: number
  courant: boolean
}

/**
 * Coupe les segments d'un champ aux bornes des surlignages, sans toucher au texte :
 * la concaténation des segments rendus est toujours celle d'origine.
 */
export function surligner(spans: Span[], surlignages: Surlignage[]): SegmentSurligne[] {
  if (!surlignages.length) return spans.map((span, source) => ({ ...span, source }))
  const coupes: SegmentSurligne[] = []
  let position = 0
  for (const [source, span] of spans.entries()) {
    const debutDuSegment = position
    const finDuSegment = position + span.text.length
    const bornes = new Set([debutDuSegment, finDuSegment])
    for (const s of surlignages) {
      if (s.debut > debutDuSegment && s.debut < finDuSegment) bornes.add(s.debut)
      if (s.fin > debutDuSegment && s.fin < finDuSegment) bornes.add(s.fin)
    }
    const triees = [...bornes].sort((a, b) => a - b)
    for (let i = 0; i < triees.length - 1; i += 1) {
      const a = triees[i] as number
      const b = triees[i + 1] as number
      const couvrant = surlignages.find((s) => s.debut <= a && s.fin >= b)
      const morceau: SegmentSurligne = { ...span, source, text: span.text.slice(a - debutDuSegment, b - debutDuSegment) }
      if (couvrant) morceau.surlignage = couvrant.courant ? 'courant' : 'autre'
      coupes.push(morceau)
    }
    position = finDuSegment
  }
  return coupes
}

/** Le décompte de pages cherchées : celles du document, pas du fichier. */
export const pagesCherchees = (lecture: DocumentReading): number => lecture.page_count || lecture.pages.length

