/**
 * Le back-office du savoir de Guide Négo — FAQ, lexique, parcours. Champ pour
 * champ comme `negotiation/src/domain/admin_savoir.rs` ; contrat dans
 * `specs/013-guide-nego-faq-lexique/contracts/api-admin-savoir.md`.
 *
 * Les textes métier arrivent **non résolus** : l'écran les modifie dans chaque
 * langue. Aucun auteur d'un retour ou d'un signalement n'est jamais rendu (R9).
 */

import type { I18nText, IsoDate, IsoDateTime, Uuid } from './shared'

/** `negotiation.knowledge_status`. `to_review` reste servi au téléphone. */
export type AdminKnowledgeStatus = 'draft' | 'published' | 'to_review'

// ---------------------------------------------------------------------------
// Sources et liens, communs à la FAQ et au lexique
// ---------------------------------------------------------------------------

/** Un document de la bibliothèque, ou une référence extérieure titrée. */
export interface AdminKnowledgeSource {
  document_id: Uuid | null
  /** Résolu, rendu seulement. */
  document_title: string | null
  external_title: string | null
  external_url: string | null
  section_label: string | null
  page_from: number | null
  page_to: number | null
  quote: string | null
}

export type AdminKnowledgeSourceInput = Omit<AdminKnowledgeSource, 'document_title'>

/** Une entrée liée : sa désignation résolue et son état. */
export interface AdminKnowledgeRef {
  id: Uuid
  label: string
  status: AdminKnowledgeStatus
}

// ---------------------------------------------------------------------------
// FAQ — `/admin/negotiation/faq`
// ---------------------------------------------------------------------------

export interface AdminFaqList {
  entries: AdminFaqRow[]
  can_publish: boolean
  can_review: boolean
}

export interface AdminFaqRow {
  id: Uuid
  section_code: string
  /** Résolue dans la langue demandée. */
  question: string
  status: AdminKnowledgeStatus
  verified_on: IsoDate | null
  has_answer: boolean
  open_reports: number
  /** Posée : l'entrée ne se supprime plus, elle se dépublie. */
  first_published_at: IsoDateTime | null
  updated_at: IsoDateTime
}

/** « Cette réponse vous a-t-elle aidée ? », compté, sans personne. */
export interface AdminFaqFeedback {
  helpful: number
  not_helpful: number
  too_vague: number
  off_topic: number
  outdated: number
}

/** Un signalement « Dépassé ou faux », sans son auteur ni l'expert qui l'a clos. */
export interface AdminFaqReport {
  id: Uuid
  reasons: string[]
  from_feedback: boolean
  details: string | null
  status: 'open' | 'closed'
  outcome: 'revised' | 'confirmed' | 'dismissed' | null
  created_at: IsoDateTime
  handled_at: IsoDateTime | null
}

export interface AdminFaqEntry {
  id: Uuid
  section_code: string
  question: I18nText
  answer: I18nText | null
  status: AdminKnowledgeStatus
  verified_on: IsoDate | null
  /** Le nom de l'expert qui a daté la vérification. */
  verified_by_name: string | null
  editorial_rank: number | null
  origin_question_id: Uuid | null
  sources: AdminKnowledgeSource[]
  related: AdminKnowledgeRef[]
  feedback: AdminFaqFeedback
  reports: AdminFaqReport[]
  first_published_at: IsoDateTime | null
  created_at: IsoDateTime
  updated_at: IsoDateTime
  can_publish: boolean
  can_review: boolean
}

/**
 * Création et modification. Un champ absent ne change rien ; `null` vide un
 * champ facultatif. Sources et liées se remplacent en bloc. À la création,
 * `section_code` et `question` sont exigés.
 */
export interface AdminFaqInput {
  section_code?: string
  question?: I18nText
  answer?: I18nText | null
  editorial_rank?: number | null
  sources?: AdminKnowledgeSourceInput[]
  related_ids?: Uuid[]
}

/** `POST …/verify` — sans date : aujourd'hui, heure de Paris. */
export interface AdminFaqVerifyInput {
  verified_on?: IsoDate
}

// ---------------------------------------------------------------------------
// Lexique — `/admin/negotiation/glossary`
// ---------------------------------------------------------------------------

export interface AdminGlossaryList {
  entries: AdminGlossaryRow[]
  can_publish: boolean
  can_review: boolean
}

export interface AdminGlossaryRow {
  id: Uuid
  slug: string
  family_code: string
  term: string
  acronym: string | null
  /** Résolue dans la langue demandée. */
  translation: string
  status: AdminKnowledgeStatus
  first_published_at: IsoDateTime | null
  updated_at: IsoDateTime
}

export interface AdminGlossaryEntry {
  id: Uuid
  /** Posé à la création depuis le terme, jamais recalculé ni accepté en entrée. */
  slug: string
  family_code: string
  term: string
  acronym: string | null
  variants: string[]
  translation: I18nText
  definition: I18nText
  heard_in_room: string | null
  status: AdminKnowledgeStatus
  sources: AdminKnowledgeSource[]
  related: AdminKnowledgeRef[]
  first_published_at: IsoDateTime | null
  created_at: IsoDateTime
  updated_at: IsoDateTime
  can_publish: boolean
  can_review: boolean
}

/** À la création, `family_code`, `term`, `translation` et `definition` sont exigés. */
export interface AdminGlossaryInput {
  family_code?: string
  term?: string
  acronym?: string | null
  variants?: string[]
  translation?: I18nText
  definition?: I18nText
  heard_in_room?: string | null
  sources?: AdminKnowledgeSourceInput[]
  related_ids?: Uuid[]
}

// ---------------------------------------------------------------------------
// Parcours « Ma première COP » — `/admin/negotiation/pathway`
// ---------------------------------------------------------------------------

export type AdminPathwayLinkKind = 'document' | 'faq' | 'glossary'

export interface AdminPathwayLink {
  kind: AdminPathwayLinkKind
  target_id: Uuid
  /** Titre du document, question ou terme, résolu. */
  target_label: string | null
  page: number | null
  section: string | null
  label: I18nText | null
}

export interface AdminPathwayStep {
  id: Uuid
  group_id: Uuid
  label: I18nText
  detail: I18nText | null
  origin_label: I18nText | null
  link: AdminPathwayLink | null
  sort_order: number
  is_published: boolean
  /** Comptes qui l'ont cochée : une étape cochée se dépublie, elle ne se supprime pas. */
  checks: number
}

export interface AdminPathwayGroup {
  id: Uuid
  label: I18nText
  sort_order: number
  is_published: boolean
  steps: AdminPathwayStep[]
}

/** Toutes les écritures du parcours rendent le parcours entier. */
export interface AdminPathway {
  groups: AdminPathwayGroup[]
  can_publish: boolean
}

export interface AdminPathwayGroupInput {
  label?: I18nText
  is_published?: boolean
}

export interface AdminPathwayLinkInput {
  kind: AdminPathwayLinkKind
  target_id: Uuid
  page?: number | null
  section?: string | null
  label?: I18nText | null
}

/** À la création, `group_id` et `label` sont exigés ; une étape nouvelle va en fin de groupe. */
export interface AdminPathwayStepInput {
  group_id?: Uuid
  label?: I18nText
  detail?: I18nText | null
  origin_label?: I18nText | null
  link?: AdminPathwayLinkInput | null
  is_published?: boolean
}

/** L'ordre des groupes, et de chaque groupe ses étapes. */
export interface AdminPathwayOrderInput {
  groups: { id: Uuid; step_ids: Uuid[] }[]
}
