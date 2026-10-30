/**
 * La page publique d'un fil, en une réponse : `GET /events/{event_slug}/tracks/{track_slug}`.
 * Sans session ; un fil non publié, une édition en brouillon ou annulée et une
 * adresse inconnue rendent le même 404.
 *
 * Le déroulé (`agenda`) fusionne les moments sans activité
 * (`event.programme_track_moments`) et les activités PUBLIÉES du fil
 * (`programme.session_tracks`), triés par heure. Les heures de la journée et
 * de chaque partie sont lues sur ce déroulé, jamais saisies.
 *
 * Toute heure se lit dans `edition.timezone`.
 */

import type { AttachedDocument, AttachedImage } from '../media'
import type {
  EventId,
  I18nText,
  IsoDate,
  IsoDateTime,
  OrganizationId,
  SessionId,
  TrackId,
  VenueId,
} from '../shared'
import type { TrackKind } from '../event/edition'
import type { VenueKind } from '../event/venue'
import type { TrackMomentId, TrackPartId, TrackPartnerRoleCode } from '../event/track-page'
import type { PublicScheduleRow, ScheduleThemeBadge } from '../views'
import type { PublicSessionSpeaker, SessionOrganization } from './session'

export interface PublicTrackEdition {
  id: EventId
  slug: string
  title: I18nText
  acronym: string | null
  /** Fuseau de référence de toute heure de la page (« heure d'Antalya »). */
  timezone: string
  starts_at: IsoDateTime
  ends_at: IsoDateTime
  city: string | null
  country_name: I18nText | null
}

export interface PublicTrackHeader {
  id: TrackId
  slug: string
  code: string
  kind: TrackKind
  title: I18nText
  subtitle: I18nText | null
  description: I18nText | null
  /** Accroche en pastille : « Hackathon, pitchs, jeu, scène ». */
  tagline: I18nText | null
  starts_on: IsoDate | null
  ends_on: IsoDate | null
  color_hex: string | null
  themes: ScheduleThemeBadge[]
}

/** Le lieu du fil, à défaut le pavillon de l'édition. */
export interface PublicTrackVenue {
  id: VenueId
  name: I18nText
  kind: VenueKind
  address: string | null
  map_url: string | null
  /** « Zone bleue ». */
  area: I18nText | null
  /** « En ville, hors de la zone bleue. » */
  access_note: I18nText | null
  latitude: number | null
  longitude: number | null
}

/** L'encart mis en avant (le Guide des négociations pour l'atelier). */
export interface PublicTrackFeature {
  /** « Présenté pendant l'atelier ». */
  kicker: I18nText | null
  title: I18nText
  body: I18nText | null
  link_label: I18nText | null
  /** Adresse absolue, ou chemin du site (« /guide-nego »). */
  link_url: string | null
  /** La couverture du Guide. */
  visual: AttachedImage | null
}

export interface PublicTrackPartner {
  organization_id: OrganizationId
  name: string
  acronym: string | null
  logo: AttachedImage | null
  role_code: TrackPartnerRoleCode
  role_label: I18nText
}

export interface PublicTrackPart {
  id: TrackPartId
  /** Rang à partir de 1 : « Partie 1 » quand `label` est nul. */
  number: number
  label: I18nText | null
  title: I18nText | null
  summary: I18nText | null
  /** Lues sur le contenu de la partie ; nulles si elle est vide. */
  starts_at: IsoDateTime | null
  ends_at: IsoDateTime | null
}

export interface PublicTrackActivity {
  /** La ligne de la programmation publique ; lien : slug de l'édition + `session.slug`. */
  session: PublicScheduleRow
  is_highlight: boolean
  speakers: PublicSessionSpeaker[]
  /** Porteur en tête, logo compris. */
  organizations: SessionOrganization[]
}

export interface PublicTrackMoment {
  id: TrackMomentId
  label: I18nText
  description: I18nText | null
}

interface PublicTrackEntryBase {
  starts_at: IsoDateTime
  /** Nulle pour un moment sans fin (la clôture). */
  ends_at: IsoDateTime | null
  part_id: TrackPartId | null
}

export type PublicTrackEntry =
  | (PublicTrackEntryBase & { kind: 'activity'; activity: PublicTrackActivity })
  | (PublicTrackEntryBase & { kind: 'moment'; moment: PublicTrackMoment })

/** Un fil publié de l'édition, pour relier les pages (« Les autres temps forts »). */
export interface PublicTrackSummary {
  slug: string
  kind: TrackKind
  title: I18nText
  color_hex: string | null
  starts_on: IsoDate | null
  ends_on: IsoDate | null
  starts_at: IsoDateTime | null
  ends_at: IsoDateTime | null
  venue_name: I18nText | null
  is_off_site: boolean
  /** La page qu'on lit. */
  is_current: boolean
}

export interface PublicTrackPage {
  edition: PublicTrackEdition
  track: PublicTrackHeader
  venue: PublicTrackVenue | null
  /** Le fil se tient ailleurs qu'au pavillon : « Hors du pavillon ». */
  is_off_site: boolean
  /** Photo du bandeau ; nulle pour l'atelier, qui garde l'aplat. */
  cover: AttachedImage | null
  /** Programme en PDF, Résumé du Guide ; libellé = `caption`. */
  documents: AttachedDocument[]
  feature: PublicTrackFeature | null
  target_audiences: I18nText[]
  support_note: I18nText | null
  partners: PublicTrackPartner[]
  parts: PublicTrackPart[]
  agenda: PublicTrackEntry[]
  /** Première activité (« 09:00 »). */
  starts_at: IsoDateTime | null
  /** Dernière fin, ou début d'un moment sans fin (« à 18:00 »). */
  ends_at: IsoDateTime | null
  /** Premier moment s'il précède : « accueil dès 08:00 ». */
  opens_at: IsoDateTime | null
  other_tracks: PublicTrackSummary[]
}

/** `PUT /admin/planner/tracks/{trackId}/sessions/{sessionId}`. */
export interface TrackSessionPlacementPayload {
  /** Partie de CE fil ; nulle : hors partie. */
  part_id: TrackPartId | null
  is_highlight: boolean
  sort_order: number
}

export interface TrackSessionPlacement {
  session_id: SessionId
  track_id: TrackId
  part_id: TrackPartId | null
  is_highlight: boolean
  sort_order: number
}
