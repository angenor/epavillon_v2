//! La personne telle que l'API la rend — le type `Person` du site, champ pour
//! champ.
//!
//! Aucun secret n'y figure : ni empreinte, ni compteur d'échecs, ni verrou.
//! `display_name` et `search_vector` sont des colonnes générées ; la première se
//! lit, la seconde n'a aucune représentation côté client.

use serde::Serialize;
use time::OffsetDateTime;
use uuid::Uuid;

use super::ids::PersonId;
use super::login::PersonStatus;

#[derive(Debug, Clone, Serialize)]
pub struct PersonView {
    pub id: PersonId,
    pub primary_email: String,
    #[serde(with = "time::serde::rfc3339::option")]
    pub email_verified_at: Option<OffsetDateTime>,
    pub first_name: String,
    pub last_name: String,
    pub civility: Option<String>,
    pub display_name: String,
    pub phone: Option<String>,
    pub job_title: Option<String>,
    pub biography: Option<serde_json::Value>,
    pub country_id: Option<Uuid>,
    pub city: Option<String>,
    pub preferred_locale: String,
    pub timezone: String,
    pub primary_organization_id: Option<Uuid>,
    pub status: PersonStatus,
    pub status_reason: Option<String>,
    pub status_changed_by: Option<PersonId>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub status_changed_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub suspended_until: Option<OffsetDateTime>,
    pub is_directory_visible: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}
