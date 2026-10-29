/**
 * L'atelier de l'image de couverture d'un dossier, `070_programme_proposals.sql`
 * § 4 ter : `proposal_cover_designs`, et le rôle `cover_source` de `050`.
 *
 * Modèles et couleurs vivent dans le site (`utils/cover-studio`) : la base n'en
 * garde que les codes. L'image fabriquée est le rattachement `cover` du dossier.
 */

import type { IsoDateTime } from '../shared'
import type { AttachedImage } from '../media'

export interface CoverDesignOptions {
  show_date: boolean
  show_speakers: boolean
  show_logos: boolean
  /** Le nom est d'origine : vrai, la photo de fond est l'image de l'édition ; faux, celle de l'organisation (`cover_source`). */
  show_event_banner: boolean
  show_event_icon: boolean
}

export interface CoverDesign {
  /** Code d'un modèle défini dans le site (`utils/cover-studio`). */
  template_code: string
  /** Code d'une couleur de la charte, défini dans le site. */
  accent: string
  options: CoverDesignOptions
  updated_at: IsoDateTime
}

/** Tout ce que l'atelier lit, en une fois. */
export interface CoverStudioContext {
  /** Nul tant que l'atelier n'a jamais été enregistré pour ce dossier. */
  design: CoverDesign | null
  /** Rôle `cover_source` du dossier. */
  source_photo: AttachedImage | null
  /** Rôle `cover` (16:9) de l'édition : la photo de fond par défaut. */
  edition_cover: AttachedImage | null
  /** Rôle `banner` de l'édition (event.events). */
  edition_banner: AttachedImage | null
  /** Rôle `thumbnail` de l'édition. */
  edition_thumbnail: AttachedImage | null
  /** Rôle `logo` de l'édition : l'icône le préfère à la vignette. */
  edition_logo: AttachedImage | null
  /** Rôle `logo` de l'organisation porteuse. */
  organization_logo: AttachedImage | null
}

export interface SaveCoverDesignPayload {
  template_code: string
  accent: string
  options: CoverDesignOptions
}
