//! Les drapeaux de fonctionnalité, **résolus pour qui demande**.
//!
//! LE SITE NE REFAIT PAS LE CALCUL. Un drapeau ne dit pas seulement « ouvert » ou
//! « fermé » : il porte un pourcentage de déploiement et une liste de personnes
//! explicitement ouvertes, et c'est `platform.is_feature_enabled()` qui tranche.
//! Rendre la table brute obligerait le site à réimplémenter cette fonction — donc
//! à en donner une seconde version, qui divergerait au premier ajustement — et à
//! publier au passage les identifiants des personnes du déploiement progressif.
//!
//! **Aucune session n'est exigée**, et c'est indispensable : le routage du site
//! lit ces drapeaux à la première navigation, avant même de savoir qui regarde.
//! Sans session, seul un déploiement à 100 % ouvre — la valeur sûre.
//!
//! LA DESCRIPTION N'EST PAS RENDUE. Elle est écrite pour l'exploitant, en clair,
//! et nomme parfois ce qui n'est pas encore annoncé.

use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use kernel::auth::{PermissionSpec, Requires};
use kernel::context::RequestContext;
use kernel::db::Db;
use kernel::error::{ApiError, Result};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/platform/feature-flags", web::get().to(drapeaux))
        .route("/admin/platform/maintenance", web::get().to(maintenance))
        .route(
            "/admin/platform/maintenance",
            web::put().to(basculer_maintenance),
        );
}

/// Portée globale seulement : fermer la plateforme touche toutes les éditions.
pub struct MaintenanceManage;

impl PermissionSpec for MaintenanceManage {
    const CODE: &'static str = "identity.platform.maintenance";
}

#[derive(Debug, Serialize)]
pub struct PlatformMaintenance {
    pub enabled: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct PlatformMaintenanceUpdate {
    pub enabled: bool,
}

#[derive(Debug, Serialize)]
pub struct ResolvedFeatureFlag {
    pub key: String,
    /// Le verdict pour l'appelant, déjà calculé par `platform.is_feature_enabled()`.
    pub is_enabled: bool,
}

#[utoipa::path(
    get,
    description = "`ResolvedFeatureFlag[]` — chaque drapeau et son verdict POUR L'APPELANT. Le déploiement progressif est tranché par `platform.is_feature_enabled()`, jamais par le site : une seconde implémentation du calcul divergerait, et rendre `enabled_for` publierait les identifiants des personnes visées. Sans session, seul un déploiement à 100 % ouvre.",
    path = "/platform/feature-flags",
    tag = "Plateforme",
    operation_id = "platform_drapeaux",
    responses((status = 200, description = "ResolvedFeatureFlag[]", body = Object))
)]
pub(crate) async fn drapeaux(db: web::Data<Db>, requete: HttpRequest) -> Result<HttpResponse> {
    // Hors session, `actor_id` est nul : `is_feature_enabled()` n'ouvre alors
    // qu'un drapeau déployé à 100 %, ce qui est la valeur sûre.
    let personne = requete
        .extensions()
        .get::<RequestContext>()
        .and_then(|ctx| ctx.actor_id);

    Ok(HttpResponse::Ok().json(resoudre(db.pool(), personne).await?))
}

pub(crate) async fn resoudre(
    pool: &PgPool,
    personne: Option<Uuid>,
) -> Result<Vec<ResolvedFeatureFlag>> {
    Ok(sqlx::query_as!(
        ResolvedFeatureFlag,
        r#"SELECT key AS "key!",
                  platform.is_feature_enabled(key, $1) AS "is_enabled!"
             FROM platform.feature_flags
            ORDER BY key"#,
        personne
    )
    .fetch_all(pool)
    .await?)
}

#[utoipa::path(
    get,
    description = "`PlatformMaintenance` — la plateforme est-elle fermée aux non-administrateurs ? Lu en base à chaque appel, jamais dans le cache du site : l'interrupteur doit dire l'état réel.",
    path = "/admin/platform/maintenance",
    tag = "Plateforme",
    operation_id = "admin_plateforme_maintenance",
    responses(
        (status = 200, description = "PlatformMaintenance", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente en portée globale", body = crate::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn maintenance(
    _droit: Requires<MaintenanceManage>,
    db: web::Data<Db>,
) -> Result<HttpResponse> {
    let etat = sqlx::query_as!(
        PlatformMaintenance,
        r#"SELECT (is_enabled AND rollout_percent = 100) AS "enabled!", updated_at
             FROM platform.feature_flags
            WHERE key = 'platform.maintenance'"#
    )
    .fetch_optional(db.pool())
    .await?
    .ok_or_else(ApiError::not_found)?;

    Ok(HttpResponse::Ok().json(etat))
}

#[utoipa::path(
    put,
    description = "`PlatformMaintenanceUpdate` → `PlatformMaintenance`. Fermer réserve le site et les écritures aux personnes qui administrent ; la connexion reste ouverte. Le déploiement suit (100 % ou 0 %) : le drapeau se résout sans personne, un pourcentage partiel ne fermerait rien.",
    path = "/admin/platform/maintenance",
    tag = "Plateforme",
    operation_id = "admin_plateforme_maintenance_basculer",
    request_body(content = Object, description = "PlatformMaintenanceUpdate"),
    responses(
        (status = 200, description = "PlatformMaintenance", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente en portée globale", body = crate::openapi::ApiErrorBody),
        (status = 404, description = "Drapeau absent : migration du 16/10 non jouée", body = crate::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn basculer_maintenance(
    _droit: Requires<MaintenanceManage>,
    db: web::Data<Db>,
    requete: HttpRequest,
    corps: web::Json<PlatformMaintenanceUpdate>,
) -> Result<HttpResponse> {
    let contexte = requete
        .extensions()
        .get::<RequestContext>()
        .cloned()
        .ok_or_else(ApiError::unauthenticated)?;

    let mut tx = db.write(&contexte).await?;
    let etat = sqlx::query_as!(
        PlatformMaintenance,
        r#"UPDATE platform.feature_flags
              SET is_enabled = $1,
                  rollout_percent = CASE WHEN $1 THEN 100 ELSE 0 END
            WHERE key = 'platform.maintenance'
        RETURNING is_enabled AS "enabled!", updated_at"#,
        corps.enabled
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(ApiError::not_found)?;
    tx.commit().await?;

    Ok(HttpResponse::Ok().json(etat))
}
