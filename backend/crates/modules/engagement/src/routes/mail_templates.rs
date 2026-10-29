//! Les gabarits de courriel personnalisés (29/10).

use actix_web::{web, HttpRequest, HttpResponse};
use kernel::auth::Actor;
use kernel::error::Result;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::mail_template::MailTemplatePayload;
use crate::routes::contexte_de;
use crate::service::mail_templates;
use crate::state::EngagementState;

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/admin/mail-templates", web::get().to(lister))
        .route("/admin/mail-templates", web::post().to(creer))
        .route("/admin/mail-templates/{id}", web::get().to(detail))
        .route("/admin/mail-templates/{id}", web::put().to(modifier))
        .route("/admin/mail-templates/{id}", web::delete().to(supprimer))
        .route("/admin/mail-templates/{id}/preview", web::get().to(apercu));
}

#[derive(Debug, Deserialize)]
pub struct EditionQuery {
    pub event_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct ApercuQuery {
    pub event_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
struct Suppression {
    deleted: bool,
}

/// Les gabarits d'une édition.
#[utoipa::path(
    get,
    description = "`MailTemplate[]` : les gabarits personnalisés utilisables pour l'édition, les siens puis les communs à toutes, avec leurs images (`background`, `logo`, rattachements du module media). `can_edit` faux : un gabarit commun sans la permission globale, qui se choisit sans se modifier.\n\nGardé par `engagement.mailing.send` **sur l'édition** et le périmètre d'administration.",
    path = "/admin/mail-templates",
    tag = "Back-office : envois de courriels",
    operation_id = "engagement_gabarits_lister",
    params(("event_id" = Uuid, Query, description = "L'édition")),
    responses(
        (status = 200, description = "MailTemplate[]", body = Object),
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
    Ok(HttpResponse::Ok().json(mail_templates::lister(&state, acteur, edition.event_id).await?))
}

/// Créer un gabarit.
#[utoipa::path(
    post,
    description = "`MailTemplatePayload` → `MailTemplate`. `event_id` nul : commun à toutes les éditions, ce qui demande la permission globale. `preselect_for_submitters` suppose un événement, et un seul gabarit présélectionné par événement. Les images se posent ensuite par `PUT /media/attachments` sur `('engagement', 'mail_templates', id)`.",
    path = "/admin/mail-templates",
    tag = "Back-office : envois de courriels",
    operation_id = "engagement_gabarits_creer",
    request_body = MailTemplatePayload,
    responses(
        (status = 200, description = "MailTemplate", body = Object),
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
    payload: web::Json<MailTemplatePayload>,
) -> Result<HttpResponse> {
    let ctx = contexte_de(&requete, acteur);
    Ok(HttpResponse::Ok().json(mail_templates::creer(&state, &ctx, acteur, &payload).await?))
}

/// Un gabarit.
#[utoipa::path(
    get,
    description = "`MailTemplate`.",
    path = "/admin/mail-templates/{id}",
    tag = "Back-office : envois de courriels",
    operation_id = "engagement_gabarits_detail",
    params(("id" = Uuid, Path, description = "Le gabarit")),
    responses(
        (status = 200, description = "MailTemplate", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Gabarit inexistant ou hors droit", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn detail(
    state: web::Data<EngagementState>,
    Actor(acteur): Actor,
    id: web::Path<Uuid>,
) -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(mail_templates::detail(&state, acteur, id.into_inner()).await?))
}

/// Modifier un gabarit.
#[utoipa::path(
    put,
    description = "`MailTemplatePayload` → `MailTemplate` : écriture complète. Les envois qui le citent prennent ses nouveaux textes et images au prochain rendu ; un envoi parti n'est pas réécrit.",
    path = "/admin/mail-templates/{id}",
    tag = "Back-office : envois de courriels",
    operation_id = "engagement_gabarits_modifier",
    params(("id" = Uuid, Path, description = "Le gabarit")),
    request_body = MailTemplatePayload,
    responses(
        (status = 200, description = "MailTemplate", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Gabarit inexistant ou hors droit", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "VALIDATION_FAILED", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn modifier(
    state: web::Data<EngagementState>,
    Actor(acteur): Actor,
    requete: HttpRequest,
    id: web::Path<Uuid>,
    payload: web::Json<MailTemplatePayload>,
) -> Result<HttpResponse> {
    let ctx = contexte_de(&requete, acteur);
    Ok(HttpResponse::Ok()
        .json(mail_templates::modifier(&state, &ctx, acteur, id.into_inner(), &payload).await?))
}

/// Supprimer un gabarit.
#[utoipa::path(
    delete,
    description = "`{ deleted: true }`. Les envois qui le citaient gardent sa mise en page, sans ses textes ni ses images.",
    path = "/admin/mail-templates/{id}",
    tag = "Back-office : envois de courriels",
    operation_id = "engagement_gabarits_supprimer",
    params(("id" = Uuid, Path, description = "Le gabarit")),
    responses(
        (status = 200, description = "Supprimé", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Gabarit inexistant ou hors droit", body = crate::routes::openapi::ApiErrorBody),
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
    mail_templates::supprimer(&state, &ctx, acteur, id.into_inner()).await?;
    Ok(HttpResponse::Ok().json(Suppression { deleted: true }))
}

/// L'aperçu d'un gabarit.
#[utoipa::path(
    get,
    description = "`MailTemplatePreview` : le gabarit rendu par la fonction du worker sur un texte d'exemple, avec les visuels de son édition sous les siens. Un gabarit commun demande `event_id`, l'édition de l'aperçu. N'envoie rien.",
    path = "/admin/mail-templates/{id}/preview",
    tag = "Back-office : envois de courriels",
    operation_id = "engagement_gabarits_apercu",
    params(
        ("id" = Uuid, Path, description = "Le gabarit"),
        ("event_id" = Option<Uuid>, Query, description = "L'édition de l'aperçu, pour un gabarit commun"),
    ),
    responses(
        (status = 200, description = "MailTemplatePreview", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Gabarit inexistant ou hors droit", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "VALIDATION_FAILED", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn apercu(
    state: web::Data<EngagementState>,
    Actor(acteur): Actor,
    id: web::Path<Uuid>,
    requete: web::Query<ApercuQuery>,
) -> Result<HttpResponse> {
    Ok(HttpResponse::Ok()
        .json(mail_templates::apercu(&state, acteur, id.into_inner(), requete.event_id).await?))
}
