//! Le fil des notifications : lire, marquer lu, archiver.

use actix_web::{web, HttpRequest, HttpResponse};
use kernel::auth::Actor;
use kernel::error::Result;
use serde::Serialize;

use crate::routes::contexte_de;
use crate::service::notifications::{self, ArchivagePayload, FilQuery, MarquagePayload};
use crate::state::EngagementState;

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/notifications", web::get().to(lire))
        .route("/notifications/read", web::post().to(marquer_lues))
        .route("/notifications/archive", web::post().to(archiver));
}

#[derive(Debug, Serialize)]
struct Compte {
    marked: u64,
}

#[derive(Debug, Serialize)]
struct Archivees {
    archived: u64,
}

/// Le fil, **et le compte de non lues**.
#[utoipa::path(
    get,
    description = "`NotificationFeed` — les lignes **et** le nombre de non lues, dans la même réponse. Deux appels donneraient deux chiffres mesurés à deux instants, et un badge qui contredit la liste qu'il coiffe.\n\nLe compte porte sur **toutes** les non lues, pas sur la page : un badge qui ne compterait que la page afficherait « 30 » pour toujours.\n\nUne ligne peut porter un `group_count` supérieur à un : trois faits de même nature sur la même cible forment **une** ligne tant qu'elle n'est pas lue.",
    path = "/notifications",
    tag = "Notifications",
    operation_id = "engagement_fil_de_notifications",
    params(
        ("unread_only" = Option<bool>, Query, description = "Ne rendre que les non lues"),
        ("limit" = Option<i64>, Query, description = "Taille de page, bornée à 100"),
        ("before" = Option<String>, Query, description = "Pagination : avant cet instant"),
    ),
    responses(
        (status = 200, description = "NotificationFeed", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn lire(
    state: web::Data<EngagementState>,
    Actor(acteur): Actor,
    requete: web::Query<FilQuery>,
) -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(notifications::fil(&state, acteur, &requete).await?))
}

/// Marquer lues.
#[utoipa::path(
    post,
    description = "`{ marked }` — sans `ids`, **toutes** les non lues de la personne. Les siennes, et uniquement : le filtre porte sur le compte de l'appelant, jamais sur la seule liste d'identifiants reçue.",
    path = "/notifications/read",
    tag = "Notifications",
    operation_id = "engagement_marquer_notifications_lues",
    request_body = MarquagePayload,
    responses(
        (status = 200, description = "{ marked }", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn marquer_lues(
    state: web::Data<EngagementState>,
    Actor(acteur): Actor,
    requete: HttpRequest,
    payload: Option<web::Json<MarquagePayload>>,
) -> Result<HttpResponse> {
    let ctx = contexte_de(&requete, acteur);
    let payload = payload.map(|p| p.into_inner()).unwrap_or_default();
    let marked = notifications::marquer_lues(&state, &ctx, acteur, &payload).await?;
    Ok(HttpResponse::Ok().json(Compte { marked }))
}

/// Archiver.
#[utoipa::path(
    post,
    description = "`{ archived }` — la liste d'identifiants est **exigée** : « tout archiver » n'est pas un geste qu'on fait par mégarde. Archiver marque aussi lu, une notification rangée n'ayant plus à peser sur le badge.",
    path = "/notifications/archive",
    tag = "Notifications",
    operation_id = "engagement_archiver_notifications",
    request_body = ArchivagePayload,
    responses(
        (status = 200, description = "{ archived }", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn archiver(
    state: web::Data<EngagementState>,
    Actor(acteur): Actor,
    requete: HttpRequest,
    payload: web::Json<ArchivagePayload>,
) -> Result<HttpResponse> {
    let ctx = contexte_de(&requete, acteur);
    let archived = notifications::archiver(&state, &ctx, acteur, &payload).await?;
    Ok(HttpResponse::Ok().json(Archivees { archived }))
}
