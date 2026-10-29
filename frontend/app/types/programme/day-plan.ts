/**
 * Le jour J d'une activité retenue, `070_programme_proposals.sql` § 4 bis :
 * `proposal_focal_points`, `proposal_technical_needs`, `proposal_social_posts`,
 * et `programme.proposal_needs_deadline()` (`075` § 7 ter).
 *
 * Les supports présentés ne sont pas ici : ce sont des pièces du dossier
 * (`ProposalDocument`, type `presentation`), qui disent qui les présente.
 */

import type { IsoDateTime, PersonId, Uuid } from '../shared'

/** Déduite de l'adresse à la lecture : aucune colonne, aucune énumération en base. */
export type SocialPlatform = 'linkedin' | 'facebook' | 'x' | 'instagram' | 'youtube' | 'tiktok' | 'other'

export interface FocalPoint {
  /** Nul pour le point focal PAR DÉFAUT (le déposant), tant que rien n'a été enregistré. */
  id: Uuid | null
  speaker_id: Uuid | null
  person_id: PersonId | null
  full_name: string
  phone: string
  whatsapp: string
  /** Vrai : c'est le déposant proposé par défaut, rien n'est encore enregistré. */
  is_default: boolean
}

export interface TechnicalNeeds {
  video: boolean
  remote: boolean
  remote_details: string
  accessibility: boolean
  accessibility_details: string
  other: string
  /** Nul tant que l'organisation n'a rien déclaré (même « aucun besoin »). */
  updated_at: IsoDateTime | null
}

export interface SocialPost {
  id: Uuid
  url: string
  platform: SocialPlatform
  created_at: IsoDateTime
}

export interface ProposalDayPlan {
  focal_points: FocalPoint[]
  technical_needs: TechnicalNeeds
  social_posts: SocialPost[]
  /** 10 jours avant la première séance ; nul sans séance. Indicative : rien n'est refusé après. */
  needs_deadline: IsoDateTime | null
}

export interface FocalPointInput {
  speaker_id: Uuid | null
  person_id: PersonId | null
  full_name: string
  phone: string
  whatsapp: string
}

/** Remplace la liste entière ; vide, elle est refusée. */
export interface SaveFocalPointsPayload {
  focal_points: FocalPointInput[]
}

/** Le détail d'une case décochée est effacé par l'API. */
export type SaveTechnicalNeedsPayload = Omit<TechnicalNeeds, 'updated_at'>

export interface AddSocialPostPayload {
  url: string
}
