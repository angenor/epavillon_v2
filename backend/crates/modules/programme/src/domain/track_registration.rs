//! L'inscription à une journée (02/11) : un seul direct pour la journée, les
//! activités gardées au choix. Modèle : `075_programme_sessions.sql` § 4 bis.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::service::registration::Invite;

/// `DayRegistration` : l'inscription, ses activités gardées (N) et le nombre
/// d'activités de la journée (M), pour « vous suivez N activités sur M ».
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DayRegistration {
    pub id: Uuid,
    pub track_id: Uuid,
    /// `registered`, `cancelled`, `attended` ou `no_show` ; jamais d'attente.
    pub status: String,
    /// Activités gardées, inscriptions vivantes seulement.
    pub session_ids: Vec<Uuid>,
    /// Activités publiées et non annulées de la journée.
    pub activity_count: i64,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub joined_at: Option<OffsetDateTime>,
}

/// `DayRegistrationResult`. Toutes en 200, comme pour une activité.
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum DayRegistrationResult {
    Registered {
        registration: DayRegistration,
    },
    /// Déjà inscrit. Sans compte, rien n'est rendu (une adresse saisie ne doit
    /// rien apprendre) et le lien de gestion part de nouveau par courriel.
    AlreadyRegistered {
        registration: Option<DayRegistration>,
    },
    /// La journée est passée.
    Closed {
        #[serde(with = "time::serde::rfc3339")]
        closed_at: OffsetDateTime,
    },
}

/// `DayRegisterPayload`.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct DayRegisterPayload {
    pub track_id: Uuid,
    /// Activités gardées ; une activité hors de la journée est refusée.
    #[serde(default)]
    pub session_ids: Vec<Uuid>,
    /// Clés = `code` des champs actifs du formulaire de l'édition.
    #[serde(default)]
    pub answers: serde_json::Value,
    #[serde(default)]
    pub locale: Option<String>,
    /// Sans session seulement, si le formulaire admet l'inscription sans compte.
    #[serde(default)]
    pub guest: Option<Invite>,
    #[serde(default)]
    pub sensitive_data_consent: bool,
    #[serde(default)]
    pub organization_id: Option<Uuid>,
}

/// `DaySessionsPayload` : la nouvelle liste des activités gardées.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct DaySessionsPayload {
    pub session_ids: Vec<Uuid>,
}

/// `DayRegistrationManage` : ce que lit la page de gestion d'un invité.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DayRegistrationManage {
    pub registration: DayRegistration,
    pub event_slug: String,
    pub track_slug: String,
    pub track_title: serde_json::Value,
    pub first_name: String,
}

/// `DayRegistrationRow` : une ligne de la liste nominative du back-office.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DayRegistrationRow {
    pub id: Uuid,
    pub status: String,
    pub person_id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub organization_name: Option<String>,
    pub has_account: bool,
    pub kept_count: i64,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub joined_at: Option<OffsetDateTime>,
}

/// Les activités gardées, dédoublonnées et dans l'ordre reçu.
pub fn sans_doublons(ids: &[Uuid]) -> Vec<Uuid> {
    let mut vus = Vec::with_capacity(ids.len());
    for id in ids {
        if !vus.contains(id) {
            vus.push(*id);
        }
    }
    vus
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_doublons_tombent_dans_lordre_recu() {
        let a = Uuid::from_u128(1);
        let b = Uuid::from_u128(2);
        assert_eq!(sans_doublons(&[b, a, b]), vec![b, a]);
    }
}
