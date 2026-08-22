//! Le back-office du média : les orphelins et les quotas.
//!
//! Les trois routes sont gardées par `org.organization.manage` **sur la portée
//! globale** — aucune permission `media.*` n'existe (écart n° 127) — et un
//! compte sans aucun périmètre d'administration reçoit un **refus explicite**,
//! jamais une liste vide.

use actix_web::{web, HttpRequest, HttpResponse};
use kernel::auth::Actor;
use kernel::error::Result;
use uuid::Uuid;

use crate::routes::contexte_de;
use crate::service::admin::{self, OrphelinsQuery, PlafondPayload};
use crate::state::MediaState;

/// **Trois routes plates.** `/admin` est un préfixe partagé — B5 y pose son
/// tableau de programmation, B6 ses règles de rappel et sa liste de
/// suppression —, et deux `web::scope` du même préfixe ne se complètent pas :
/// Actix retient le premier et rend 404 si la route n'y figure pas.
pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/admin/media/orphans", web::get().to(orphelins))
        .route("/admin/media/quotas", web::get().to(quotas))
        .route(
            "/admin/media/quotas/{organizationId}",
            web::put().to(relever_le_plafond),
        );
}

/// Les objets que plus rien n'utilise.
#[utoipa::path(
    get,
    description = "`OrphanAsset[]` — les objets **servables et non rattachés** depuis un délai, du plus lourd au plus léger, **variantes comprises**. C'est le mécanisme qui manquait totalement à la v1 : sans registre des usages, un fichier retiré d'une page restait sur le disque pour toujours.\n\n`min_age_days` remplace le délai par défaut, qui vient des réglages. Zéro est accepté et rend **tous** les objets non rattachés, y compris ceux déposés il y a une minute : c'est ce qu'on veut pour vérifier, jamais pour purger en masse.\n\nUn objet **rattaché n'y figure jamais**, quel que soit son âge — c'est la définition de l'orphelin que porte le modèle.",
    path = "/admin/media/orphans",
    tag = "Back-office — médias",
    operation_id = "media_orphelins",
    params(("min_age_days" = Option<i32>, Query, description = "Ancienneté minimale en jours")),
    responses(
        (status = 200, description = "OrphanAsset[]", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente, ou périmètre d'administration vide", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn orphelins(
    state: web::Data<MediaState>,
    Actor(acteur): Actor,
    requete: web::Query<OrphelinsQuery>,
) -> Result<HttpResponse> {
    let lignes = admin::orphelins(&state, acteur, requete.min_age_days).await?;
    Ok(HttpResponse::Ok().json(lignes))
}

/// Le tableau des quotas.
#[utoipa::path(
    get,
    description = "`QuotaRow[]` — plafond, consommation, nombre de fichiers et **part consommée**, par organisation, **triés par proximité du plafond** : ce qui demande une décision est en haut.\n\n**Une organisation qui n'a rien déposé n'y figure pas** : sa ligne de quota n'existe pas tant qu'aucun octet n'a été écrit, et le plafond par défaut s'applique. L'absence de ligne est donc « rien déposé », jamais « aucun quota ».",
    path = "/admin/media/quotas",
    tag = "Back-office — médias",
    operation_id = "media_quotas",
    responses(
        (status = 200, description = "QuotaRow[]", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente, ou périmètre d'administration vide", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn quotas(
    state: web::Data<MediaState>,
    Actor(acteur): Actor,
) -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(admin::quotas(&state, acteur).await?))
}

/// Relever le plafond d'une organisation.
#[utoipa::path(
    put,
    description = "`QuotaRow` — le relèvement **prend effet immédiatement** : `media.has_storage_capacity()` lit la ligne à chaque dépôt, sans cache ni rafraîchissement.\n\nLa ligne est créée si elle n'existe pas : un plafond peut être relevé **avant** le premier dépôt, et les compteurs partent alors de zéro. Le geste est tracé par le journal d'audit du modèle.",
    path = "/admin/media/quotas/{organizationId}",
    tag = "Back-office — médias",
    operation_id = "media_relever_le_plafond",
    params(("organizationId" = Uuid, Path, description = "L'organisation dont on relève le plafond")),
    request_body = PlafondPayload,
    responses(
        (status = 200, description = "QuotaRow", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente, ou périmètre d'administration vide", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Organisation inexistante", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "VALIDATION_FAILED", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn relever_le_plafond(
    state: web::Data<MediaState>,
    Actor(acteur): Actor,
    requete: HttpRequest,
    organisation: web::Path<Uuid>,
    payload: web::Json<PlafondPayload>,
) -> Result<HttpResponse> {
    let ctx = contexte_de(&requete, acteur);
    let ligne =
        admin::relever_le_plafond(&state, &ctx, acteur, organisation.into_inner(), &payload)
            .await?;
    Ok(HttpResponse::Ok().json(ligne))
}
