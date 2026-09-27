//! Les réunions de la Francophonie au back-office. Le serveur pose l'espace, le
//! slug, l'édition, le fuseau et le `kind` (research R9 bis) : le corps n'en
//! porte aucun.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::meetings::NatureDeReunion;
use crate::domain::sessions::EditionServie;

/// `AdminFrancophoneMeetings` — `GET /admin/negotiation/meetings?edition=`.
#[derive(Debug, Clone, Serialize)]
pub struct AdminFrancophoneMeetings {
    pub edition: EditionServie,
    pub meetings: Vec<AdminFrancophoneMeeting>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminFrancophoneMeeting {
    pub id: Uuid,
    pub slug: String,
    pub edition: String,
    pub timezone: String,
    #[serde(rename = "type")]
    pub meeting_type: Option<NatureDeReunion>,
    pub title: Value,
    pub description: Option<Value>,
    #[serde(with = "time::serde::rfc3339")]
    pub start_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub end_at: OffsetDateTime,
    pub format: String,
    pub venue: Option<String>,
    pub external_url: Option<String>,
    pub capacity: Option<i32>,
    pub waitlist_enabled: bool,
    pub requires_registration: bool,
    #[serde(with = "time::serde::rfc3339::option")]
    pub registration_opens_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub registration_closes_at: Option<OffsetDateTime>,
    pub open_access: bool,
    pub access_audience: Option<Value>,
    pub is_ifdd_organized: bool,
    pub organizer_org_id: Option<Uuid>,
    pub organizer: String,
    pub status: String,
    pub cancellation_reason: Option<String>,
    pub pavilion_session_id: Option<Uuid>,
    pub registered_count: i32,
    pub waitlisted_count: i64,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FormatDeReunion {
    Onsite,
    Online,
    Hybrid,
}

impl FormatDeReunion {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Onsite => "onsite",
            Self::Online => "online",
            Self::Hybrid => "hybrid",
        }
    }
}

/// `FrancophoneMeetingInput` — corps de `POST` et de `PUT` ; `edition` n'est
/// lue qu'à la création.
#[derive(Debug, Clone, Deserialize)]
pub struct FrancophoneMeetingInput {
    #[serde(default)]
    pub edition: Option<String>,
    /// Code du vocabulaire `francophone_meeting_type` ; requis à la publication.
    #[serde(rename = "type", default)]
    pub type_code: Option<String>,
    pub title: Value,
    #[serde(default)]
    pub description: Option<Value>,
    #[serde(with = "time::serde::rfc3339")]
    pub start_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub end_at: OffsetDateTime,
    pub format: FormatDeReunion,
    #[serde(default)]
    pub venue: Option<String>,
    #[serde(default)]
    pub external_url: Option<String>,
    #[serde(default)]
    pub capacity: Option<i32>,
    pub waitlist_enabled: bool,
    pub requires_registration: bool,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub registration_opens_at: Option<OffsetDateTime>,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub registration_closes_at: Option<OffsetDateTime>,
    pub open_access: bool,
    #[serde(default)]
    pub access_audience: Option<Value>,
    pub is_ifdd_organized: bool,
    #[serde(default)]
    pub organizer_org_id: Option<Uuid>,
}

/// `CancelMeetingPayload`.
#[derive(Debug, Clone, Deserialize)]
pub struct CancelMeetingPayload {
    pub reason: String,
}

/// `MeetingPavilionPayload` — `null` retire le lien.
#[derive(Debug, Clone, Deserialize)]
pub struct MeetingPavilionPayload {
    pub pavilion_session_id: Option<Uuid>,
}

/// `AdminMeetingRegistrations`.
#[derive(Debug, Clone, Serialize)]
pub struct AdminMeetingRegistrations {
    pub registered: Vec<AdminMeetingRegistrant>,
    pub waitlisted: Vec<AdminMeetingRegistrant>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminMeetingRegistrant {
    pub person_id: Uuid,
    pub name: String,
    pub country: Option<Value>,
    #[serde(with = "time::serde::rfc3339")]
    pub registered_at: OffsetDateTime,
    pub waitlist_position: Option<i32>,
}

/// `PavilionActivityOption` — lue dans `programme.sessions`.
#[derive(Debug, Clone, Serialize)]
pub struct PavilionActivityOption {
    pub id: Uuid,
    pub title: Value,
    #[serde(with = "time::serde::rfc3339")]
    pub starts_at: OffsetDateTime,
}
