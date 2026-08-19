/**
 * Schéma `identity` — personnes, comptes, authentification, RBAC scopé, RGPD.
 * Dérivé de `docs/database/030_identity.sql`.
 *
 * DÉCISION DU MODÈLE, à ne pas perdre de vue en écrivant les écrans : la
 * PERSONNE (`people`) et le COMPTE (`accounts`) sont deux choses distinctes. Une
 * personne existe sans compte — invité inscrit à un webinaire, intervenant saisi
 * par un tiers. Toute la plateforme référence `people`.
 *
 * COLONNES ABSENTES, VOLONTAIREMENT : les secrets d'authentification
 * (`accounts.password_hash`, `mfa_secret_encrypted`, `mfa_recovery_codes`,
 * `sessions.refresh_token_hash`, `one_time_tokens.token_hash`) ne quittent
 * jamais l'API. Les déclarer ici inviterait un écran à les demander. De même,
 * `people.search_vector` (`tsvector`) n'a pas de représentation utile côté
 * client. Voir l'écart consigné dans `docs/progression/decisions/2026-08-16.md`.
 */

import type {
  AccountId,
  AssetId,
  CountryId,
  Email,
  I18nText,
  IpAddress,
  IsoDateTime,
  OrganizationId,
  PermissionCode,
  PersonId,
  RoleAssignmentId,
  RoleCode,
  TaxonomyTermCode,
  TimeZoneName,
  Uuid,
} from './shared'

// ---------------------------------------------------------------------------
// Personnes
// ---------------------------------------------------------------------------

/** ENUM `identity.person_status`. */
export type PersonStatus = 'active' | 'suspended' | 'blocked' | 'anonymized'

/** Contrainte `ck` sur `identity.people.civility`. */
export type Civility = 'mme' | 'm' | 'dr' | 'pr' | 'other'

/** Table `identity.people` — `030_identity.sql` § 1. */
export interface Person {
  id: PersonId
  /** Email canonique : clé de rapprochement entre un invité et son futur compte. */
  primary_email: Email
  email_verified_at: IsoDateTime | null
  first_name: string
  last_name: string
  civility: Civility | null
  /** Colonne générée : `first_name + ' ' + last_name`. */
  readonly display_name: string
  phone: string | null
  job_title: string | null
  biography: I18nText | null
  country_id: CountryId | null
  city: string | null
  /** Référence `reference.locales.code`. */
  preferred_locale: string
  timezone: TimeZoneName
  /** Rattachement principal, maintenu par la base depuis l'adhésion principale. */
  primary_organization_id: OrganizationId | null
  status: PersonStatus
  status_reason: string | null
  status_changed_by: PersonId | null
  status_changed_at: IsoDateTime | null
  /** Obligatoire quand `status` vaut `suspended` (`ck_people_suspension_window`). */
  suspended_until: IsoDateTime | null
  is_directory_visible: boolean
  created_at: IsoDateTime
  updated_at: IsoDateTime
}

/** Contrainte `ck` sur `identity.person_emails.label`. */
export type PersonEmailLabel = 'secondary' | 'professional' | 'legacy'

/** Table `identity.person_emails` — `030_identity.sql` § 1. */
export interface PersonEmail {
  id: Uuid
  person_id: PersonId
  email: Email
  label: PersonEmailLabel
  verified_at: IsoDateTime | null
  created_at: IsoDateTime
}

// ---------------------------------------------------------------------------
// Comptes et authentification
// ---------------------------------------------------------------------------

/** ENUM `identity.auth_provider`. */
export type AuthProvider = 'password' | 'google' | 'microsoft' | 'linkedin' | 'oidc'

/**
 * Table `identity.accounts` — `030_identity.sql` § 2, secrets exclus.
 * Un compte = un moyen de se connecter. Une personne peut en cumuler plusieurs.
 */
