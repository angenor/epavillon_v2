//! Les visites de l'espace d'une organisation (02/11/2026) : la page déclare
//! son ouverture, l'onglet « Visites » du back-office les relit avec les
//! connexions, les modifications et les refus des mêmes personnes.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

/// Ce que déclare la page ouverte : « Mon organisation » (l'organisation) ou un
/// dossier (le dossier, dont l'organisation se déduit).
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SpaceVisit {
    pub organization_id: Option<Uuid>,
    pub proposal_id: Option<Uuid>,
}

/// `OrganizationVisits` : l'onglet entier.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct OrganizationVisits {
    /// Première visite enregistrée sur toute la plateforme : avant, rien ne
    /// pouvait l'être. Nulle tant que personne n'est venu.
    #[serde(with = "time::serde::rfc3339::option")]
    pub recording_since: Option<OffsetDateTime>,
    /// Début de la fenêtre lue ; nul pour « depuis le début ».
    #[serde(with = "time::serde::rfc3339::option")]
    pub window_start: Option<OffsetDateTime>,
    pub members: Vec<VisitMember>,
    pub events: Vec<VisitEvent>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct VisitMember {
    pub person_id: Uuid,
    pub display_name: String,
    pub role: String,
    /// Faux pour une personne qui n'est plus membre mais est venue dans la
    /// fenêtre : ses visites restent lisibles.
    pub active: bool,
    #[serde(with = "time::serde::rfc3339::option")]
    pub last_connection_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub last_visit_at: Option<OffsetDateTime>,
    pub last_visit_reference: Option<String>,
    pub visit_days: i64,
    pub changes: i64,
    #[serde(with = "time::serde::rfc3339::option")]
    pub last_change_at: Option<OffsetDateTime>,
}

/// Un fait de la chronologie. `kind` : `connection`, `visit`, `change`, `refusal`.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct VisitEvent {
    pub kind: String,
    pub person_id: Uuid,
    #[serde(with = "time::serde::rfc3339")]
    pub at: OffsetDateTime,
    /// Fin d'une connexion ou dernière ouverture d'une page ce jour-là.
    #[serde(with = "time::serde::rfc3339::option")]
    pub until: Option<OffsetDateTime>,
    /// Ouvertures de la page ce jour-là.
    pub count: Option<i32>,
    pub proposal_id: Option<Uuid>,
    pub proposal_reference: Option<String>,
    pub proposal_title: Option<serde_json::Value>,
    /// Connexion : navigateur et appareil, déduits de l'agent utilisateur.
    pub device: Option<String>,
    /// Modification : tables touchées (`proposals`, `proposal_speakers`…).
    pub tables: Vec<String>,
    /// Refus : modèle de route, code HTTP, message du catalogue tel que la
    /// personne l'a lu.
    pub route: Option<String>,
    pub status: Option<i16>,
    pub message: Option<String>,
    pub request_id: Option<String>,
}
