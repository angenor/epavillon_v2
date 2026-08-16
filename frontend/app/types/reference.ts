/**
 * Schéma `reference` — pays, locales, taxonomies.
 * Dérivé de `docs/database/020_reference.sql`.
 *
 * Rappel du modèle : les vocabulaires métier (thématiques, catégories, secteurs,
 * types d'organisation, types de document, canaux d'acquisition, licences) sont
 * des DONNÉES administrables, pas des ENUM et surtout pas des libellés i18n.
 * Un écran affiche `resolveI18nText(term.label)` ; il ne connaît que `term.code`.
 */

import type {
  ColorHex,
  CountryId,
  I18nText,
  IsoDateTime,
  TermId,
  TimeZoneName,
  Uuid,
} from './shared'

// ---------------------------------------------------------------------------
// Locales
// ---------------------------------------------------------------------------

/** Table `reference.locales` — `020_reference.sql` § 1. */
export interface Locale {
  /** Clé primaire : `^[a-z]{2}(-[A-Z]{2})?$`, ex. `fr`, `fr-CA`. */
  code: string
  native_label: string
  english_label: string
  is_default: boolean
  is_active: boolean
  sort_order: number
  /** `regconfig` du dictionnaire plein texte associé, ex. `french`. */
  text_search_config: string
}

// ---------------------------------------------------------------------------
// Pays
// ---------------------------------------------------------------------------

/** ENUM `reference.oif_membership`. */
export type OifMembership = 'member' | 'associate' | 'observer' | 'none'

/** Contrainte `ck` sur `reference.countries.continent`. */
export type Continent =
  | 'africa'
  | 'europe'
  | 'asia'
  | 'oceania'
  | 'north_america'
  | 'south_america'
  | 'antarctica'

/** Table `reference.countries` — `020_reference.sql` § 2. */
export interface Country {
  id: CountryId
  iso2: string
  iso3: string
  numeric_code: string | null
  name: I18nText
  official_name: I18nText | null
  /** Colonne générée : forme canonique du nom français, base de la recherche floue. */
  readonly name_normalized: string | null
  /** Ex. `africa-west`, `europe`, `caribbean`. */
  region_code: string | null
  continent: Continent | null
  /** Statut au sein de l'OIF — donnée structurante pour l'IFDD. */
  oif_status: OifMembership
  default_timezone: TimeZoneName | null
  calling_code: string | null
  flag_emoji: string | null
  is_active: boolean
  created_at: IsoDateTime
  updated_at: IsoDateTime
}

// ---------------------------------------------------------------------------
// Taxonomies
// ---------------------------------------------------------------------------

/** Table `reference.taxonomies` — `020_reference.sql` § 3. */
export interface Taxonomy {
  /** Clé primaire, ex. `activity_theme`, `organization_type`. */
  code: string
  label: I18nText
  description: I18nText | null
  /** Un terme peut-il être choisi plusieurs fois sur une même entité ? */
  is_multi_select: boolean
  is_hierarchical: boolean
  /** Taxonomie verrouillée : ses codes pilotent des règles métier. */
  is_system: boolean
  created_at: IsoDateTime
}

/** Table `reference.taxonomy_terms` — `020_reference.sql` § 3. */
export interface TaxonomyTerm {
  id: TermId
  taxonomy_code: string
  parent_id: TermId | null
  /** Code stable : c'est lui que manipulent le frontend et les exports. */
  code: string
  label: I18nText
  description: I18nText | null
  /** Pastille du calendrier et des filtres — évite les `switch` de couleurs. */
  color_hex: ColorHex | null
  icon: string | null
  sort_order: number
  is_active: boolean
  /** Terme déprécié : pointe vers son successeur, jamais supprimé. */
  superseded_by: TermId | null
  metadata: Record<string, unknown>
  created_at: IsoDateTime
  updated_at: IsoDateTime
}

/**
 * Table `reference.entity_terms` — `020_reference.sql` § 4.
 * Rattachement N-N d'une entité de n'importe quel module à des termes.
 */
export interface EntityTerm {
  entity_schema: string
  entity_table: string
  entity_id: Uuid
  term_id: TermId
  /** Distingue « thème principal » de « thème secondaire » sur une même taxonomie. */
  role: string
  sort_order: number
  created_at: IsoDateTime
}
