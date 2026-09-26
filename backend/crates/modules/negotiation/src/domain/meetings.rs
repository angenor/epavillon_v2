//! Les réunions de la Francophonie d'une édition, telles que l'application les
//! lit. **Jamais le lien de visioconférence ici** : la liste est publique, le
//! lien n'est servi qu'aux inscrites (research R6).

use serde::Serialize;
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::sessions::EditionServie;

/// `FrancophoneMeetings` — ce que `GET /negotiation/meetings?edition=` rend.
#[derive(Debug, Clone, Serialize)]
pub struct FrancophoneMeetings {
    pub edition: EditionServie,
    /// Hors de l'empreinte, comme `server_time` : l'heure de cette lecture.
    #[serde(with = "time::serde::rfc3339")]
    pub read_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub server_time: OffsetDateTime,
    pub meetings: Vec<FrancophoneMeeting>,
}

impl FrancophoneMeetings {
    pub fn empreinte(&self) -> String {
        let mut corps = serde_json::to_value(self).unwrap_or(Value::Null);
        if let Some(objet) = corps.as_object_mut() {
            objet.remove("read_at");
            objet.remove("server_time");
        }
        kernel::empreinte::de(&corps.to_string())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct FrancophoneMeeting {
    pub id: Uuid,
    #[serde(rename = "type")]
    pub meeting_type: NatureDeReunion,
    pub title: Value,
    pub description: Option<Value>,
    #[serde(with = "time::serde::rfc3339")]
    pub start_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub end_at: OffsetDateTime,
    /// `onsite`, `online` ou `hybrid`.
    pub format: String,
    pub venue: Option<String>,
    pub has_video: bool,
    /// « IFDD », ou le nom de l'organisation (R9 bis).
    pub organizer: String,
    pub open_access: bool,
    pub access_audience: Option<Value>,
    pub requires_registration: bool,
    pub capacity: Option<i32>,
    pub registered_count: i32,
    pub waitlist_enabled: bool,
    #[serde(with = "time::serde::rfc3339::option")]
    pub registration_opens_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub registration_closes_at: Option<OffsetDateTime>,
    pub status: StatutDeReunion,
    pub cancellation_reason: Option<String>,
    pub pavilion_session_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NatureDeReunion {
    pub code: String,
    pub label: Value,
}

/// « Terminée » se déduit de `end_at` ; `ongoing` et `completed` se lisent
/// `scheduled`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StatutDeReunion {
    Scheduled,
    Cancelled,
}
