//! Les gabarits de courriel personnalisés (29/10) : ce que l'écran
//! `/admin/courriels/gabarits` lit et écrit.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

/// Un gabarit (`MailTemplate`). `background` et `logo` : ses images, telles
/// que `media.attached_image()` les rend (`AttachedImage`), nulles sans dépôt.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MailTemplate {
    pub id: Uuid,
    pub name: String,
    pub layout: String,
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub gradient: bool,
    pub event_id: Option<Uuid>,
    pub preselect_for_submitters: bool,
    pub created_by_name: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    #[schema(value_type = Object)]
    pub background: Option<serde_json::Value>,
    #[schema(value_type = Object)]
    pub logo: Option<serde_json::Value>,
    /// Faux pour un gabarit commun à toutes les éditions, sans la permission
    /// globale : il se choisit, il ne se modifie pas.
    pub can_edit: bool,
}

/// L'écriture d'un gabarit (`MailTemplatePayload`), complète à chaque fois.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct MailTemplatePayload {
    pub name: String,
    pub layout: String,
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub gradient: bool,
    /// Nul : commun à toutes les éditions.
    pub event_id: Option<Uuid>,
    pub preselect_for_submitters: bool,
}

/// L'aperçu d'un gabarit (`MailTemplatePreview`), rendu par la fonction du
/// worker sur un texte d'exemple.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MailTemplatePreview {
    pub subject: String,
    pub html: String,
}
