/**
 * Schéma `media` — objets stockés (Garage/S3), variantes, rattachements.
 * Dérivé de `docs/database/050_media.sql`.
 *
 * Ce fichier ne figurait pas dans l'arborescence du prompt A0.2 ; il est requis
 * par le périmètre du jalon : le formulaire de soumission téléverse des
 * documents (`programme.proposal_documents.asset_id`) et les fiches
 * d'organisation portent un logo. Écart consigné dans `docs/progression/decisions/2026-08-16.md`.
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
  OrganizationId,
  PersonId,
  TaxonomyTermCode,
  Url,
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

/**
 * ENUM `media.attachment_role`.
 *
 * L'ordre suit celui du SQL. `video` y a été ajouté le 19/08 pour le module
 * `content` : l'énumération ne portait que des rôles d'image et de document,
 * alors que la plateforme stocke déjà des enregistrements de séance et fait
 * défiler des fonds vidéo sur sa page d'accueil.
 *
 * `thumbnail` y a été ajouté le 19/08 pour les trois déclinaisons d'une édition.
 * UN RÔLE DIT UN USAGE, JAMAIS UNE FORME : c'est la vignette qui représente
 * l'entité là où la place est comptée. Que cet usage appelle un carré est une
 * conséquence, déclarée en base (`attachable_roles.expected_aspect_ratio`) et
 * non dans ce nom — `square` aurait figé la forme dans le vocabulaire.
 */
export type AttachmentRole =
  | 'cover'
  | 'banner'
  | 'logo'
  | 'gallery'
  | 'document'
  | 'avatar'
  | 'video'
  | 'thumbnail'
  | 'attachment'

// ---------------------------------------------------------------------------
// Les trois déclinaisons d'une édition
// ---------------------------------------------------------------------------

/**
 * LES TROIS FORMES D'UNE ÉDITION, ET LEUR RÔLE EN BASE.
 *
 * Trois recadrages TÉLÉVERSÉS À LA MAIN, jamais déduits l'un de l'autre : un
 * bandeau 32:9 rogné depuis une photographie de conférence décapite les
 * intervenants, et un carré tiré du même fichier ne garde qu'une épaule. La
 * forme attendue est déclarée dans `media.attachable_roles` et vérifiée par
 * trigger — l'écran l'annonce, il ne la fait pas respecter.
 */
export const EDITION_IMAGE_ROLES = ['banner', 'cover', 'thumbnail'] as const

export type EditionImageRole = (typeof EDITION_IMAGE_ROLES)[number]

/** Le rapport largeur ÷ hauteur exigé par la base, pour l'annoncer à l'éditeur. */
export const EDITION_IMAGE_RATIO: Record<EditionImageRole, string> = {
  banner: '32 / 9',
  cover: '16 / 9',
  thumbnail: '1 / 1',
}

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
  /** `numeric(10,3)` traversée en TEXTE (`duration_seconds::text`) : un flottant
   *  perdrait les millisecondes d'une vidéo sans le dire. */
  duration_seconds: string | null
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
  /** Adresse publique de l'ORIGINAL. Ce n'est pas une colonne : la base ne
   *  stocke jamais d'URL, `media.object_url()` la compose à la lecture. */
  url: Url
  /** Déclinaisons prêtes. Objet VIDE ET NON NUL tant que le worker n'a rien
   *  produit — l'écran affiche alors `url`, pas un trou. */
  sources: AssetSources
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

/**
 * Retour de `media.attached_image(schéma, table, id, rôle)` — `050_media.sql` § 4.
 * Une image rattachée, prête à l'affichage : c'est ce que les VUES exposent, et
 * ce qu'un composant d'image reçoit. Jamais un identifiant d'objet à résoudre.
 *
 * `url` porte TOUJOURS l'original. La génération des variantes est asynchrone :
 * entre le téléversement et le passage du worker, `sources` est vide alors que
 * l'image est parfaitement valide. Un composant qui n'afficherait que `sources`
 * laisserait donc un trou pendant ce délai.
 *
 * `alt_text` N'EST PAS NULLABLE, contrairement à `Asset.alt_text`. La différence
 * est voulue et vient du modèle : `ck_assets_alt_text_required` interdit à une
 * image d'atteindre l'état `ready` sans texte alternatif, et seul `ready` est
 * servi. Toute image que l'API peut rendre en a donc forcément un — pas de repli
 * sur une chaîne vide, pas de repli sur le nom de fichier. C'est une DONNÉE
 * multilingue, résolue par `resolveI18nText()`, jamais une clé de traduction.
 */
