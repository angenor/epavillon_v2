//! Les cinq énumérations du schéma, et ce que les écrans lisent d'un objet.
//!
//! Les énumérations traversent la frontière SQL **en texte**, patron des cinq
//! modules livrés : la macro de SQLx ne sait pas typer un paramètre de type
//! personnalisé, et un `::text::media.asset_status` explicite se relit mieux
//! qu'une dérivation qui échoue à la compilation pour une raison illisible.
//!
//! **`ready` est le seul état servi**, et il exige deux choses que la base
//! vérifie et que ce module ne revérifie jamais : un verdict d'analyse
//! acceptable (`ck_assets_scan_before_ready`) et un texte alternatif si c'est
//! une image (`ck_assets_alt_text_required`).

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

macro_rules! enumeration_texte {
    (
        $(#[$meta:meta])*
        $nom:ident { $( $variante:ident => $texte:literal ),* $(,)? }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
        #[serde(rename_all = "snake_case")]
        pub enum $nom { $( $variante, )* }

        impl $nom {
            pub fn as_str(self) -> &'static str {
                match self { $( Self::$variante => $texte, )* }
            }

            /// L'énuméré est fermé en base : une valeur inconnue signale que le
            /// code et le modèle ont divergé.
            pub fn from_db(valeur: &str) -> Option<Self> {
                Some(match valeur {
                    $( $texte => Self::$variante, )*
                    _ => return None,
                })
            }
        }
    };
}

enumeration_texte! {
    /// `media.asset_visibility`. `private` promet une adresse signée à durée
    /// limitée : rien de ce jalon n'en émet, et aucun objet du périmètre n'en
    /// porte.
    AssetVisibility {
        Public => "public",
        Authenticated => "authenticated",
        Private => "private",
    }
}

enumeration_texte! {
    /// `media.asset_status`. Le service ne pose que `uploaded` ; tout le reste
    /// appartient au travail différé. `quarantined` et `failed` sont terminaux.
    AssetStatus {
        Uploaded => "uploaded",
        Scanning => "scanning",
        Processing => "processing",
        Ready => "ready",
        Quarantined => "quarantined",
        Failed => "failed",
    }
}

enumeration_texte! {
    /// `media.scan_verdict`. **`Unsupported` n'est pas une absence de verdict** :
    /// c'est « aucun moteur ne sait analyser ceci », littéralement vrai quand
    /// aucun moteur n'est branché. Écrire `Clean` sans avoir regardé rendrait
    /// fausse la preuve d'inspection (B6, R13).
    ScanVerdict {
        Pending => "pending",
        Clean => "clean",
        Infected => "infected",
        Unsupported => "unsupported",
        Error => "error",
    }
}

enumeration_texte! {
    /// `media.rendition_format`. `Webp` et `Avif` sont déclarés par le modèle et
    /// **ne sont pas produits** par ce jalon : l'encodeur WebP disponible est
    /// sans perte, ce qui alourdirait une photographie au lieu de l'alléger, et
    /// l'AVIF exige un encodeur hors de proportion avec le besoin (B6, R12).
    /// Les ajouter reste une insertion, jamais une migration.
    RenditionFormat {
        Webp => "webp",
        Avif => "avif",
        Jpeg => "jpeg",
        Png => "png",
        Mp4 => "mp4",
        Pdf => "pdf",
    }
}

enumeration_texte! {
    /// `media.rendition_status`.
    RenditionStatus {
        Pending => "pending",
        Generating => "generating",
        Ready => "ready",
        Failed => "failed",
    }
}

enumeration_texte! {
    /// `media.attachment_role`. **Un rôle dit un USAGE, jamais une forme** : la
    /// vignette est « ce qui représente l'entité là où la place est comptée ».
    /// Qu'elle appelle un carré est une conséquence, déclarée en base.
    AttachmentRole {
        Cover => "cover",
        Banner => "banner",
        Logo => "logo",
        Gallery => "gallery",
        Document => "document",
        Avatar => "avatar",
        Video => "video",
        Thumbnail => "thumbnail",
        Attachment => "attachment",
    }
}

