/**
 * Schéma `event`, partie 2 — éditions, calendrier, fils de programmation.
 * Dérivé de `docs/database/060_events.sql` § 2, 3 et 3 bis.
 *
 * `event.events` est nommée ici `EventEdition` et non `Event` : `Event` est un
 * type global du DOM, et l'ombrer donnerait des messages d'erreur incompréhensibles
 * dans tout composant qui manipule aussi des événements du navigateur.
 */

import type {
  ColorHex,
  Numeric,
  CountryId,
  EventDayId,
  EventId,
  I18nText,
  IsoDate,
  IsoDateTime,
  PersonId,
  Slug,
  TimeZoneName,
  TrackId,
  Uuid,
} from '../shared'

// ---------------------------------------------------------------------------
// Éditions
// ---------------------------------------------------------------------------

/** ENUM `event.event_status`. */
export type EventStatus =
  | 'draft'
  | 'announced'
  | 'ongoing'
  | 'completed'
  | 'cancelled'
  | 'suspended'

/**
 * ENUM `event.participation_mode`.
 * Partagé par les éditions, les appels (`allowed_formats`), les propositions et
 * les sessions : c'est le même vocabulaire d'un bout à l'autre de la chaîne.
 */
export type ParticipationMode = 'online' | 'in_person' | 'hybrid'

/** Table `event.events` — `060_events.sql` § 2. */
export interface EventEdition {
  id: EventId
  /** `event.event_series.id`. Nul pour un rendez-vous hors série. */
  series_id: Uuid | null
  /** Libellé de l'édition dans sa série : « COP30 », « Session 7 ». */
  edition_label: string | null
  edition_year: number
  title: I18nText
  acronym: string | null
  slug: Slug
  description: I18nText
  status: EventStatus
  participation_mode: ParticipationMode
  /** Fuseau de référence. Les horaires sont stockés en `timestamptz` et convertis
   *  à l'affichage — jamais l'inverse. Toute heure affichée le mentionne. */
  timezone: TimeZoneName
  starts_at: IsoDateTime
  ends_at: IsoDateTime
  country_id: CountryId | null
  /**
   * Le pays RÉSOLU, joint par l'API. Il évite à chaque écran de recharger les
   * deux cent quarante-neuf pays du référentiel pour en nommer un seul — c'est
   * ce que faisait la page publique, et cela coûtait une vague d'appels.
   */
  country_code: string | null
  country_name: I18nText | null
  city: string | null
  address: string | null
  /** Point relevé du lieu, facultatif — `ck_events_coordinates` les veut tous
   *  deux ou aucun. Sert à ouvrir un plan : aucun calcul spatial n'en dépend,
   *  d'où deux `numeric` plutôt que PostGIS. */
  latitude: Numeric | null
  longitude: Numeric | null
  /** L'OIF tient-elle un stand sur cette édition ? Sans pavillon, pas d'appel
   *  à propositions : l'IFDD n'envoie qu'un représentant. */
  has_pavilion: boolean
  /** La programmation publique est-elle visible, et depuis quand ? */
  programme_published_at: IsoDateTime | null
  /** Contenus éditoriaux légers (message d'accueil, consignes d'accès). Les
   *  visuels passent par `media.attachments`. */
  highlights: I18nText | null
  created_by: PersonId | null
  created_at: IsoDateTime
  updated_at: IsoDateTime
}

// ---------------------------------------------------------------------------
// Calendrier
// ---------------------------------------------------------------------------

/**
 * Table `event.event_days` — `060_events.sql` § 3.
 * Le CALENDRIER de l'édition : une ligne par jour, rien de plus. Une journée
 * spéciale n'est PAS un jour du calendrier — voir `ProgrammeTrack`.
 */
export interface EventDay {
  id: EventDayId
  event_id: EventId
  day_date: IsoDate
  title: I18nText | null
  /** Alimente la route `/programmation/[event]/[day]`. */
  slug: Slug | null
  description: I18nText | null
  /** Jour remarquable : ouverture, clôture, segment de haut niveau. */
  is_featured: boolean
  color_hex: ColorHex | null
  sort_order: number
  created_at: IsoDateTime
  updated_at: IsoDateTime
}

// ---------------------------------------------------------------------------
// Fils de programmation — les JOURNÉES SPÉCIALES
// ---------------------------------------------------------------------------

/** ENUM `event.track_kind`. */
export type TrackKind = 'special_day' | 'thematic_track' | 'side_programme'

/**
 * Table `event.programme_tracks` — `060_events.sql` § 3 bis.
 *
 * Une journée spéciale (« Journée finance durable ») est composée À LA MAIN par
 * l'IFDD parmi les activités retenues : ce n'est ni un jour du calendrier, ni
 * une déduction faite sur les dates. Le rattachement est explicite et vit dans
 * `programme.session_tracks`. Trois raisons : un fil n'occupe pas forcément le
 * jour entier, un jour peut en porter deux, un fil peut déborder sur deux jours.
 */
export interface ProgrammeTrack {
  id: TrackId
  event_id: EventId
  code: string
  slug: Slug
  kind: TrackKind
  title: I18nText
  subtitle: I18nText | null
  description: I18nText | null
  /** Portée annoncée au public, PUREMENT INDICATIVE : elle ne contraint pas le
   *  rattachement des sessions. */
  starts_on: IsoDate | null
  ends_on: IsoDate | null
  color_hex: ColorHex | null
  /** Responsable du fil au sein de l'IFDD. */
  curated_by: PersonId | null
  /** Page publique du fil ; `null` tant qu'elle n'est pas ouverte. */
  published_at: IsoDateTime | null
  sort_order: number
  created_at: IsoDateTime
  updated_at: IsoDateTime
}