export interface AttachedImage {
  asset_id: AssetId
  /** URL absolue de l'original, composée en base par `media.object_url()`. */
  url: Url
  width: number | null
  height: number | null
  /** Surcharge du rattachement d'abord, texte de l'objet ensuite — résolu en base. */
  alt_text: I18nText
  caption: I18nText | null
  credit: string | null
  /** Variantes prêtes. Objet vide tant que le worker n'a rien produit. */
  sources: AssetSources
}

/**
 * Retour de `GET /media/attachments` — `AttachedMedia`.
 *
 * C'est `AttachedImage` PLUS ce que l'écran qui GÈRE les médias d'une entité
 * exige, et que l'affichage n'a jamais eu à connaître : l'identifiant du
 * rattachement, sans lequel on ne sait pas quoi détacher ; le rôle, sans lequel
 * on ne sait pas où ranger la ligne ; l'ordre de tri, sans lequel une galerie ne
 * se réordonne pas ; et l'état de l'objet, pour dire « en traitement » au lieu
 * de laisser un trou entre le dépôt et le passage du worker.
 *
 * LES DEUX FORMES COEXISTENT : `AttachedImage` décrit ce que rend
 * `media.attached_image()`, qui ne sert que des objets servables ; celle-ci sert
 * aussi les documents et les objets encore en traitement.
 */
export interface AttachedMedia {
  attachment_id: Uuid
  role: AttachmentRole
  sort_order: number
  asset_id: AssetId
  /** L'ORIGINAL, déjà là au dépôt quand `sources` est encore vide. */
  url: Url
  width: number | null
  height: number | null
  /** NULLABLE ici, contrairement à `AttachedImage.alt_text` : un document n'a
   *  pas de texte alternatif, et un objet en traitement n'a pas encore atteint
   *  l'état où `ck_assets_alt_text_required` l'exige. Résolu en base,
   *  surcharge du rattachement d'abord, texte de l'objet ensuite. */
  alt_text: I18nText | null
  caption: I18nText | null
  credit: string | null
  sources: AssetSources
  /** `ready` est le seul état servi au public ; les autres se disent ici. */
  status: AssetStatus
}

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
  /**
   * Largeur ÷ hauteur exigée — `3.5556` pour un 32:9, `1.0000` pour un carré.
   *
   * EN TEXTE, et non en nombre : `numeric(6,4)` n'a pas de représentant flottant
   * exact, et le rapport sert à AFFICHER autant qu'à comparer. C'est ce que
   * l'éditeur de recadrage impose à la poignée qu'on tire — la forme n'est donc
   * plus apprise par le refus, après que le fichier a traversé le réseau.
   */
  expected_aspect_ratio: string | null
  /** Écart relatif toléré par le trigger. `0.02` = 2 %. */
  aspect_ratio_tolerance: string
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

/**
 * Une ligne du tableau des quotas du back-office — `QuotaRow`.
 *
 * CE N'EST PAS `StorageQuota`, qui est la TABLE : celle-là porte `id`,
 * `updated_at` et un `organization_id` nul pour la ligne de défaut ; celle-ci
 * est une lecture jointe à la dénomination et triée par proximité du plafond.
 * Une organisation qui n'a rien déposé n'y figure pas — l'absence de ligne dit
 * « rien déposé », jamais « aucun quota ».
 */
export interface QuotaRow {
  organization_id: OrganizationId
  organization_name: string
  max_bytes: Int8
  used_bytes: Int8
  max_files: number
  used_files: number
  /** Part consommée — c'est par elle que le tableau se trie. Elle DÉPASSE 1
   *  quand le plafond a été abaissé sous la consommation déjà écrite : une
   *  jauge doit l'écrêter plutôt que déborder. */
  used_ratio: number
  note: string | null
}

/** Plafond, consommation et reste — ce qu'un refus de quota affiche, `QuotaSnapshot`. */
export interface QuotaSnapshot {
  max_bytes: Int8
  used_bytes: Int8
  remaining_bytes: Int8
  max_files: number
  used_files: number
}