export interface Account {
  id: AccountId
  person_id: PersonId
  provider: AuthProvider
  /** Identifiant chez le fournisseur ; `null` pour `password` (l'email sert). */
  provider_subject: string | null
  password_changed_at: IsoDateTime | null
  mfa_enabled_at: IsoDateTime | null
  last_login_at: IsoDateTime | null
  failed_attempts: number
  locked_until: IsoDateTime | null
  created_at: IsoDateTime
  updated_at: IsoDateTime
}

/**
 * Table `identity.sessions` — `030_identity.sql` § 2, empreinte du jeton exclue.
 * Nommée `AuthSession` pour ne pas entrer en collision avec `Session`, la séance
 * programmée de `programme.sessions`.
 */
export interface AuthSession {
  id: Uuid
  person_id: PersonId
  account_id: AccountId | null
  user_agent: string | null
  ip_address: IpAddress | null
  issued_at: IsoDateTime
  expires_at: IsoDateTime
  last_seen_at: IsoDateTime
  revoked_at: IsoDateTime | null
  revoked_reason: string | null
}

/** ENUM `identity.token_purpose`. */
export type TokenPurpose =
  | 'email_verification'
  | 'password_reset'
  | 'invitation'
  | 'magic_link'
  | 'speaker_confirmation'

/** Table `identity.one_time_tokens` — `030_identity.sql` § 2, empreinte exclue. */
export interface OneTimeToken {
  id: Uuid
  person_id: PersonId | null
  purpose: TokenPurpose
  payload: Record<string, unknown>
  expires_at: IsoDateTime
  consumed_at: IsoDateTime | null
  created_at: IsoDateTime
}

// ---------------------------------------------------------------------------
// RBAC scopé
//
// Une attribution de rôle porte TOUJOURS une portée. Le code teste une
// permission (`has_permission`), jamais un nom de rôle.
// ---------------------------------------------------------------------------

/** ENUM `identity.scope_type`. */
export type ScopeType = 'global' | 'organization' | 'event' | 'negotiation_space'

/** Table `identity.permissions` — `030_identity.sql` § 3. */
export interface Permission {
  /** Clé primaire : `module.ressource.action`. */
  code: PermissionCode
  label: I18nText
  /** Référence `platform.modules.code`. */
  module_code: string
}

/** Table `identity.roles` — `030_identity.sql` § 3. */
export interface Role {
  code: RoleCode
  label: I18nText
  description: I18nText | null
  /** Portées sur lesquelles ce rôle peut être attribué, ex. `['global', 'event']`. */
  allowed_scopes: ScopeType[]
  /** Rôle système : non supprimable. */
  is_system: boolean
  created_at: IsoDateTime
}

/** Table `identity.role_permissions` — `030_identity.sql` § 3. */
export interface RolePermission {
  role_code: RoleCode
  permission_code: PermissionCode
}

/** Table `identity.role_assignments` — `030_identity.sql` § 3. */
export interface RoleAssignment {
  id: RoleAssignmentId
  person_id: PersonId
  role_code: RoleCode
  scope_type: ScopeType
  /** `null` si et seulement si `scope_type` vaut `global`. Aucune FK : la cible
   *  vit dans un autre module et peut devenir un service distant. */
  scope_id: Uuid | null
  granted_by: PersonId | null
  granted_at: IsoDateTime
  valid_from: IsoDateTime
  valid_until: IsoDateTime | null
  revoked_at: IsoDateTime | null
  /**
   * Qui a retiré le rôle, et pourquoi. Colonnes AJOUTÉES AU MODÈLE au prompt
   * A12 : `note` est le motif de l'OCTROI et ne pouvait pas servir deux fois.
   * `ck_role_assignment_revocation` interdit de les renseigner tant que
   * `revoked_at` est nul — une attribution vivante portant « fin de mission » se
   * lirait comme révoquée dans toute liste qui ne teste pas la date.
   */
  revoked_by: PersonId | null
  revoked_reason: string | null
  /** Motif de l'OCTROI, saisi à l'attribution. */
  note: string | null
}

/**
 * Ligne de `identity.effective_permissions(person_id)` — `030_identity.sql` § 3.
 * Consommée pour composer le jeton d'accès et pour afficher ou masquer les
 * entrées du menu d'administration.
 */
