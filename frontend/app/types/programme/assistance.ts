/**
 * L'ASSISTANCE DE CHAQUE ÉCRAN (02/11) : la fenêtre « Aide » des organisations
 * et celle de l'équipe. Un sujet porte sur un dossier (`proposal_id`) ou sur la
 * plateforme (nul) ; ce que chacun lit est décidé en base
 * (`programme.support_visible_comments`, 070_programme_proposals.sql § 6).
 */
import type { I18nText, IsoDateTime, PersonId, ProposalId, Uuid } from '../shared'

/** De quel côté se tient la personne connectée ; nul : pas d'assistance pour elle. */
export type AssistanceSide = 'organization' | 'team'

export interface AssistanceStatus {
  side: AssistanceSide | null
  unread: number
}

export interface AssistanceMessage {
  id: Uuid
  proposal_id: ProposalId | null
  parent_id: Uuid | null
  author_id: PersonId
  /** Nul côté organisation quand l'équipe écrit : elle signe « l'équipe ». */
  author_name: string | null
  from_team: boolean
  body: string
  is_change_request: boolean
  resolved_at: IsoDateTime | null
  edited_at: IsoDateTime | null
  created_at: IsoDateTime
  /** Code de `proposal_comment_topic` (dossier) ou de `platform_support_topic` (plateforme). */
  topic: string | null
  topic_other: string | null
  subject: string | null
  closed_at: IsoDateTime | null
  closed_by: PersonId | null
  is_read: boolean
  reference_code: string | null
  proposal_title: I18nText | null
  /** Sur le message qui ouvre un sujet seulement. */
  organization_name: string | null
}

/** Un dossier sur lequel l'organisation peut ouvrir un sujet. */
export interface AssistanceProposal {
  id: ProposalId
  reference_code: string
  title: I18nText
}

export interface AssistanceFeed {
  side: AssistanceSide
  messages: AssistanceMessage[]
  /** Côté organisation : ses dossiers ouverts à l'assistance. Vide côté équipe. */
  proposals: AssistanceProposal[]
  /** Côté organisation : les organisations à suivre en temps réel. */
  organization_ids: Uuid[]
}

/** Ouvrir un sujet sur la plateforme. */
export interface OpenSupportSubjectPayload {
  /** Code de `platform_support_topic`. */
  topic: string
  subject: string
  body: string
}

export interface ReplySupportSubjectPayload {
  body: string
}

export interface MarkAssistanceReadPayload {
  comment_ids: Uuid[]
}
