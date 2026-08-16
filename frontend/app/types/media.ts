/**
 * Schéma `media` — objets stockés (Garage/S3), variantes, rattachements.
 * Dérivé de `docs/database/050_media.sql`.
 *
 * Ce fichier ne figurait pas dans l'arborescence du prompt A0.2 ; il est requis
 * par le périmètre du jalon : le formulaire de soumission téléverse des
 * documents (`programme.proposal_documents.asset_id`) et les fiches
 * d'organisation portent un logo. Écart consigné dans `docs/PROGRESSION.md`.
 *
 * RÈGLE DU MODÈLE : la base ne stocke JAMAIS d'URL. Un objet est décrit par
 * `(bucket, object_key)` et l'URL publique est composée à la lecture par l'API
 * (`media.object_url()`, réglage `media.public_base_url`). Un composant reçoit
 * donc une URL déjà construite ; il n'en fabrique aucune.
 */

import type {
  AssetId,
  I18nText,
  Int8,
  IsoDateTime,
  Numeric,
  OrganizationId,
  PersonId,
  TaxonomyTermCode,
  Uuid,
} from './shared'

// ---------------------------------------------------------------------------
// Types du module
// ---------------------------------------------------------------------------

/** ENUM `media.asset_visibility`. */
export type AssetVisibility = 'public' | 'authenticated' | 'private'

/** ENUM `media.asset_status`. Seul `ready` est servi au public. */
export type AssetStatus =
  | 'uploaded'
  | 'scanning'
  | 'processing'
  | 'ready'
  | 'quarantined'
  | 'failed'

/** ENUM `media.scan_verdict`. */
export type ScanVerdict = 'pending' | 'clean' | 'infected' | 'unsupported' | 'error'

/** ENUM `media.rendition_format`. */
export type RenditionFormat = 'webp' | 'avif' | 'jpeg' | 'png' | 'mp4' | 'pdf'

/** ENUM `media.rendition_status`. */
export type RenditionStatus = 'pending' | 'generating' | 'ready' | 'failed'

/** ENUM `media.attachment_role`. */
export type AttachmentRole =
  | 'cover'
  | 'banner'
  | 'logo'
  | 'gallery'
  | 'document'
  | 'avatar'
  | 'attachment'

// ---------------------------------------------------------------------------
// Objets stockés
// ---------------------------------------------------------------------------

/** Table `media.assets` — `050_media.sql` § 2. */
export interface Asset {
  id: AssetId
  bucket: string
  /** Chemin dans le bucket : `<année>/<mois>/<uuid>/<nom-normalisé>.<ext>`.
   *  Ni schéma, ni nom d'hôte — changer de fournisseur ne touche pas les données. */
  object_key: string
  /** Empreinte du contenu : socle de la déduplication et du contrôle d'intégrité. */
  checksum_sha256: string
  mime_type: string
  byte_size: Int8
  original_filename: string | null
  width: number | null
  height: number | null
  duration_seconds: Numeric | null
  owner_person_id: PersonId | null
  owner_organization_id: OrganizationId | null
  visibility: AssetVisibility
  status: AssetStatus
  scan_verdict: ScanVerdict
  scan_engine: string | null
  scanned_at: IsoDateTime | null
  scan_details: Record<string, unknown> | null
  /** Obligatoire pour toute image servie — exigence d'accessibilité portée par
   *  la contrainte `ck_assets_alt_text_required`, pas par le formulaire. */
  alt_text: I18nText | null
  caption: I18nText | null
  credit: string | null
  /** Code de la taxonomie `media_license`. */
  license_code: TaxonomyTermCode | null
  deleted_at: IsoDateTime | null
  deleted_by: PersonId | null
  purge_after: IsoDateTime | null
  purged_at: IsoDateTime | null
  created_at: IsoDateTime
  updated_at: IsoDateTime
}

/**
 * Table `media.renditions` — `050_media.sql` § 3.
 * Une ligne par déclinaison : c'est ce qui remplace les six colonnes
 * `banner_*_url` de la v1. Ajouter l'AVIF est une insertion, pas une migration.
 */
export interface Rendition {
  id: Uuid
  asset_id: AssetId
  /** `original`, `hd_16_9`, `card_1_1`, `thumb`… */
  variant_code: string
  format: RenditionFormat
  width: number | null
  height: number | null
  object_key: string
  byte_size: Int8 | null
  status: RenditionStatus
  generated_at: IsoDateTime | null
  last_error: string | null
  created_at: IsoDateTime
}

/**
 * Retour de `media.asset_sources(asset_id)` — `050_media.sql` § 3.
 * Clé : `<variant_code>_<format>`. Directement exploitable par un `<picture>`.
 */
export type AssetSources = Record<
  string,
  { url: string; width: number | null; height: number | null; bytes: Int8 | null }
>

// ---------------------------------------------------------------------------
// Rattachements
// ---------------------------------------------------------------------------

/**
 * Table `media.attachable_roles` — `050_media.sql` § 4.
 * Table blanche : toute combinaison entité × rôle non déclarée est refusée.
 */
export interface AttachableRoleRule {
  owner_schema: string
  owner_table: string
  role: AttachmentRole
  label: I18nText
  /** `false` : un seul objet pour ce rôle (logo, avatar, bannière). */
  is_multiple: boolean
  /** Préfixes MIME acceptés, `*` autorisé. Tableau vide = tout accepté. */
  allowed_mime_prefixes: string[]
  max_byte_size: Int8 | null
  is_active: boolean
}

/** Table `media.attachments` — `050_media.sql` § 4. */
export interface Attachment {
  id: Uuid
  owner_schema: string
  owner_table: string
  owner_id: Uuid
  asset_id: AssetId
  role: AttachmentRole
  sort_order: number
  /** Un objet dédupliqué est partagé : le texte alternatif pertinent peut
   *  différer d'un usage à l'autre. */
  alt_text_override: I18nText | null
  /** Renseigné par trigger depuis la règle ; sert l'index unique partiel. */
  readonly is_exclusive: boolean
  created_by: PersonId | null
  created_at: IsoDateTime
}

// ---------------------------------------------------------------------------
// Quotas
// ---------------------------------------------------------------------------

/**
 * Table `media.storage_quotas` — `050_media.sql` § 5.
 * La ligne à `organization_id` nul est le quota par défaut (5 Gio, 5 000 fichiers).
 */
export interface StorageQuota {
  id: Uuid
  organization_id: OrganizationId | null
  max_bytes: Int8
  max_files: number
  /** Consommation courante (objets + variantes), maintenue par trigger. */
  used_bytes: Int8
  used_files: number
  note: string | null
  updated_at: IsoDateTime
}
