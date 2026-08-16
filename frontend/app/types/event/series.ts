/**
 * Schéma `event`, partie 1 — séries d'événements.
 * Dérivé de `docs/database/060_events.sql` § 1.
 *
 * DÉCISION DU MODÈLE : la SÉRIE porte l'identité durable (« les COP climat »),
 * l'ÉDITION porte l'occurrence (« COP30 »). C'est ce qui rend une comparaison
 * pluriannuelle possible — la v1 posait une table plate avec une colonne `year`,
 * et rapprocher deux éditions demandait de comparer des titres à la main.
 */

import type { I18nText, IsoDateTime, OrganizationId, Slug, TaxonomyTermCode, Uuid } from '../shared'

/** ENUM `event.series_kind`. */
export type SeriesKind =
  | 'cop_climate'
  | 'cop_biodiversity'
  | 'cop_desertification'
  | 'un_conference'
  | 'webinar_series'
  | 'standalone'
  | 'other'

/** Table `event.event_series` — `060_events.sql` § 1. */
export interface EventSeries {
  id: Uuid
  /** Code stable, `^[a-z][a-z0-9_]*$`. */
  code: string
  kind: SeriesKind
  name: I18nText
  description: I18nText | null
  slug: Slug
  /** Code de la taxonomie `negotiation_track` : croise programmation et espace
   *  de négociation sur la même filière. */
  track_code: TaxonomyTermCode | null
  organizer_organization_id: OrganizationId | null
  is_active: boolean
  created_at: IsoDateTime
  updated_at: IsoDateTime
}
