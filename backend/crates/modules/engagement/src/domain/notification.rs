//! Ce que les écrans lisent d'une notification et d'une préférence.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

/// Une notification, telle que la liste la rend.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct Notification {
    pub id: Uuid,
    pub type_code: String,
    pub title: Option<serde_json::Value>,
    pub body: Option<serde_json::Value>,
    pub variables: serde_json::Value,
    /// Chemin relatif Nuxt, jamais une adresse absolue : les domaines de
    /// préproduction ne doivent pas fuiter dans les données.
    pub link_path: Option<String>,
    pub subject_schema: Option<String>,
    pub subject_table: Option<String>,
    pub subject_id: Option<Uuid>,
    /// « 3 nouveaux commentaires » plutôt que trois lignes. Vaut 1 pour une
    /// notification seule.
    pub group_count: i32,
    #[serde(with = "time::serde::rfc3339::option")]
    pub read_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

/// La liste **et** le compte, dans la même réponse — `NotificationFeed`.
///
/// Deux appels donneraient deux chiffres mesurés à deux instants : c'est le
/// défaut que B4 a nommé sur les facettes d'une liste, et il se reproduirait ici
/// avec un badge qui contredit la liste qu'il coiffe.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct NotificationFeed {
    pub items: Vec<Notification>,
    pub unread_count: i64,
}

/// Une ligne de l'écran des préférences — `NotificationPreferenceRow`.
///
/// **`is_overridable` est le champ qui compte.** Une préférence posée sur un
/// type critique est enregistrée telle quelle — l'API ne refuse pas —, mais
/// `is_channel_enabled()` l'ignore. Sans ce champ, l'écran afficherait un
/// interrupteur éteint pour un avis qui part quand même (FR-095).
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct NotificationPreferenceRow {
    pub type_code: String,
    pub label: serde_json::Value,
    pub description: Option<serde_json::Value>,
    pub module_code: String,
    pub criticality: String,
    pub channel: String,
    pub is_enabled: bool,
    /// Faux pour un type critique : la préférence est enregistrée, elle
    /// n'oppose rien.
    pub is_overridable: bool,
}

/// L'écriture d'une préférence.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct NotificationPreferencePayload {
    pub type_code: String,
    pub channel: String,
    pub is_enabled: bool,
}
