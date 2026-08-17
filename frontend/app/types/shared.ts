/**
 * Types transverses, dérivés de `docs/database/000_bootstrap.sql`
 * (domaines réutilisables, § 5) et de `010_platform.sql` (drapeaux).
 *
 * Les types d'entités vivent dans les fichiers voisins, un par groupe :
 * `reference.ts`, `identity.ts`, `org.ts`, `media.ts`, `event/`, `programme/`,
 * `views.ts`. Ce fichier ne porte que ce qu'ils partagent tous.
 */

/** Clé primaire : `uuid` généré par `platform.uuid_v7()`. */
export type Uuid = string

/** `timestamptz` sérialisé en ISO 8601 avec décalage — jamais une date locale nue. */
export type IsoDateTime = string

/** `date` PostgreSQL, au format `AAAA-MM-JJ`. */
export type IsoDate = string

/** Identifiant de fuseau IANA — domaine `platform.timezone_name`, ex. `America/Belem`. */
export type TimeZoneName = string

/** Domaine `platform.email` (citext validé). */
export type Email = string

/** Domaine `platform.slug` — segment d'URL stable. */
export type Slug = string

/** Domaine `platform.url` — adresse absolue http(s). */
export type Url = string

/** Codes des locales actives dans `reference.locales`. */
export type LocaleCode = 'fr' | 'en'

/**
 * Domaine `platform.i18n_text` : `{"fr": "…", "en": "…"}`.
 * Le français est obligatoire en base — la langue pivot de la Francophonie —,
 * les autres locales sont facultatives et ajoutables sans migration.
 *
 * À ne pas confondre avec les fichiers i18n : ceci est une DONNÉE saisie au
 * back-office (titre d'activité, libellé de thématique, nom de salle), pas un
 * libellé d'interface. Voir `resolveI18nText()`.
 */
export interface I18nText {
  fr: string
  /**
   * `string | undefined` et non `string` : une locale ABSENTE est le cas normal
   * — le français est la seule obligation de la base. La signature stricte
   * refusait d'ailleurs toute donnée n'ayant qu'un français, l'inférence d'un
   * tableau hétérogène produisant `{ fr: string; en?: undefined }` : les fiches
   * de démonstration ne s'assignaient plus à leur propre type dès qu'on
   * l'annotait. `resolveI18nText()` traite l'absence depuis toujours (`isFilled`),
   * c'est le type qui promettait plus que la donnée. Corrigé au prompt A2.
   */
  [locale: string]: string | undefined
}

/**
 * `numeric(p, s)` PostgreSQL. Sérialisé en nombre JSON par l'API : les échelles
 * du modèle (notes sur 5, pondérations, scores sur 20) tiennent toutes dans un
 * `number` sans perte.
 */
export type Numeric = number

/**
 * `bigint` PostgreSQL — tailles de fichier et quotas de `media`. Sérialisé en
 * nombre : 5 Gio valent 5 368 709 120, très en deçà de `Number.MAX_SAFE_INTEGER`.
 */
export type Int8 = number

/** `tstzrange` sérialisé, ex. `["2027-11-08T14:00:00+00","2027-11-08T16:00:00+00")`. */
export type TsTzRange = string

/** `inet` PostgreSQL — adresse IPv4 ou IPv6. */
export type IpAddress = string

/** Couleur de la charte d'un terme, d'un jour ou d'un fil : `#RRGGBB`. */
export type ColorHex = string

/**
 * Code d'un terme de `reference.taxonomy_terms` (`^[a-z][a-z0-9_]*$`).
 * Le CODE est stable et voyage dans l'application ; le LIBELLÉ vient de la base
 * et se résout par `resolveI18nText()`. Ne jamais figer un libellé en i18n.
 */
export type TaxonomyTermCode = string

/** Code d'une permission `identity.permissions` : `module.ressource.action`. */
export type PermissionCode = string

/** Code d'un rôle `identity.roles` (`^[a-z][a-z0-9_]*$`). */
export type RoleCode = string

/** Clé d'un drapeau `platform.feature_flags`, ex. `publications.enabled`. */
export type FeatureFlagKey = string

// ---------------------------------------------------------------------------
// Alias d'identifiants
//
// Ce sont des alias documentaires, pas des types marqués : TypeScript les tient
// pour interchangeables avec `Uuid`. Ils existent pour qu'une signature dise ce
// qu'elle attend (`loadProposal(id: ProposalId)`), pas pour empêcher une
// confusion à la compilation — une marque obligerait chaque donnée simulée et
// chaque réponse d'API à une conversion, pour un gain théorique.
// ---------------------------------------------------------------------------

export type CountryId = Uuid
export type TermId = Uuid
export type PersonId = Uuid
export type AccountId = Uuid
export type RoleAssignmentId = Uuid
export type OrganizationId = Uuid
export type AssetId = Uuid
export type EventId = Uuid
export type EventDayId = Uuid
export type TrackId = Uuid
export type VenueId = Uuid
export type RoomId = Uuid
export type BroadcastChannelId = Uuid
export type CallId = Uuid
export type CriterionId = Uuid
export type ProposalId = Uuid
export type ReviewId = Uuid
export type SessionId = Uuid
export type RegistrationFormId = Uuid
export type RegistrationId = Uuid

/** Enveloppe d'une liste paginée renvoyée par l'API. */
export interface Paginated<T> {
  items: T[]
  total: number
  page: number
  perPage: number
}
