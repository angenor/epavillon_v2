/**
 * Le rendu de la forme lisible — **grammaire close** (`contracts/forme-lisible.md`).
 *
 * Aucun HTML n'arrive jamais : des blocs et des segments, rendus par des balises
 * choisies ici. La forme vient d'un JSON gardé sur le téléphone, peut-être écrit par
 * une version plus récente de l'API : un bloc d'un `kind` inconnu, un segment mal
 * formé s'ignorent au lieu de casser la page.
 */
import type { Block, DocumentReading, OutlineEntry, ReadingPage, Span } from '~/types/negotiation-documents'
import { replier } from './repli.ts'

const estObjet = (v: unknown): v is Record<string, unknown> => typeof v === 'object' && v !== null

function segmentsSurs(brut: unknown): Span[] {
  if (!Array.isArray(brut)) return []
  return brut.flatMap((s): Span[] => {
    if (!estObjet(s) || typeof s.text !== 'string') return []
    const span: Span = { text: s.text }
    if (s.italic === true || s.term === true) span.italic = true
    if (s.bold === true) span.bold = true
    if (s.term === true) span.term = true
    return [span]
  })
}

const NIVEAUX = new Set([1, 2, 3])
const PROFONDEURS = new Set([0, 1, 2])

function blocSur(b: unknown): Block | null {
  if (!estObjet(b)) return null
  const spans = segmentsSurs(b.spans)
  switch (b.kind) {
    case 'heading':
      return NIVEAUX.has(b.level as number) ? { kind: 'heading', level: b.level as 1 | 2 | 3, spans } : null
    case 'paragraph':
      return { kind: 'paragraph', spans }
    case 'list_item':
      return {
        kind: 'list_item',
        spans,
        depth: PROFONDEURS.has(b.depth as number) ? (b.depth as 0 | 1 | 2) : 0,
        marker: typeof b.marker === 'string' ? b.marker : '•',
      }
    case 'note':
      return { kind: 'note', spans, mark: typeof b.mark === 'string' ? b.mark : '' }
    case 'origin': {
      if (b.reason !== 'table' && b.reason !== 'figure') return null
      const bloc: Block = { kind: 'origin', reason: b.reason }
      const legende = segmentsSurs(b.caption)
      const texte = segmentsSurs(b.text)
      if (legende.length) bloc.caption = legende
      if (texte.length) bloc.text = texte
      return bloc
    }
    default:
      return null
  }
}

/** Une forme écrite par une autre version de l'API, ou abîmée, ne s'ouvre pas : elle se retélécharge. */
export function estUneFormeLisible(v: unknown): v is DocumentReading {
  if (!estObjet(v)) return false
  return (
    typeof v.version === 'string' &&
    typeof v.has_text === 'boolean' &&
    typeof v.large_text === 'boolean' &&
    Array.isArray(v.pages) &&
    v.pages.length > 0 &&
    v.pages.every((p) => estObjet(p) && typeof p.index === 'number' && typeof p.label === 'string')
  )
}

/** Les blocs qu'on sait rendre, dans l'ordre du flux ; les notes de bas de page en fin de page. */
export function blocsDeLaPage(page: ReadingPage): Block[] {
  const connus = (Array.isArray(page.blocks) ? page.blocks : []).flatMap((b) => blocSur(b) ?? [])
  return [...connus.filter((b) => b.kind !== 'note'), ...connus.filter((b) => b.kind === 'note')]
}

function aplatir(entrees: OutlineEntry[]): OutlineEntry[] {
  return entrees.flatMap((e) => [e, ...aplatir(e.children ?? [])])
}

/**
 * La section d'une page : la dernière entrée ouverte avant elle, jusqu'au niveau dit.
 * Le pied s'arrête au niveau 2 — « 3.6 Adaptation » et non « 3.6.1 » : il dit où l'on
 * est, pas le détail ; un passage trouvé, lui, dit sa sous-partie.
 */
export function sectionDeLaPage(sommaire: OutlineEntry[], index: number, niveau = 2): OutlineEntry | null {
  let trouvee: OutlineEntry | null = null
  for (const entree of aplatir(sommaire)) {
    if (entree.level <= niveau && entree.page_index <= index) trouvee = entree
  }
  return trouvee
}

/** La page où reprendre : celle notée, si elle existe encore ; jamais la première. */
export function pageDeReprise(lecture: DocumentReading, notee: number | null): ReadingPage | null {
  if (notee === null || notee <= 1) return null
  return lecture.pages.find((p) => p.index === notee) ?? null
}

/** La part lue, de 0 à 1 : la page en cours sur le total. */
export const partLue = (index: number, total: number): number => (total > 0 ? Math.min(1, index / total) : 0)

const texteDe = (spans: Span[] | undefined): string => (spans ?? []).map((s) => s.text).join('')
const texteDuBloc = (bloc: Block): string =>
  bloc.kind === 'origin' ? `${texteDe(bloc.caption)} ${texteDe(bloc.text)}` : texteDe(bloc.spans)

const replie = (texte: string): string => replier(texte).replie.trim()

/**
 * Le rang du bloc qui porte l'extrait cité d'une note, ou nul : la tête de page.
 * Un extrait choisi à cheval sur deux blocs s'ancre au premier, par sa première ligne.
 */
export function blocDuPassage(blocs: Block[], passage: string | null): number | null {
  if (!passage) return null
  const textes = blocs.map((b) => replie(texteDuBloc(b)))
  const premiereLigne = passage.split(/\n/).find((l) => l.trim()) ?? ''
  for (const cherche of [replie(passage), replie(premiereLigne)]) {
    if (!cherche) continue
    const rang = textes.findIndex((t) => t.includes(cherche))
    if (rang >= 0) return rang
  }
  return null
}

/** Les notes d'une page, rangées par bloc ; celles qu'aucun bloc ne porte vont en tête. */
export function ancrerLesNotes<N extends { passage: string | null }>(
  blocs: Block[],
  notes: N[],
): { enTete: N[]; parBloc: Map<number, N[]> } {
  const enTete: N[] = []
  const parBloc = new Map<number, N[]>()
  for (const note of notes) {
    const rang = blocDuPassage(blocs, note.passage)
    if (rang === null) enTete.push(note)
    else parBloc.set(rang, [...(parBloc.get(rang) ?? []), note])
  }
  return { enTete, parBloc }
}