export interface EffectivePermission {
  permission_code: PermissionCode
  scope_type: ScopeType
  scope_id: Uuid | null
}

/**
 * Retour de `identity.administered_events(person_id)` — `030_identity.sql` § 3.
 *
 * La fonction agrège sans `GROUP BY` : elle renvoie TOUJOURS exactement une
 * ligne, y compris pour une personne sans aucune attribution. Depuis la
 * correction du 16/08, ses deux colonnes ne valent JAMAIS `null` — les trois
 * cas se lisent donc sans ambiguïté :
 *
 *   { is_global: true,  event_ids: [] }      administrateur de la plateforme
 *   { is_global: false, event_ids: [id…] }   administrateur des éditions listées
 *   { is_global: false, event_ids: [] }      aucun droit → accès refusé
 *
 * Quand `is_global` vaut `true`, `event_ids` n'est plus signifiant : une
 * attribution globale couvre toutes les éditions, présentes et à venir.
 *
 * TOUTE liste du back-office se filtre là-dessus — `is_global || event_ids.includes(id)` —
 * y compris quand l'utilisateur forge une URL.
 */
export interface AdministeredEvents {
  is_global: boolean
  event_ids: Uuid[]
}

// ---------------------------------------------------------------------------
// Profil négociateur
//
// Extension de profil, et non un rôle : la désignation annuelle est une donnée
// métier, l'accès à l'espace de négociation reste porté par le RBAC. L'espace
// Négociations lui-même est hors du jalon en cours.
// ---------------------------------------------------------------------------

/** Table `identity.negotiator_profiles` — `030_identity.sql` § 4. */
export interface NegotiatorProfile {
  id: Uuid
  person_id: PersonId
  country_id: CountryId | null
  is_unfccc_focal_point: boolean
  /** Codes de `reference.taxonomy_terms`. */
  specializations: TaxonomyTermCode[]
  notes: string | null
  created_at: IsoDateTime
  updated_at: IsoDateTime
}

/** Table `identity.negotiator_designations` — `030_identity.sql` § 4. */
export interface NegotiatorDesignation {
  id: Uuid
  profile_id: Uuid
  year: number
  /** Code de la taxonomie `negotiation_track` ; défaut `climate`. */
  track_code: TaxonomyTermCode
  designated_by: PersonId | null
  designated_at: IsoDateTime
}

// ---------------------------------------------------------------------------
// RGPD
// ---------------------------------------------------------------------------

/** Table `identity.consents` — `030_identity.sql` § 5. */
export interface Consent {
  id: Uuid
  person_id: PersonId
  /** Ex. `newsletter`, `directory_listing`, `photo_usage`, `analytics`. */
  purpose: string
  is_granted: boolean
  policy_version: string
  /** Ex. `registration_form`, `profile_settings`, `import`. */
  source: string | null
  ip_address: IpAddress | null
  recorded_at: IsoDateTime
}

/**
 * Vue `identity.current_consents` — `030_identity.sql` § 5.
 * État courant de chaque finalité ; l'historique complet reste dans `consents`.
 */
export interface CurrentConsent {
  person_id: PersonId
  purpose: string
  is_granted: boolean
  policy_version: string
  recorded_at: IsoDateTime
}

/** ENUM `identity.privacy_request_type`. */
export type PrivacyRequestType = 'export' | 'erasure' | 'rectification'

/** ENUM `identity.privacy_request_status`. */
export type PrivacyRequestStatus = 'received' | 'in_progress' | 'completed' | 'rejected'

/** Table `identity.privacy_requests` — `030_identity.sql` § 5. */
export interface PrivacyRequest {
  id: Uuid
  person_id: PersonId
  request_type: PrivacyRequestType
  status: PrivacyRequestStatus
  /** Échéance réglementaire : 30 jours après réception. */
  due_at: IsoDateTime
  handled_by: PersonId | null
  resolution: string | null
  /** Archive d'export : `media.assets.id`. */
  result_asset_id: AssetId | null
  created_at: IsoDateTime
  completed_at: IsoDateTime | null
}
