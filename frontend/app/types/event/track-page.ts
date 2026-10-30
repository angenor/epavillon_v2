/**
 * La page publique d'un fil (journée spéciale, atelier préparatoire), vue du
 * back-office. Dérivé de `docs/database/060_events.sql` § 3 bis et § 4 ter, et
 * de `programme.session_tracks` (075 § 2 bis) pour la composition.
 *
 * Routes : `GET|PUT /admin/tracks/{id}/page`, `POST /admin/tracks/{id}/parts`,
 * `PUT|DELETE /admin/tracks/{id}/parts/{partId}`, `POST /admin/tracks/{id}/moments`,
 * `PUT|DELETE /admin/tracks/{id}/moments/{momentId}`. Toutes rendent
 * `TrackPageSettings` entier. Le placement d'une activité dans une partie est
 * dans `programme/track-page.ts` (`TrackSessionPlacementPayload`).
 */

import type { AttachedImage } from '../media'
import type {
  EventId,
  I18nText,
  IsoDateTime,
  OrganizationId,
  SessionId,
  TrackId,
  Uuid,
  VenueId,
} from '../shared'
import type { SessionStatus } from '../programme/session'

export type TrackPartId = Uuid
export type TrackMomentId = Uuid

/** Code de la taxonomie `track_partner_role` (vocabulaire ouvert, libellé en base). */
export type TrackPartnerRoleCode = string

/** Table `event.programme_track_parts`. */
export interface TrackPart {
  id: TrackPartId
  track_id: TrackId
  /** Nom court (« Matin », « S'inspirer ») ; nul : « Partie N ». */
  label: I18nText | null
  /** Intitulé long. Au moins l'un des deux, `label` ou `title`. */
  title: I18nText | null
  /** Version d'une ligne pour le sommaire « La journée en trois temps ». */
  summary: I18nText | null
  sort_order: number
}

/** Table `event.programme_track_moments` : accueil, déjeuner, clôture. Pas une activité. */
export interface TrackMoment {
  id: TrackMomentId
  track_id: TrackId
  part_id: TrackPartId | null
  starts_at: IsoDateTime
  /** Nulle : la clôture n'a qu'une heure de début. */
  ends_at: IsoDateTime | null
  label: I18nText
  description: I18nText | null
}

/** Table `event.programme_track_partners`, résolue. */
export interface TrackPartner {
  organization_id: OrganizationId
  name: string
  acronym: string | null
  logo: AttachedImage | null
  role_code: TrackPartnerRoleCode
  /** Donnée de `reference.taxonomy_terms`, jamais une traduction. */
  role_label: I18nText
  sort_order: number
}

/** Une activité rattachée au fil (`programme.session_tracks`), publiée ou non. */
export interface TrackCompositionEntry {
  session_id: SessionId
  slug: string
  title: I18nText
  starts_at: IsoDateTime
  ends_at: IsoDateTime
  status: SessionStatus
  is_published: boolean
  part_id: TrackPartId | null
  is_highlight: boolean
  sort_order: number
}

/** `GET /admin/tracks/{id}/page`. */
export interface TrackPageSettings {
  track_id: TrackId
  event_id: EventId
  /** Nul : le pavillon de l'édition. */
  venue_id: VenueId | null
  tagline: I18nText | null
  target_audiences: I18nText[]
  support_note: I18nText | null
  feature_kicker: I18nText | null
  feature_title: I18nText | null
  feature_body: I18nText | null
  feature_link_label: I18nText | null
  /** Adresse absolue, ou chemin du site (« /guide-nego »). */
  feature_link_url: string | null
  parts: TrackPart[]
  moments: TrackMoment[]
  partners: TrackPartner[]
  sessions: TrackCompositionEntry[]
}

export interface TrackPartnerPayload {
  organization_id: OrganizationId
  /** `partner` par défaut. */
  role_code?: TrackPartnerRoleCode
}

/**
 * `PUT /admin/tracks/{id}/page` : écriture TOTALE de ces champs et
 * REMPLACEMENT des partenaires (l'ordre envoyé est l'ordre affiché).
 */
export interface TrackPagePayload {
  venue_id: VenueId | null
  tagline: I18nText | null
  target_audiences: I18nText[]
  support_note: I18nText | null
  feature_kicker: I18nText | null
  feature_title: I18nText | null
  feature_body: I18nText | null
  /** Donné avec `feature_link_url`, ou aucun des deux. */
  feature_link_label: I18nText | null
  feature_link_url: string | null
  partners: TrackPartnerPayload[]
}

export interface TrackPartPayload {
  label: I18nText | null
  title: I18nText | null
  summary: I18nText | null
  sort_order: number
}

export interface TrackMomentPayload {
  part_id: TrackPartId | null
  starts_at: IsoDateTime
  ends_at: IsoDateTime | null
  label: I18nText
  description: I18nText | null
}
