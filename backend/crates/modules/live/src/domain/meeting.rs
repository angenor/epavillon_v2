//! Les formes des routes de réunion. Leur nom est celui du type TypeScript du
//! site (`frontend/app/types/`), seule définition qui fait foi.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// La salle ouvre trente minutes avant le début. Elle ferme quand Zoom annonce
/// la fin de la réunion ; sans cette annonce, quatre heures après la fin
/// prévue, car une activité de COP déborde souvent. Une réunion en cours reste
/// ouverte au-delà, jusqu'à ce que Zoom en annonce la fin.
pub const OUVERTURE_AVANT_MINUTES: i64 = 30;
pub const FERMETURE_APRES_MINUTES: i64 = 240;
/// Au-delà de la borne, une réunion en cours annonce une fermeture glissante.
pub const PROLONGATION_MINUTES: i64 = 15;

/// L'heure de fermeture de la salle, telle que la page la montre.
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

/// La salle est-elle ouverte à cet instant ?
pub fn salle_ouverte(
    starts_at: OffsetDateTime,
    ends_at: OffsetDateTime,
    statut: Option<&str>,
    maintenant: OffsetDateTime,
) -> bool {
    let ouverture = starts_at - time::Duration::minutes(OUVERTURE_AVANT_MINUTES);
    let borne = ends_at + time::Duration::minutes(FERMETURE_APRES_MINUTES);
    match statut {
        _ if maintenant < ouverture => false,
        Some("ended") => false,
        Some("started") => true,
        _ => maintenant <= borne,
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
    /// Voir `fermeture()` : la même règle que la route d'accès.
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
