//! La réponse entière, et la zone 3 — **la santé opérationnelle**.
//!
//! Ce qui casse en silence : l'outbox qui ne part plus, les courriels qui
//! rebondissent. Trois niveaux de gravité, **calculés en base avec leurs
//! seuils** — le code ne les recalcule pas, et la vue est rendue par le **code**
//! de l'indicateur, son libellé français restant un repli.

use serde::Serialize;
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use super::action::AdminAction;
use super::figures::DashboardFigures;

/// L'édition mesurée — `event.events` telle quelle.
#[derive(Debug, Clone, Serialize)]
pub struct EventEdition {
    pub id: Uuid,
    pub series_id: Option<Uuid>,
    pub edition_label: Option<String>,
    pub edition_year: i16,
    pub title: Value,
    pub acronym: Option<String>,
    pub slug: String,
    pub description: Value,
    pub status: String,
    pub participation_mode: String,
    pub timezone: String,
    #[serde(with = "time::serde::rfc3339")]
    pub starts_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub ends_at: OffsetDateTime,
    pub country_id: Option<Uuid>,
    pub city: Option<String>,
    pub address: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub has_pavilion: bool,
    #[serde(with = "time::serde::rfc3339::option")]
    pub programme_published_at: Option<OffsetDateTime>,
    pub highlights: Option<Value>,
    pub created_by: Option<Uuid>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

/// L'appel de l'édition — `event.calls_for_proposals` telle quelle. **Zéro ou
/// un, jamais deux** (règle métier n° 5).
#[derive(Debug, Clone, Serialize)]
pub struct CallForProposals {
    pub id: Uuid,
    pub event_id: Uuid,
    pub code: String,
    pub title: Value,
    pub description: Option<Value>,
    pub status: String,
    #[serde(with = "time::serde::rfc3339")]
    pub opens_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub closes_at: OffsetDateTime,
    /// Prolongation, conservée à part pour garder trace de l'échéance annoncée
    /// à l'origine. L'échéance qui fait foi vient d'`event.effective_deadline()`.
    #[serde(with = "time::serde::rfc3339::option")]
    pub extended_until: Option<OffsetDateTime>,
    pub results_expected_at: Option<time::Date>,
    pub max_proposals_per_organization: Option<i16>,
    pub requires_verified_organization: bool,
    pub min_speakers: i16,
    pub max_speakers: i16,
    pub default_duration_minutes: i16,
    pub min_duration_minutes: i16,
    pub max_duration_minutes: i16,
    pub daily_start_time: String,
    pub daily_end_time: String,
    pub allowed_formats: Vec<String>,
    pub required_reviews: i16,
    pub blind_review: bool,
    pub guidelines_url: Option<String>,
    pub created_by: Option<Uuid>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

/// Une ligne de `analytics.v_operational_health`, **telle quelle**.
#[derive(Debug, Clone, Serialize)]
pub struct OperationalHealthRow {
    pub code: String,
    pub libelle: String,
    pub domaine: String,
    pub valeur: i64,
    pub seuil_attention: i64,
    pub seuil_critique: i64,
    pub gravite: String,
    pub detail: Value,
    #[serde(with = "time::serde::rfc3339")]
    pub mesure_le: OffsetDateTime,
}

/// Un incident actif de l'édition — `live.active_incidents_for_event()`.
///
/// Elle **descend** la hiérarchie (édition → journées → activités →
/// organisations qui y animent), là où `live.active_incidents(session)` la
/// remonte. Sans elle, le tableau de bord et l'écran des messages
/// recomposeraient chacun ce balayage.
#[derive(Debug, Clone, Serialize)]
pub struct EventIncident {
    pub incident_id: Uuid,
    pub scope: String,
    pub severity: String,
    pub kind_code: String,
    pub title: Option<Value>,
    pub message: Value,
    /// Cible **résolue par la fonction** : nom d'activité, de journée,
    /// d'organisation.
    pub target_label: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub display_from: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub display_until: Option<OffsetDateTime>,
}

/// **Tout l'écran en une réponse, et en un instant**, pour l'édition
/// sélectionnée.
///
/// `timezone` est celui de l'ÉDITION, et toute date affichée le porte — un
/// chevauchement de créneaux ne se lit pas dans le fuseau du navigateur de la
/// personne qui arbitre.
#[derive(Debug, Clone, Serialize)]
pub struct AdminDashboard {
    pub edition: EventEdition,
    pub timezone: String,
    pub call: Option<CallForProposals>,
    pub actions: Vec<AdminAction>,
    pub figures: DashboardFigures,
    pub health: Vec<OperationalHealthRow>,
    pub incidents: Vec<EventIncident>,
}
