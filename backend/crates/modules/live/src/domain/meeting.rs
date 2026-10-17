//! Les formes des routes de réunion. Leur nom est celui du type TypeScript du
//! site (`frontend/app/types/`), seule définition qui fait foi.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// La salle ouvre trente minutes avant le début et ferme trente minutes après
/// la fin : on entre en avance, on déborde parfois.
pub const OUVERTURE_AVANT_MINUTES: i64 = 30;
pub const FERMETURE_APRES_MINUTES: i64 = 30;

#[derive(Debug, Serialize)]
pub struct PublicSessionMeeting {
    pub provider: String,
    pub status: String,
    #[serde(with = "time::serde::rfc3339")]
    pub opens_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub starts_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub ends_at: OffsetDateTime,
}

/// Ce que le Meeting SDK attend pour faire entrer une personne inscrite.
#[derive(Serialize)]
pub struct MeetingAccess {
    pub meeting_number: String,
    pub password: String,
    pub registrant_token: Option<String>,
    pub user_name: String,
    pub user_email: String,
    pub sdk_key: String,
    pub signature: String,
    #[serde(with = "time::serde::rfc3339")]
    pub expires_at: OffsetDateTime,
    pub join_url: String,
}

impl std::fmt::Debug for MeetingAccess {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MeetingAccess")
            .field("meeting_number", &self.meeting_number)
            .field("user_email", &self.user_email)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Default, Deserialize, utoipa::ToSchema)]
pub struct MeetingSyncInput {
    #[serde(default)]
    pub external_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AdminSessionMeeting {
    pub id: Uuid,
    pub session_id: Uuid,
    pub provider: String,
    pub external_id: Option<String>,
    pub status: String,
    pub sync_status: String,
    pub last_sync_error: Option<String>,
    pub join_url: Option<String>,
    pub participants_total: i64,
    pub participants_synced: i64,
    pub participants_failed: i64,
}