// ---------------------------------------------------------------------------
// Dépôt et traitement
// ---------------------------------------------------------------------------

/**
 * Le verdict d'une annonce préalable — `UploadVerdict`.
 *
 * ELLE N'ÉCRIT RIEN ET NE RÉSERVE RIEN : ni espace, ni clé, ni identifiant. Elle
 * rend ce que le dépôt ferait de ce fichier, et TOUS SES REFUS SORTENT EN 200 —
 * une annonce est une question, pas une tentative.
 */
export interface UploadVerdict {
  accepted: boolean
  /** Le code stable que le dépôt rendrait, s'il refusait. */
  code: string | null
  /** Le champ que l'écran doit souligner. */
  field: string | null
  message: string | null
  /** L'objet déjà connu pour cette empreinte : le succès de la déduplication,
   *  pas un refus. */
  existing_asset: Asset | null
  /** Renseigné quand le refus vient du quota. */
  quota: QuotaSnapshot | null
}

/**
 * L'avancement du traitement — `AssetProgress`.
 *
 * Sans elle, « en cours » et « en échec » se lisent tous les deux « pas encore
 * là ». Un objet en échec ou en quarantaine sort ici en 200 avec son état ; il
 * est simplement absent des lectures publiques.
 */
export interface AssetProgress {
  asset_id: AssetId
  status: AssetStatus
  scan_verdict: ScanVerdict
  scan_engine: string | null
  width: number | null
  height: number | null
  renditions_ready: number
  /** Zéro pour un document : rien n'est décliné. */
  renditions_expected: number
  last_error: string | null
}

/**
 * Un objet prêt que plus rien n'utilise — `OrphanAsset`.
 * Retour de `media.find_orphan_assets()`, du plus lourd au plus léger.
 */
export interface OrphanAsset {
  asset_id: AssetId
  bucket: string
  object_key: string
  byte_size: Int8
  /** Octets des déclinaisons prêtes, EN PLUS de `byte_size` — il ne les contient pas. */
  rendition_bytes: Int8
  owner_organization_id: OrganizationId | null
  created_at: IsoDateTime
  /** Jours entiers écoulés depuis le dépôt. */
  age_days: number
}

// ---------------------------------------------------------------------------
// Le dépôt, vu de l'écran
// ---------------------------------------------------------------------------

/**
 * Ce qu'un écran envoie à `POST /media/assets` — `UploadPayload`.
 *
 * LE FICHIER EST UN `Blob`, jamais un `File` : ce qui part n'est pas ce qui a
 * été choisi sur le disque, mais ce que l'éditeur de recadrage a produit. Le nom
 * et le type voyagent donc à côté, puisque le `Blob` n'en porte pas.
 *
 * L'ENTITÉ PORTEUSE EST FACULTATIVE, et son absence n'est pas un oubli : à la
 * création d'une édition, l'entité n'existe pas encore. Renseignée, elle vaut
 * refus AVANT lecture — type, poids et droit sont vérifiés sans qu'un octet
 * traverse le réseau.
 *
 * LE TEXTE ALTERNATIF EST OBLIGATOIRE POUR UNE IMAGE. Ce n'est pas une politesse
 * d'écran : `ck_assets_alt_text_required` interdit à une image d'atteindre
 * l'état servable sans lui, et le dépôt le refuse avant de lire le flux.
 */
export interface UploadPayload {
  file: Blob
  filename: string
  mimeType: string
  altText: I18nText
  ownerSchema?: string
  ownerTable?: string
  ownerId?: Uuid
  role?: AttachmentRole
}

/**
 * Ce que rend le dépôt — `UploadedAsset`.
 *
 * C'est `Asset` PLUS `deduplicated`, que la route ajoute au corps. Le drapeau
 * est un SUCCÈS, jamais un refus : le contenu était déjà connu du stockage,
 * aucun second objet n'a été écrit, et l'objet rendu est celui d'avant. Un écran
 * qui l'ignorerait afficherait quand même la bonne image ; il ne saurait
 * simplement pas qu'il n'a rien coûté.
 */
export interface UploadedAsset extends Asset {
  deduplicated: boolean
}
