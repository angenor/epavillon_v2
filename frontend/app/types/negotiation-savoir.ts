/**
 * Le savoir de Guide Négo — FAQ, parcours, lexique. Champ pour champ comme
 * `negotiation/src/domain/savoir.rs` ; contrat dans
 * `specs/013-guide-nego-faq-lexique/contracts/api-savoir.md`.
 *
 * Les textes arrivent **résolus** dans la langue demandée : ce sont des
 * chaînes, pas des `I18nText`.
 */

import type { IsoDate, IsoDateTime, Uuid } from './shared'

/** Les brouillons ne sortent jamais. */
export type KnowledgeStatus = 'published' | 'to_review'

// ---------------------------------------------------------------------------
// Le paquet — `GET /negotiation/knowledge`
// ---------------------------------------------------------------------------

export interface KnowledgeBundle {
  served_at: IsoDateTime
  /** Faux pour une différence (`?since=`). */
  complete: boolean
  faq_sections: FaqSection[]
  glossary_families: GlossaryFamily[]
  faq: FaqEntry[]
  glossary: GlossaryEntry[]
  /** Toujours entier, même dans une différence. */
  pathway: Pathway
  /** Trois identifiants d'entrées de FAQ au plus. */
  most_read: Uuid[]
  /** Vide quand `complete`. */
  removed: KnowledgeRemoved
}

export interface FaqSection {
  code: string
  label: string
  /** Nom d'un pictogramme de Guide Négo. */
  icon: string | null
  sort_order: number
}

export interface GlossaryFamily {
  code: string
  label: string
  sort_order: number
}

export interface FaqEntry {
  id: Uuid
  section_code: string
  question: string
  answer: string
  status: KnowledgeStatus
  verified_on: IsoDate
  sources: KnowledgeSource[]
  /** Tous les liens, publiés ou non : à filtrer sur les entrées présentes. */
  related_ids: Uuid[]
  updated_at: IsoDateTime
}

export interface GlossaryEntry {
  id: Uuid
  slug: string
  family_code: string
  term: string
  acronym: string | null
  variants: string[]
  translation: string
  definition: string
  heard_in_room: string | null
  sources: KnowledgeSource[]
  related_ids: Uuid[]
  status: KnowledgeStatus
  updated_at: IsoDateTime
}

/** Un document de la bibliothèque ou une référence extérieure ; une clé nulle est omise. */
export interface KnowledgeSource {
  document_id?: Uuid
  document_title?: string
  external_title?: string
  external_url?: string
  section_label?: string
  page_from?: number
  page_to?: number
  quote?: string
}

export interface Pathway {
  groups: PathwayGroup[]
}

export interface PathwayGroup {
  id: Uuid
  label: string
  sort_order: number
  steps: PathwayStep[]
}

export interface PathwayStep {
  id: Uuid
  label: string
  detail: string | null
  origin_label: string | null
  link: PathwayLink | null
  sort_order: number
}

export interface PathwayLink {
  kind: 'document' | 'faq' | 'glossary'
  target_id: Uuid
  /** Document seulement. */
  page: number | null
  section: string | null
  label: string | null
}

export interface KnowledgeRemoved {
  faq: Uuid[]
  glossary: Uuid[]
}

// ---------------------------------------------------------------------------
// Les termes favoris — `GET /negotiation/me/glossary-favorites`
// ---------------------------------------------------------------------------

export interface MyGlossaryFavorites {
  entry_ids: Uuid[]
}
