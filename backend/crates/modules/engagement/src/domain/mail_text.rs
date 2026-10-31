//! Les textes types (01/11) : ce que l'écran `/admin/courriels/textes` et la
//! fenêtre de rédaction lisent et écrivent.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

/// Un texte type (`MailText`).
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MailText {
    pub id: Uuid,
    pub name: String,
    pub category: Option<String>,
    pub subject: String,
    pub body_html: String,
    pub sort_order: i16,
    pub event_id: Option<Uuid>,
    pub created_by_name: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    /// Faux pour un texte commun à toutes les éditions, sans la permission
    /// globale : il se choisit, il ne se modifie pas.
    pub can_edit: bool,
}

/// L'écriture d'un texte type (`MailTextPayload`), complète à chaque fois.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct MailTextPayload {
    pub name: String,
    pub category: Option<String>,
    pub subject: String,
    pub body_html: String,
    #[serde(default)]
    pub sort_order: i16,
    /// Nul : commun à toutes les éditions.
    pub event_id: Option<Uuid>,
}
