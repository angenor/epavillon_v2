//! Ce que les écrans lisent d'un modèle de message.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

/// Une ligne de la liste des modèles — `MessageTemplateRow`.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MessageTemplateRow {
    pub id: Uuid,
    pub key: String,
    pub label: serde_json::Value,
    pub type_code: Option<String>,
    /// Nul tant qu'aucune révision n'est publiée — et un type sans révision
    /// publiée part quand même, avec un texte de secours (FR-086).
    pub current_version: Option<i16>,
    pub is_active: bool,
    pub version_count: i64,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

/// Une révision.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TemplateVersion {
    pub id: Uuid,
    pub template_id: Uuid,
    pub version: i16,
    pub subject: serde_json::Value,
    /// **Assaini à l'écriture** — jamais à l'affichage.
    pub body_html: serde_json::Value,
    pub body_text: Option<serde_json::Value>,
    pub variables: Vec<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub published_at: Option<OffsetDateTime>,
    pub created_by: Option<Uuid>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

/// Le détail d'un modèle — `TemplateDetail`.
///
/// `promised_variables` vient du **type** servi, pas du modèle : c'est ce que
/// l'émetteur s'engage à fournir, et c'est contre cette liste qu'une
/// publication est refusée (FR-083). Sans elle, l'écran ne peut annoncer les
/// variables disponibles qu'en les devinant.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TemplateDetail {
    pub template: MessageTemplateRow,
    pub versions: Vec<TemplateVersion>,
    /// La révision réellement servie. Nulle tant qu'aucune n'est publiée.
    pub current: Option<TemplateVersion>,
    pub promised_variables: Vec<String>,
}

/// L'écriture d'une révision. Le numéro n'est pas reçu : il est **posé** par le
/// service, à la suite du dernier — deux administrateurs qui enregistrent en
/// même temps ne doivent pas se disputer un numéro.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct TemplateVersionPayload {
    pub subject: serde_json::Value,
    pub body_html: serde_json::Value,
    pub body_text: Option<serde_json::Value>,
}

/// Le rendu d'un aperçu — `RenderedMail`. **N'envoie rien.**
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RenderedMail {
    pub subject: String,
    pub body_html: String,
    pub body_text: String,
}

/// Une adresse écartée du circuit — `EmailSuppression`.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct EmailSuppression {
    pub email: String,
    pub reason: String,
    pub detail: Option<String>,
    /// Nulle = définitive. Une valeur permet de lever une suppression
    /// temporaire — une boîte pleine — sans intervention.
    #[serde(with = "time::serde::rfc3339::option")]
    pub expires_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub suppressed_at: OffsetDateTime,
    pub suppressed_by: Option<Uuid>,
}
