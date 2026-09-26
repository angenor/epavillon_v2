//! S'inscrire à une réunion de la Francophonie. La jauge, la fenêtre et la liste
//! d'attente sont tenues par la base (research R4) ; le code traduit ses refus.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// `MyMeetingRegistrations` — `GET /negotiation/me/meeting-registrations`.
#[derive(Debug, Clone, Serialize)]
pub struct MyMeetingRegistrations {
    pub registrations: Vec<MeetingRegistration>,
    pub video: Vec<VideoLink>,
}

impl MyMeetingRegistrations {
    /// La personne entre dans l'empreinte : deux comptes sans inscription ne
    /// partagent jamais la même.
    pub fn empreinte(&self, personne: Uuid) -> String {
        let corps = serde_json::to_string(self).unwrap_or_default();
        kernel::empreinte::de(&format!("{personne}:{corps}"))
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct MeetingRegistration {
    pub meeting_id: Uuid,
    pub status: EtatDInscription,
    pub waitlist_position: Option<i32>,
    pub client_ref: Option<Uuid>,
    #[serde(with = "time::serde::rfc3339")]
    pub registered_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize)]
pub struct VideoLink {
    pub meeting_id: Uuid,
    pub url: String,
}

/// `MeetingRegistrationPayload` — un `client_ref` neuf par geste (R5).
#[derive(Debug, Clone, Deserialize)]
pub struct MeetingRegistrationPayload {
    pub client_ref: Uuid,
}

/// `MeetingRegistrationState` — ce que rend `PUT`.
#[derive(Debug, Clone, Serialize)]
pub struct MeetingRegistrationState {
    pub status: EtatDInscription,
    pub waitlist_position: Option<i32>,
}

/// `cancelled` n'est rendu que par le rejeu d'un geste suivi d'une désinscription.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EtatDInscription {
    Registered,
    Waitlisted,
    Cancelled,
}

impl EtatDInscription {
    pub fn from_db(valeur: &str) -> Self {
        match valeur {
            "registered" => Self::Registered,
            "waitlisted" => Self::Waitlisted,
            _ => Self::Cancelled,
        }
    }
}
