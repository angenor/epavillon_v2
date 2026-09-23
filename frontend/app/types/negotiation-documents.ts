/**
 * Les lectures publiques des documents de Guide Négo, et la forme lisible que
 * le lecteur affiche. Champ pour champ comme `negotiation/src/domain/documents.rs`
 * et `domain/extraction/forme.rs` ; contrats dans
 * `specs/011-guide-nego-documents/contracts/api-documents.md` et `forme-lisible.md`.
 *
 * Les titres et résumés arrivent **résolus** dans la langue demandée : ce sont
 * des chaînes, pas des `I18nText`.
 */

import type { IsoDate, IsoDateTime, Uuid } from './shared'

export type DocumentSource = 'file' | 'link'

export type ReadingMode = 'reflow' | 'as_is'

// ---------------------------------------------------------------------------
// La bibliothèque — `GET /negotiation/documents`
// ---------------------------------------------------------------------------

export interface DocumentLibrary {
  documents: LibraryDocument[]
  vocabulary: LibraryVocabulary
  /** Hors de l'empreinte : il change à chaque lecture. */
  served_at: IsoDateTime
}

export interface LibraryDocument {
  id: Uuid
  slug: string
  version: string
  title: string
  /** Nul pour un réservé sans accès. */
  summary: string | null
  /** Code du terme `document_type`. */
  type: string
  /** Codes `negotiation_theme` ; vide pour un réservé sans accès. */
  themes: string[]
  /** Vrai pour tout réservé sans accès : qu'il en porte ou non ne se dit pas. */
  themes_hidden: boolean
  /** L'édition (`event.events`) que le document concerne. */
  cop: Uuid | null
  issued_on: IsoDate | null
  published_at: IsoDateTime
  publisher: string | null
  locale: string
  source: DocumentSource
  /** `null` pour un réservé sans accès : l'adresse d'un lien est son contenu. */
  external_url: string | null
  /** « enb.iisd.org », sans `www.`. */
  link_host: string | null
  restricted: boolean
  accessible: boolean
  page_count: number | null
  reading_bytes: number | null
  mode: ReadingMode | null
  /** Le bout publié de la chaîne de remplacement. */
  superseded_by: Successor | null
  /** Une copie gardée dont l'empreinte diffère est celle d'un autre fichier. */
  reading_etag: string | null
}

export interface Successor {
  id: Uuid
  title: string
  published_at: IsoDateTime
  page_count: number | null
}

/** Les libellés des seules valeurs que la liste cite. */
export interface LibraryVocabulary {
  types: VocabularyTerm[]
  themes: VocabularyTerm[]
  cops: VocabularyCop[]
}

export interface VocabularyTerm {
  code: string
  label: string
  sort_order: number
}

export interface VocabularyCop {
  id: Uuid
  label: string
  city: string | null
}

// ---------------------------------------------------------------------------
// La recherche dans le texte — `GET /negotiation/documents?q=`
// ---------------------------------------------------------------------------

export interface DocumentTextHits {
  query: string
  hits: TextHit[]
}

/** Pour un réservé sans accès : l'identifiant seul, `pages` vide. */
export interface TextHit {
  document_id: Uuid
  pages: PageHit[]
}

export interface PageHit {
  index: number
  label: string
  excerpt: string
}

// ---------------------------------------------------------------------------
// La forme lisible — `GET /negotiation/documents/{id}/reading`
// ---------------------------------------------------------------------------

export interface DocumentReading {
  id: Uuid
  version: string
  mode: ReadingMode
  /** Pages du document, pas du fichier. */
  page_count: number
  /** Vide si aucun sommaire n'a été repéré, et en mode `as_is`. */
  outline: OutlineEntry[]
  pages: ReadingPage[]
}

export interface OutlineEntry {
  title: string
  level: 1 | 2 | 3
  /** Renvoie à `ReadingPage.index`. */
  page_index: number
  children: OutlineEntry[]
}

export interface ReadingPage {
  /** De 1 à `page_count` : clé de la progression et des notes. */
  index: number
  /** L'étiquette imprimée, « 59 ». */
  label: string
  /** Chemin d'API, relatif à la base : en `as_is`, et sur les pages d'origine. */
  image?: string
  /** Vide en mode `as_is`. */
  blocks: Block[]
}

/**
 * Un segment de texte. Les marques ne sont présentes que vraies ; `term`
 * implique `italic` et ouvre la feuille du lexique, titrée de `text`.
 */
export interface Span {
  text: string
  italic?: true
  bold?: true
  term?: true
}

export interface HeadingBlock {
  kind: 'heading'
  level: 1 | 2 | 3
  spans: Span[]
}

export interface ParagraphBlock {
  kind: 'paragraph'
  spans: Span[]
}

export interface ListItemBlock {
  kind: 'list_item'
  spans: Span[]
  depth: 0 | 1 | 2
  /** « • », « 1. », « a) ». */
  marker: string
}

/** Note de bas de page, rendue en fin de page. */
export interface NoteBlock {
  kind: 'note'
  spans: Span[]
  mark: string
}

export type OriginReason = 'table' | 'figure'

/** Un tableau ou une figure : renvoie à l'image de la page. */
export interface OriginBlock {
  kind: 'origin'
  reason: OriginReason
  /** Prévu par le contrat ; l'extraction ne le produit pas encore. */
  caption?: Span[]
  /** Le texte de la zone, affiché replié : il la rend cherchable. */
  text?: Span[]
}

/** Grammaire close : un `kind` inconnu du lecteur s'ignore. */
export type Block = HeadingBlock | ParagraphBlock | ListItemBlock | NoteBlock | OriginBlock

export type BlockKind = Block['kind']

// ---------------------------------------------------------------------------
// Les notes de correction — `GET /negotiation/documents/corrections`
// ---------------------------------------------------------------------------

export interface CorrectionNoteList {
  notes: CorrectionNote[]
}

/** Une note vivante, posée par-dessus le texte sans le modifier. */
export interface CorrectionNote {
  id: Uuid
  document_id: Uuid
  page_index: number
  /** L'extrait cité ; nul : la note vaut pour la page. */
  passage: string | null
  /** Résolu dans la langue demandée. */
  body: string
  author_name: string
  posted_at: IsoDateTime
}

// ---------------------------------------------------------------------------
// Les favoris — `GET /negotiation/me/bookmarks`
// ---------------------------------------------------------------------------

export interface DocumentBookmarkList {
  bookmarks: DocumentBookmark[]
}

export interface DocumentBookmark {
  document_id: Uuid
  created_at: IsoDateTime
}
