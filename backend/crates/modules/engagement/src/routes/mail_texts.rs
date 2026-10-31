//! Les textes types des courriels (01/11).

use actix_web::{web, HttpRequest, HttpResponse};
use kernel::auth::Actor;
use kernel::error::Result;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::mail_text::MailTextPayload;
use crate::routes::contexte_de;
use crate::service::mail_texts;
use crate::state::EngagementState;

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/admin/mail-texts", web::get().to(lister))
        .route("/admin/mail-texts", web::post().to(creer))
        .route("/admin/mail-texts/{id}", web::get().to(detail))
        .route("/admin/mail-texts/{id}", web::put().to(modifier))
        .route("/admin/mail-texts/{id}", web::delete().to(supprimer));
}

#[derive(Debug, Deserialize)]
pub struct EditionQuery {
    pub event_id: Uuid,
}

#[derive(Debug, Serialize)]
struct Suppression {
    deleted: bool,
}

/// Les textes types d'une édition.
#[utoipa::path(
    get,
    description = "`MailText[]` : les textes types utilisables pour l'édition, les siens et les communs à toutes, rangés par catégorie, rang puis nom. `can_edit` faux : un texte commun sans la permission globale, qui se choisit sans se modifier.\n\nGardé par `engagement.mailing.send` **sur l'édition** et le périmètre d'administration.",
    path = "/admin/mail-texts",
    tag = "Back-office : envois de courriels",
    operation_id = "engagement_textes_types_lister",
    params(("event_id" = Uuid, Query, description = "L'édition")),
    responses(
        (status = 200, description = "MailText[]", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente sur l'édition", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Édition inexistante ou hors périmètre", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn lister(
    state: web::Data<EngagementState>,
    Actor(acteur): Actor,
    edition: web::Query<EditionQuery>,
) -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(mail_texts::lister(&state, acteur, edition.event_id).await?))
}

/// Créer un texte type.
#[utoipa::path(
    post,
    description = "`MailTextPayload` → `MailText`. `event_id` nul : commun à toutes les éditions, ce qui demande la permission globale. Le corps est **assaini** (liste blanche des courriels) ; les variables ne sont vérifiées qu'à l'envoi.",
    path = "/admin/mail-texts",
    tag = "Back-office : envois de courriels",
    operation_id = "engagement_textes_types_creer",
    request_body = MailTextPayload,
    responses(
        (status = 200, description = "MailText", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Édition inexistante ou hors périmètre", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "VALIDATION_FAILED", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn creer(
    state: web::Data<EngagementState>,
    Actor(acteur): Actor,
    requete: HttpRequest,
    payload: web::Json<MailTextPayload>,
) -> Result<HttpResponse> {
    let ctx = contexte_de(&requete, acteur);
    Ok(HttpResponse::Ok().json(mail_texts::creer(&state, &ctx, acteur, &payload).await?))
}

/// Un texte type.
#[utoipa::path(
    get,
    description = "`MailText`.",
    path = "/admin/mail-texts/{id}",
    tag = "Back-office : envois de courriels",
    operation_id = "engagement_textes_types_detail",
    params(("id" = Uuid, Path, description = "Le texte type")),
    responses(
        (status = 200, description = "MailText", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Texte inexistant ou hors droit", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn detail(
    state: web::Data<EngagementState>,
    Actor(acteur): Actor,
    id: web::Path<Uuid>,
) -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(mail_texts::detail(&state, acteur, id.into_inner()).await?))
}

/// Modifier un texte type.
#[utoipa::path(
    put,
    description = "`MailTextPayload` → `MailText` : écriture complète. Les envois qui l'ont repris gardent leur objet et leur corps.",
    path = "/admin/mail-texts/{id}",
    tag = "Back-office : envois de courriels",
    operation_id = "engagement_textes_types_modifier",
    params(("id" = Uuid, Path, description = "Le texte type")),
    request_body = MailTextPayload,
    responses(
        (status = 200, description = "MailText", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Texte inexistant ou hors droit", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "VALIDATION_FAILED", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn modifier(
    state: web::Data<EngagementState>,
    Actor(acteur): Actor,
    requete: HttpRequest,
    id: web::Path<Uuid>,
    payload: web::Json<MailTextPayload>,
) -> Result<HttpResponse> {
    let ctx = contexte_de(&requete, acteur);
    Ok(HttpResponse::Ok()
        .json(mail_texts::modifier(&state, &ctx, acteur, id.into_inner(), &payload).await?))
}

/// Supprimer un texte type.
#[utoipa::path(
    delete,
    description = "`{ deleted: true }`. Les envois qui l'avaient repris gardent leur objet et leur corps.",
    path = "/admin/mail-texts/{id}",
    tag = "Back-office : envois de courriels",
    operation_id = "engagement_textes_types_supprimer",
    params(("id" = Uuid, Path, description = "Le texte type")),
    responses(
        (status = 200, description = "Supprimé", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Texte inexistant ou hors droit", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn supprimer(
    state: web::Data<EngagementState>,
    Actor(acteur): Actor,
    requete: HttpRequest,
    id: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let ctx = contexte_de(&requete, acteur);
    mail_texts::supprimer(&state, &ctx, acteur, id.into_inner()).await?;
    Ok(HttpResponse::Ok().json(Suppression { deleted: true }))
}
