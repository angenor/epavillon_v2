//! Les formes des routes de réunion. Leur nom est celui du type TypeScript du
//! site (`frontend/app/types/`), seule définition qui fait foi.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// Repères purement indicatifs de `PublicSessionMeeting` (décision du 17/10 :
/// la COP est trop imprévisible pour une fenêtre). La salle s'ouvre à toute
/// heure tant que la réunion existe chez Zoom ; la salle d'attente de Zoom
/// accueille qui arrive avant l'animateur.
pub const OUVERTURE_AVANT_MINUTES: i64 = 30;
pub const FERMETURE_APRES_MINUTES: i64 = 240;
pub const PROLONGATION_MINUTES: i64 = 15;

/// L'heure de fin indicative, telle que la page la montre.
pub fn fermeture(
    ends_at: OffsetDateTime,
    statut: Option<&str>,
    fin_constatee: Option<OffsetDateTime>,
    maintenant: OffsetDateTime,
) -> OffsetDateTime {
    let borne = ends_at + time::Duration::minutes(FERMETURE_APRES_MINUTES);
    match statut {
        Some("ended") => fin_constatee.unwrap_or(maintenant.min(borne)),
        Some("started") if maintenant > borne => {
            maintenant + time::Duration::minutes(PROLONGATION_MINUTES)
        }
        _ => borne,
    }
}

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
    /// Indicatif : voir `fermeture()`. L'accès ne dépend d'aucune heure.
    #[serde(with = "time::serde::rfc3339")]
    pub closes_at: OffsetDateTime,
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
