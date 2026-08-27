//! Vivacité et santé d'exploitation.
//!
//! **Deux routes, deux publics.** `/ready` répond à l'orchestrateur : elle ne
//! demande rien, ne divulgue rien, et dit seulement si le processus sait encore
//! parler à sa base. `/health` porte des chiffres d'exploitation — profondeur de
//! l'outbox, travaux morts, courriels en rebond — et se protège comme une
//! donnée. Les confondre publierait l'état interne de la plateforme à qui sonde
//! le port.
//!
//! Les indicateurs et leurs seuils viennent de `analytics.v_operational_health`
//! et **ne sont pas recalculés ici** : le modèle porte déjà la décision de ce
//! qui mérite attention, et la redoubler en Rust ferait deux vérités.

use actix_web::{web, HttpResponse};
use kernel::auth::RequiresAnyScope;
use kernel::db::Db;
use kernel::error::{ApiError, ErrorCode, Result};
use serde::Serialize;
use serde_json::Value;
use time::OffsetDateTime;

/// La permission est **déclarée par le module `analytics`**, à qui elle
/// appartient, et réemployée ici.
///
/// **La route, elle, ne bouge pas.** Elle fait paire avec `/ready` — vivacité
/// anonyme d'un côté, santé protégée de l'autre — et montée derrière
/// `is_mounted("analytics")` elle disparaîtrait le jour où le module serait
/// éteint, emportant la sonde d'exploitation avec lui.
use analytics::authz::DashboardRead;

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/ready", web::get().to(ready))
        .route("/health", web::get().to(health));
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct Readiness {
    pub status: &'static str,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct OperationalHealth {
    /// La pire gravité rencontrée : `ok`, `attention` ou `critique`.
    pub status: String,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub measured_at: OffsetDateTime,
    pub indicators: Vec<HealthIndicator>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct HealthIndicator {
    pub code: String,
    pub label: String,
    pub domain: String,
    pub value: i64,
    pub warning_threshold: i64,
    pub critical_threshold: i64,
    pub severity: String,
    #[schema(value_type = Object)]
    pub detail: Value,
}

#[utoipa::path(
    get,
    description = "Vivacité : le processus répond et son pool de connexions est ouvert. Aucune autorisation, aucune divulgation — c'est l'orchestrateur qui la lit.",
    path = "/ready",
    tag = "Exploitation",
    operation_id = "ready",
    responses(
        (status = 200, description = "Le processus est prêt à servir", body = Readiness),
        (status = 503, description = "Base injoignable", body = crate::openapi::ApiErrorBody),
    )
)]
/// Vivacité. **Le pool est réellement sollicité** : un processus qui répond
/// pendant que sa base est injoignable n'est pas prêt, et le dire en 200 ferait
/// router du trafic vers un serveur qui ne peut rien servir.
async fn ready(db: web::Data<Db>) -> Result<HttpResponse> {
    sqlx::query_scalar!(r#"SELECT 1 AS "un!""#)
        .fetch_one(db.pool())
        .await
        .map_err(|e| ApiError::new(ErrorCode::ServiceUnavailable).detail(e))?;

    Ok(HttpResponse::Ok().json(Readiness { status: "ok" }))
}

#[utoipa::path(
    get,
    description = "État d'exploitation, depuis `analytics.v_operational_health` : outbox en retard, travaux en échec, courriels en rebond, partitions manquantes. Protégé comme une donnée.",
    path = "/health",
    tag = "Exploitation",
    operation_id = "health",
    responses(
        (status = 200, description = "Les indicateurs et leurs seuils", body = OperationalHealth),
        (status = 401, description = "Aucune session", body = crate::openapi::ApiErrorBody),
        (status = 403, description = "`analytics.dashboard.read` absente, quelle que soit la portée", body = crate::openapi::ApiErrorBody),
    )
)]
/// Santé d'exploitation. Ce sont les chiffres de la plateforme entière, et il
/// n'existe aucune édition à laquelle les rapporter — mais la permission n'est
/// PAS exigée en portée globale pour autant : ce que ces indicateurs révèlent —
/// des courriels en rebond, un outbox en retard — touche d'abord les rappels des
/// activités d'un administrateur détaché, qui doit pouvoir les voir. La portée
/// commande ce qu'on lit, pas la nature de ce qu'on regarde.
async fn health(
    db: web::Data<Db>,
    _permission: RequiresAnyScope<DashboardRead>,
) -> Result<HttpResponse> {
    let lignes = sqlx::query!(
        r#"SELECT code            AS "code!",
                  libelle         AS "libelle!",
                  domaine         AS "domaine!",
                  valeur          AS "valeur!",
                  seuil_attention AS "seuil_attention!",
                  seuil_critique  AS "seuil_critique!",
                  gravite         AS "gravite!",
                  detail          AS "detail!",
                  mesure_le       AS "mesure_le!"
             FROM analytics.v_operational_health"#
    )
    .fetch_all(db.pool())
    .await?;

    // La vue trie déjà du plus grave au plus calme : le premier porte donc la
    // pire gravité, et un tableau vide est un `ok` — il n'y a rien à signaler.
    let status = lignes
        .first()
        .map(|l| l.gravite.clone())
        .unwrap_or_else(|| "ok".to_owned());
    let measured_at = lignes
        .first()
        .map(|l| l.mesure_le)
        .unwrap_or_else(OffsetDateTime::now_utc);

    let indicators = lignes
        .into_iter()
        .map(|l| HealthIndicator {
            code: l.code,
            label: l.libelle,
            domain: l.domaine,
            value: l.valeur,
            warning_threshold: l.seuil_attention,
            critical_threshold: l.seuil_critique,
            severity: l.gravite,
            detail: l.detail,
        })
        .collect();

    Ok(HttpResponse::Ok().json(OperationalHealth {
        status,
        measured_at,
        indicators,
    }))
}