/// Un objet stocké, tel que `frontend/app/types/media.ts` le nomme. Les noms de
/// champs sont **exactement** les siens : le contrat du front a une seule source.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct Asset {
    pub id: Uuid,
    pub bucket: String,
    pub object_key: String,
    pub checksum_sha256: String,
    pub mime_type: String,
    pub byte_size: i64,
    pub original_filename: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    /// `numeric(10,3)` en base, traversée en texte : un flottant perdrait les
    /// millisecondes d'une vidéo sans le dire.
    pub duration_seconds: Option<String>,
    pub owner_person_id: Option<Uuid>,
    pub owner_organization_id: Option<Uuid>,
    pub visibility: String,
    pub status: String,
    pub scan_verdict: String,
    pub scan_engine: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub scanned_at: Option<OffsetDateTime>,
    pub scan_details: Option<serde_json::Value>,
    pub alt_text: Option<serde_json::Value>,
    pub caption: Option<serde_json::Value>,
    pub credit: Option<String>,
    pub license_code: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub deleted_at: Option<OffsetDateTime>,
    pub deleted_by: Option<Uuid>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub purge_after: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub purged_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    /// L'adresse publique de l'ORIGINAL, composée en base par
    /// `media.object_url()`. Elle n'est pas une colonne : la base ne stocke
    /// jamais d'URL.
    pub url: String,
    /// Les déclinaisons prêtes, indexées `<variante>_<format>`. **Objet vide et
    /// non nul** tant que le worker n'a rien produit : un écran qui n'afficherait
    /// que celles-ci laisserait un trou entre le dépôt et le traitement.
    pub sources: serde_json::Value,
}

/// L'avancement du traitement — `AssetProgress`.
///
/// Sans elle, un écran ne sait pas distinguer « en cours » de « en échec » : les
/// deux se lisent « pas encore là » (FR-032). Un objet en échec ou en
/// quarantaine rend son état ici, en **200** ; il est simplement absent des
/// lectures publiques.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AssetProgress {
    pub asset_id: Uuid,
    pub status: String,
    pub scan_verdict: String,
    pub scan_engine: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    /// Déclinaisons réellement prêtes.
    pub renditions_ready: i64,
    /// Déclinaisons attendues d'après la configuration du worker. Zéro pour un
    /// document : rien n'est décliné.
    pub renditions_expected: i64,
    pub last_error: Option<String>,
}

/// Le verdict d'une annonce préalable — `UploadVerdict`.
///
/// **N'écrit rien, ne réserve rien.** Elle rend ce que rendrait le dépôt : un
/// refus y est une RÉPONSE, jamais une erreur, et sort donc en 200.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct UploadVerdict {
    pub accepted: bool,
    /// Le code stable qui sortirait, quand le dépôt serait refusé.
    pub code: Option<String>,
    /// Le champ que l'écran doit souligner.
    pub field: Option<String>,
    pub message: Option<String>,
    /// L'objet déjà connu pour cette empreinte, si le client en a fourni une :
    /// c'est le succès de la déduplication, pas un refus.
    pub existing_asset: Option<Box<Asset>>,
    /// Plafond, consommation et reste, quand le refus vient du quota — les
    /// trois chiffres que l'écran affiche.
    pub quota: Option<QuotaSnapshot>,
}

#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
pub struct QuotaSnapshot {
    pub max_bytes: i64,
    pub used_bytes: i64,
    pub remaining_bytes: i64,
    pub max_files: i32,
    pub used_files: i32,
}

/// Une ligne du tableau des quotas du back-office — `QuotaRow`.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct QuotaRow {
    pub organization_id: Uuid,
    pub organization_name: String,
    pub max_bytes: i64,
    pub used_bytes: i64,
    pub max_files: i32,
    pub used_files: i32,
    /// Part consommée, de 0 à 1 — c'est par elle que le tableau se trie.
    pub used_ratio: f64,
    pub note: Option<String>,
}

/// Un objet prêt que plus rien n'utilise — `OrphanAsset`.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct OrphanAsset {
    pub asset_id: Uuid,
    pub bucket: String,
    pub object_key: String,
    pub byte_size: i64,
    pub rendition_bytes: i64,
    pub owner_organization_id: Option<Uuid>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    pub age_days: i32,
}
