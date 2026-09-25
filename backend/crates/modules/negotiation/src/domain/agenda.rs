//! « Mon agenda » — les sessions officielles qu'une personne garde. L'agenda ne
//! recopie pas la session : ses lignes se lisent dans `OfficialSessions`.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// `MyAgenda` — ce que `GET /negotiation/me/agenda` rend.
#[derive(Debug, Clone, Serialize)]
pub struct MyAgenda {
    pub entries: Vec<AgendaEntry>,
    /// Les réunions non annoncées gardées (3b).
    pub network_entries: Vec<NetworkAgendaEntry>,
}

impl MyAgenda {
    pub fn empreinte(&self) -> String {
        kernel::empreinte::de(&serde_json::to_string(self).unwrap_or_default())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct NetworkAgendaEntry {
    pub network_meeting_id: Uuid,
    /// Effectif : faux dès que la réunion est retirée.
    pub remind: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub added_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgendaEntry {
    pub session_id: Uuid,
    /// Effectif : faux dès que la session est annulée (FR-035).
    pub remind: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub added_at: OffsetDateTime,
}

/// `AgendaEntryPayload` — ajouter, ou changer le rappel.
#[derive(Debug, Clone, Deserialize)]
pub struct AgendaEntryPayload {
    pub remind: bool,
}
