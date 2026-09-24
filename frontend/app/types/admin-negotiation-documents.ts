/**
 * Le back-office des documents de Guide Négo et des notes de correction. Champ
 * pour champ comme `negotiation/src/domain/admin_documents.rs` ; contrat dans
 * `specs/011-guide-nego-documents/contracts/api-admin-documents.md`.
 *
 * Contrairement aux lectures publiques, les textes arrivent **non résolus** :
 * l'écran les modifie dans chaque langue.
 */

import type { I18nText, IsoDate, IsoDateTime, Uuid } from './shared'
import type { Block, DocumentSource, OutlineEntry } from './negotiation-documents'

/** Dérivé de `published_at` et `unpublished_at`, jamais stocké. */
export type AdminDocumentState = 'draft' | 'published' | 'unpublished'

/** `negotiation.rendition_status`. */
export type ExtractionStatus = 'pending' | 'extracting' | 'ready' | 'failed'

export interface DocumentLink {
  id: Uuid
  title: string
  version: string
}

/** L'extraction du fichier du moment. */
export interface ExtractionState {
  status: ExtractionStatus
  page_count: number | null
  is_reflowable: boolean | null
  /** `null` : suit le verdict ; vrai ou faux : le choix de l'administratrice. */
  large_text_choice: boolean | null
  has_text: boolean
  /** « Texte agrandi » offert, choix et verdict appliqués. */
  large_text: boolean
  failure_reason: string | null
  reading_bytes: number | null
  extracted_at: IsoDateTime | null
}

// ---------------------------------------------------------------------------
// La liste — `GET /admin/negotiation/documents`
// ---------------------------------------------------------------------------

export interface AdminDocumentList {
  documents: AdminDocumentRow[]
  can_publish: boolean
  can_correct: boolean
}

export interface AdminDocumentRow {
  id: Uuid
  /** Résolu dans la langue demandée. */
  title: string
  type: string
  version: string
  state: AdminDocumentState
  /** Nul : un brouillon sans fichier ni lien. */
  source: DocumentSource | null
  restricted: boolean
  published_at: IsoDateTime | null
  updated_at: IsoDateTime
  supersedes: DocumentLink | null
  superseded_by: DocumentLink | null
  extraction: ExtractionState | null
}

// ---------------------------------------------------------------------------
// La fiche — `GET /admin/negotiation/documents/{id}`
// ---------------------------------------------------------------------------

export interface AdminDocument {
  id: Uuid
  slug: string
  title: I18nText
  summary: I18nText | null
  type: string
  themes: string[]
  cop: Uuid | null
  version: string
  issued_on: IsoDate | null
  publisher: string | null
  locale: string
  source: DocumentSource | null
  asset_id: Uuid | null
  file: AdminDocumentFile | null
  external_url: string | null
  supersedes: DocumentLink | null
  superseded_by: DocumentLink | null
  restricted: boolean
  /** Ne prend effet qu'avec l'assistant. */
  rag_eligible: boolean
  state: AdminDocumentState
  published_at: IsoDateTime | null
  unpublished_at: IsoDateTime | null
  /** Vrai : déjà publié une fois, son fichier ne change plus. */
  file_locked: boolean
  extraction: ExtractionState | null
  created_at: IsoDateTime
  updated_at: IsoDateTime
  can_publish: boolean
  can_correct: boolean
}

export interface AdminDocumentFile {
  filename: string | null
  byte_size: number
  mime_type: string
}

/**
 * Création et modification. Un champ absent ne change rien ; `null` vide un
 * champ facultatif. À la création, `title` et `type` sont exigés.
 */
export interface AdminDocumentInput {
  title?: I18nText
  summary?: I18nText | null
  type?: string
  /** Remplacement en bloc. */
  themes?: string[]
  cop?: Uuid | null
  version?: string
  issued_on?: IsoDate | null
  publisher?: string | null
  locale?: string
  supersedes_id?: Uuid | null
  restricted?: boolean
  rag_eligible?: boolean
  external_url?: string | null
}

/** `PUT …/file` — le PDF déposé par la garde média. */
export interface AttachFileInput {
  asset_id: Uuid
}

/** `PUT …/large-text` — `null` rend la main au verdict de l'extraction. */
export interface LargeTextChoiceInput {
  choice: boolean | null
}

// ---------------------------------------------------------------------------
// L'aperçu — `GET /admin/negotiation/documents/{id}/preview`
// ---------------------------------------------------------------------------

/** Les indicateurs du verdict, tels que l'extraction les relève. */
export interface ExtractionQuality {
  pages: number
  pages_avec_texte: number
  pages_a_origine: number
  tableaux: number
  figures: number
  notes: number
  titres: number
  entrees_du_sommaire: number
  sommaire_depuis_signets: boolean
  italiques_maigres: number
  termes: number
  cesures_gardees: number
  cesures_recollees: number
}

export interface AdminDocumentPreview {
  id: Uuid
  extraction: ExtractionState | null
  quality: ExtractionQuality | null
  extractor: string | null
  outline: OutlineEntry[]
  /** Vide tant que l'extraction du fichier du moment n'est pas prête. */
  pages: AdminPreviewPage[]
}

export interface AdminPreviewPage {
  index: number
  label: string
  blocks: Block[]
  /** Chemin d'API de l'image, relatif à la base, brouillon compris. */
  image: string | null
  has_origin_block: boolean
}

// ---------------------------------------------------------------------------
// Les notes de correction
// ---------------------------------------------------------------------------

export interface PersonLink {
  id: Uuid
  name: string
}

/** Une note, vivante ou retirée — jamais supprimée. */
export interface AdminCorrectionNote {
  id: Uuid
  document_id: Uuid
  page_index: number
  passage: string | null
  body: I18nText
  author: PersonLink
  posted_at: IsoDateTime
  withdrawn_at: IsoDateTime | null
  withdrawn_by: PersonLink | null
}

export interface AdminCorrectionNoteList {
  notes: AdminCorrectionNote[]
  can_post: boolean
  can_withdraw: boolean
}

/** `POST …/corrections`. Le français du corps est exigé. */
export interface CorrectionNoteInput {
  page_index: number
  passage?: string | null
  body: I18nText
}
