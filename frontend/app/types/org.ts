/**
 * Schéma `org` — référentiel des organisations, dénominations, domaines,
 * adhésions, détection et fusion des doublons.
 * Dérivé de `docs/database/040_organizations.sql`.
 *
 * RÈGLE MÉTIER N°1 DU PROJET : une organisation, plusieurs dénominations.
 * Chercher « IFDD » ou « Institut de la Francophonie pour le développement
 * durable » doit ramener la même fiche. Le sigle n'est pas un champ à côté du
 * nom : c'est une dénomination de plein droit (`OrganizationName`), indexée
 * comme le nom légal. C'est le défaut n°1 de la v1.
 */

import type {
  Email,
  I18nText,
  IsoDateTime,
  Numeric,
  OrganizationId,
  PersonId,
  Slug,
  TaxonomyTermCode,
  Url,
  Uuid,
} from './shared'

// ---------------------------------------------------------------------------
// Organisations
// ---------------------------------------------------------------------------

/** ENUM `org.organization_status`. */
export type OrganizationStatus = 'candidate' | 'active' | 'merged' | 'archived' | 'rejected'

/** Table `org.organizations` — `040_organizations.sql` § 1. */
export interface Organization {
  id: OrganizationId
  /** Dénomination de référence. Les variantes vivent dans `organization_names`. */
  legal_name: string
  /** Colonne générée : forme canonique du nom légal. */
  readonly legal_name_normalized: string | null
  acronym: string | null
  /** Colonne générée : forme canonique du sigle. */
  readonly acronym_normalized: string | null
  slug: Slug
  /** Code de la taxonomie `organization_type`. */
  organization_type_code: TaxonomyTermCode
  country_id: Uuid | null
  city: string | null
  description: I18nText | null
  website: Url | null
  contact_email: Email | null
  contact_phone: string | null
  status: OrganizationStatus
  /** Fiche absorbante ; renseignée si et seulement si `status` vaut `merged`. */
  merged_into_id: OrganizationId | null
  merged_at: IsoDateTime | null
  /** Vérification par l'IFDD — distincte du statut : une fiche peut être active
   *  sans être vérifiée (elle soumet, son sceau n'est pas affiché). */
  verified_at: IsoDateTime | null
  verified_by: PersonId | null
  /** Score calculé 0-100 : sert au tri du back-office, pas à l'affichage public. */
  trust_score: number
  created_by: PersonId | null
  created_at: IsoDateTime
  updated_at: IsoDateTime
}

// ---------------------------------------------------------------------------
// Dénominations
// ---------------------------------------------------------------------------

/** ENUM `org.name_kind`. */
export type NameKind = 'legal' | 'acronym' | 'short' | 'translation' | 'former' | 'misspelling'

/**
 * Table `org.organization_names` — `040_organizations.sql` § 2.
 * Le nom légal et le sigle de la fiche y sont recopiés automatiquement par
 * trigger : aucune synchronisation à écrire côté application.
 */
export interface OrganizationName {
  id: Uuid
  organization_id: OrganizationId
  name: string
  /** Colonne générée : forme canonique, indexée en trigramme. */
  readonly name_normalized: string | null
  kind: NameKind
  /** Référence `reference.locales.code`. */
  locale: string | null
  /** Une dénomination non confirmée sert la recherche mais ne s'affiche pas. */
  is_confirmed: boolean
  created_by: PersonId | null
  created_at: IsoDateTime
}

// ---------------------------------------------------------------------------
// Domaines
// ---------------------------------------------------------------------------

/** Contrainte `ck` sur `org.organization_domains.verification_method`. */
export type DomainVerificationMethod = 'dns_txt' | 'email_challenge' | 'manual'

/**
 * Table `org.organization_domains` — `040_organizations.sql` § 3.
 * Le signal de dédoublonnage le plus fiable, et le mécanisme de rattachement
 * automatique des nouveaux inscrits.
 */
export interface OrganizationDomain {
  id: Uuid
  organization_id: OrganizationId
  domain: string
  verified_at: IsoDateTime | null
  verification_method: DomainVerificationMethod | null
  /** Rattachement automatique des inscrits portant ce domaine. Exige la vérification. */
  auto_join: boolean
  created_at: IsoDateTime
}

