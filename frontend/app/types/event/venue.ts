/**
 * Schéma `event`, partie 3 — lieux, salles, canaux de diffusion.
 * Dérivé de `docs/database/060_events.sql` § 4 et 4 bis.
 *
 * Ces trois tables donnent un SUJET aux conflits de créneaux : sans elles,
 * `detect_conflicts()` ne saurait dire que « la salle Baobab est réservée deux
 * fois », seulement qu'« il y a deux activités à 14 h ». Nommer n'est pas
 * interdire : le chevauchement reste parfaitement écrivable.
 */

import type {
  BroadcastChannelId,
  EventId,
  I18nText,
  IsoDateTime,
  RoomId,
  Url,
  VenueId,
} from '../shared'

// ---------------------------------------------------------------------------
// Lieux et salles
// ---------------------------------------------------------------------------

/** Contrainte `ck` sur `event.venues.kind`. */
export type VenueKind = 'pavilion' | 'partner' | 'plenary' | 'virtual' | 'other'

/** Table `event.venues` — `060_events.sql` § 4. */
export interface Venue {
  id: VenueId
  event_id: EventId
  name: I18nText
  /** `pavilion` : le stand de la Francophonie. */
  kind: VenueKind
  address: string | null
  map_url: Url | null
  created_at: IsoDateTime
}

/** Table `event.rooms` — `060_events.sql` § 4. */
export interface Room {
  id: RoomId
  venue_id: VenueId
  name: I18nText
  code: string
  capacity: number | null
  /** Une salle virtuelle accepte des sessions simultanées : `detect_conflicts()`
   *  ne signale pas de double réservation dessus. */
  is_virtual: boolean
  has_streaming: boolean
  equipment: string[]
  sort_order: number
  created_at: IsoDateTime
}

// ---------------------------------------------------------------------------
// Canaux de diffusion
// ---------------------------------------------------------------------------

/** Contrainte `ck` sur `event.broadcast_channels.provider`. */
export type BroadcastProvider =
  | 'youtube'
  | 'vimeo'
  | 'facebook'
  | 'linkedin'
  | 'dailymotion'
  | 'custom'

/**
 * Table `event.broadcast_channels` — `060_events.sql` § 4 bis.
 *
 * RÈGLE MÉTIER : un seul direct à la fois — une seule équipe technique, un seul
 * flux. Le canal est donc une RESSOURCE RÉSERVABLE, au même titre qu'une salle.
 * Deux sessions diffusées qui l'occupent sur des créneaux qui se recouvrent sont
 * remontées en gravité `blocking`, sans que l'écriture soit refusée.
 */
export interface BroadcastChannel {
  id: BroadcastChannelId
  /** Nul pour un canal général de la plateforme, hors événement. */
  event_id: EventId | null
  code: string
  name: I18nText
  provider: BroadcastProvider
  /** Compte diffuseur, ex. `@ifddoif`. */
  channel_ref: string | null
  /** Référence `reference.locales.code`. */
  locale: string | null
  /** Canal retenu d'office pour toute session marquée comme diffusée : la règle
   *  « un seul direct » ne doit pas dépendre d'une saisie. */
  is_default: boolean
  is_active: boolean
  created_at: IsoDateTime
  updated_at: IsoDateTime
}
