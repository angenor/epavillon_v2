//! Journal du back-office (`/admin/journal`) : ce qui a échoué ou traîné
//! (`platform.ops_log`), ce qui a été écrit (`platform.audit_log`), ce qui
//! attend encore (travaux et événements en échec). Le code de la demande
//! (`request_id`) relie les trois.
//!
//! Porté par l'API et non par un module : ces tables sont transverses, et le
//! journal doit rester lisible quand un module est démonté. Les noms des
//! personnes et de leur organisation sont lus par jointure, en lecture seule.
//!
//! Formes de réponse : `frontend/app/types/admin-journal.ts`.

pub(crate) mod changes;
pub(crate) mod entries;
pub(crate) mod failures;

use actix_web::web;
use kernel::auth::PermissionSpec;
use kernel::error::{ApiError, ErrorCode};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use changes::modifications;
use entries::{demande, entrees, entrees_csv, resume};
use failures::{echecs, relancer_evenement, relancer_travail};

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/admin/journal/summary", web::get().to(resume))
        .route("/admin/journal/entries", web::get().to(entrees))
        .route("/admin/journal/entries.csv", web::get().to(entrees_csv))
        .route(
            "/admin/journal/requests/{request_id}",
            web::get().to(demande),
        )
        .route("/admin/journal/changes", web::get().to(modifications))
        .route("/admin/journal/failures", web::get().to(echecs))
        .route(
            "/admin/journal/jobs/{id}/retry",
            web::post().to(relancer_travail),
        )
        .route(
            "/admin/journal/events/{id}/retry",
            web::post().to(relancer_evenement),
        );
}

/// Portée globale seulement : le journal mêle toutes les éditions.
pub struct JournalRead;

impl PermissionSpec for JournalRead {
    const CODE: &'static str = "identity.platform.journal";
}

/// Rétention de `platform.purge_ops_log`, appelée par le worker.
pub const RETENTION_MOIS: u8 = 3;
const PAR_PAGE: i64 = 50;
const PAR_PAGE_MAX: i64 = 200;

#[derive(Debug, Clone, Copy, Default, Deserialize)]
pub enum Period {
    #[default]
    #[serde(rename = "24h")]
    Day,
    #[serde(rename = "7d")]
    Week,
    #[serde(rename = "30d")]
    Month,
    #[serde(rename = "90d")]
    Quarter,
}

impl Period {
    fn debut(self) -> OffsetDateTime {
        let jours = match self {
            Period::Day => 1,
            Period::Week => 7,
            Period::Month => 30,
            Period::Quarter => 90,
        };
        OffsetDateTime::now_utc() - time::Duration::days(jours)
    }
}

/// Depuis le début de la rétention : la recherche par code de demande n'a pas
/// de période.
fn debut_de_retention() -> OffsetDateTime {
    OffsetDateTime::now_utc() - time::Duration::days(31 * (i64::from(RETENTION_MOIS) + 1))
}

#[derive(Debug, Serialize)]
pub struct JournalPerson {
    pub id: Uuid,
    pub display_name: String,
    pub organization_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

/// `(page, par_page, décalage)`, bornés.
fn pagination(page: Option<i64>, par_page: Option<i64>) -> (i64, i64, i64) {
    let par_page = par_page.unwrap_or(PAR_PAGE).clamp(1, PAR_PAGE_MAX);
    let page = page.unwrap_or(1).max(1);
    (page, par_page, (page - 1).saturating_mul(par_page))
}

/// Recherche libre : vide devient absente, longueur bornée.
fn recherche(q: Option<&str>) -> Option<String> {
    q.map(str::trim)
        .filter(|q| !q.is_empty())
        .map(|q| q.chars().take(200).collect())
}

/// Motif `ILIKE` : les jokers saisis sont pris à la lettre.
fn motif(q: &str) -> String {
    q.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn invalide(champ: &str) -> ApiError {
    ApiError::validation(format!("Valeur inconnue pour « {champ} »."), champ)
}

fn conflit(message: &str) -> ApiError {
    ApiError::with_message(ErrorCode::Conflict, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pagination_bornee() {
        assert_eq!(pagination(None, None), (1, 50, 0));
        assert_eq!(pagination(Some(3), Some(20)), (3, 20, 40));
        assert_eq!(pagination(Some(0), Some(10_000)), (1, 200, 0));
    }

    #[test]
    fn jokers_pris_a_la_lettre() {
        assert_eq!(motif("50%_a\\b"), "50\\%\\_a\\\\b");
        assert_eq!(recherche(Some("  ")), None);
        assert_eq!(recherche(Some(" x ")).as_deref(), Some("x"));
    }
}