/**
 * Table `org.public_email_domains` — `040_organizations.sql` § 3.
 * Domaines génériques (gmail, outlook…) écartés de tout rapprochement : deux ONG
 * ne sont pas la même parce que leurs référents utilisent la même messagerie.
 */
export interface PublicEmailDomain {
  domain: string
}

// ---------------------------------------------------------------------------
// Adhésions
// ---------------------------------------------------------------------------

/** ENUM `org.membership_role`. */
export type MembershipRole = 'manager' | 'member' | 'contributor'

/** ENUM `org.membership_status`. */
export type MembershipStatus = 'pending' | 'active' | 'revoked'

/** Table `org.memberships` — `040_organizations.sql` § 4. */
export interface Membership {
  id: Uuid
  organization_id: OrganizationId
  person_id: PersonId
  role: MembershipRole
  status: MembershipStatus
  /** Attribué automatiquement à la première adhésion active de la personne. */
  is_primary: boolean
  job_title: string | null
  approved_by: PersonId | null
  approved_at: IsoDateTime | null
  revoked_at: IsoDateTime | null
  created_at: IsoDateTime
  updated_at: IsoDateTime
}

// ---------------------------------------------------------------------------
// Détection et fusion des doublons
// ---------------------------------------------------------------------------

/**
 * Ligne de `org.find_similar_organizations(...)` — `040_organizations.sql` § 5.
 *
 * Appelée par le formulaire de rattachement à chaque frappe (anti-rebond de
 * 300 ms). Un `score` supérieur ou égal à 85 justifie un blocage doux côté
 * interface — « cette organisation existe déjà, souhaitez-vous la rejoindre ? » —
 * jamais un refus.
 */
export interface SimilarOrganization {
  organization_id: OrganizationId
  legal_name: string
  acronym: string | null
  country_id: Uuid | null
  status: OrganizationStatus
  /** Dénomination qui a déclenché la correspondance : à montrer à l'utilisateur
   *  quand elle diffère du nom légal (« trouvée par son sigle »). */
  matched_name: string | null
  score: Numeric
  /** `name_similarity`, `shared_domain`, `same_country`, `acronym_match`. */
  match_reasons: string[]
}

/** Décision portée sur une paire de doublons présumés. */
export type DuplicateDecision = 'merged' | 'distinct' | 'deferred'

/** Table `org.duplicate_candidates` — `040_organizations.sql` § 5. */
export interface DuplicateCandidate {
  id: Uuid
  /** Toujours inférieur à `right_id` (`ck_duplicate_candidates_ordered`). */
  left_id: OrganizationId
  right_id: OrganizationId
  score: Numeric
  reasons: string[]
  detected_at: IsoDateTime
  reviewed_at: IsoDateTime | null
  reviewed_by: PersonId | null
  decision: DuplicateDecision | null
}

/** Stratégie appliquée à une référence lors d'une fusion. */
export type OrganizationReferenceStrategy = 'reassign' | 'delete'

/**
 * Table `org.organization_references` — `040_organizations.sql` § 6.
 * Registre des colonnes pointant vers une organisation ; `merge_organizations()`
 * le parcourt dynamiquement. Chaque module y déclare ses références.
 */
export interface OrganizationReference {
  ref_schema: string
  ref_table: string
  ref_column: string
  strategy: OrganizationReferenceStrategy
  /** Colonnes formant une unicité AVEC la colonne d'organisation : les lignes en
   *  conflit côté source sont supprimées avant la bascule. */
  dedupe_on: string[]
}

/** Table `org.merge_log` — `040_organizations.sql` § 6. */
export interface MergeLogEntry {
  id: Uuid
  source_id: OrganizationId
  /** Fiche absorbée avant fusion, telle quelle : permet une reprise manuelle. */
  source_snapshot: Record<string, unknown>
  target_id: OrganizationId
  performed_by: PersonId | null
  performed_at: IsoDateTime
  /** Nombre de lignes réaffectées, par `schéma.table.colonne`. */
  rows_reassigned: Record<string, number>
  reason: string | null
}
