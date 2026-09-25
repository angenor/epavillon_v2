//! L'import de la source officielle et l'ordre du jour, vus du back-office
//! (FR-040). Aucune forme ici ne porte une session : la source fait foi (FR-041).

use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

/// `OfficialImportAdmin` — ce que `GET`/`PUT /admin/negotiation/import` rendent.
#[derive(Debug, Clone, Serialize)]
pub struct OfficialImportAdmin {
    pub edition: EditionImport,
    pub enabled: bool,
    pub reader: String,
    pub archive_name: Option<String>,
    pub archive_first_day: Option<Date>,
    pub archives: Vec<String>,
    pub live_url: Option<String>,
    pub time_correction_minutes: i16,
    pub official_programme_url: String,
    pub interval_seconds: i32,
    pub missed_threshold: i16,
    pub missed_reads: i16,
    pub serving: bool,
    #[serde(with = "time::serde::rfc3339::option")]
    pub last_success_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub last_attempt_at: Option<OffsetDateTime>,
    pub last_error: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub failing_since: Option<OffsetDateTime>,
    pub last_change_count: Option<i32>,
    pub session_count: i64,
    pub agenda_items_without_theme: i64,
    pub runs: Vec<ImportRun>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EditionImport {
    pub slug: String,
    pub name: Value,
    pub timezone: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportRun {
    #[serde(with = "time::serde::rfc3339")]
    pub started_at: OffsetDateTime,
    pub outcome: String,
    pub error: Option<String>,
    pub session_count: Option<i32>,
    pub change_count: Option<i32>,
    /// « Lire maintenant », et non la chaîne récurrente.
    pub manual: bool,
}

/// `UpdateOfficialImportPayload` — le réglage entier, interrupteur compris.
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateOfficialImportPayload {
    pub enabled: bool,
    pub reader: String,
    pub archive_name: Option<String>,
    pub archive_first_day: Option<Date>,
    pub live_url: Option<String>,
    pub time_correction_minutes: i16,
    pub official_programme_url: String,
    pub interval_seconds: i32,
    pub missed_threshold: i16,
}

/// `AgendaItemAdmin` — un point de l'ordre du jour et sa thématique.
#[derive(Debug, Clone, Serialize)]
pub struct AgendaItemAdmin {
    pub id: Uuid,
    pub code: String,
    pub title: String,
    pub theme: Option<String>,
    pub session_count: i64,
    #[serde(with = "time::serde::rfc3339::option")]
    pub theme_set_at: Option<OffsetDateTime>,
}

/// `AgendaItemThemePayload` — `null` détache le point.
#[derive(Debug, Clone, Deserialize)]
pub struct AgendaItemThemePayload {
    pub theme: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct EditionQuery {
    pub edition: String,
}
