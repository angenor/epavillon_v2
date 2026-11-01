//! L'assistance sur toute la plateforme (02/11). Gardée par la place de la
//! personne : l'équipe (`programme.support.answer`), ou une organisation qui
//! suit un dossier déposé. Les sujets d'un dossier s'écrivent par les routes du
//! dossier ; ici, la liste de tout, la lecture, et les sujets sans dossier.

use actix_web::{web, HttpRequest, HttpResponse};
use kernel::auth::Actor;
use kernel::error::Result;
use uuid::Uuid;

use crate::routes::contexte_de;
use crate::service::assistance::{
    self, MarkAssistanceReadPayload, OpenSupportSubjectPayload, ReplySupportSubjectPayload,
};
use crate::state::ProgrammeState;

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("", web::get().to(etat))
        .route("/subjects", web::get().to(sujets))
        .route("/subjects", web::post().to(ouvrir))
        .route("/subjects/{id}/replies", web::post().to(repondre))
        .route("/read", web::post().to(marquer_lus));
}

#[utoipa::path(
    get,
    description = "`AssistanceStatus` : de quel côté la personne connectée se tient (`team` si elle détient `programme.support.answer`, `organization` si elle suit un dossier déposé, en évaluation, à corriger ou retenu, `null` sinon) et combien de messages de l'assistance elle n'a pas lus. Appelée sur chaque écran : décide si le bouton d'aide s'affiche.",
    path = "/assistance",
    tag = "Assistance",
    operation_id = "assistance_etat",
    responses(
        (status = 200, description = "AssistanceStatus", body = Object),
        (status = 401, description = "Aucune session", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn etat(state: web::Data<ProgrammeState>, acteur: Actor) -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(assistance::etat(&state, acteur.0).await?))
}

#[utoipa::path(
    get,
    description = "`AssistanceFeed` : tous les messages de l'assistance que la personne lit, du plus ancien au plus récent, avec leur état de lecture. Côté équipe : les messages partagés des dossiers de son périmètre et, en portée globale, les sujets sans dossier. Côté organisation : les messages partagés de ses dossiers, ses propres sujets sans dossier, et ses dossiers ouverts à l'assistance (`proposals`). Côté organisation, un message de l'équipe ne porte pas de nom (`author_name` nul).",
    path = "/assistance/subjects",
    tag = "Assistance",
    operation_id = "assistance_sujets",
    responses(
        (status = 200, description = "AssistanceFeed", body = Object),
        (status = 401, description = "Aucune session", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Ni équipe d'assistance, ni dossier déposé", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn sujets(
    state: web::Data<ProgrammeState>,
    acteur: Actor,
) -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(assistance::fil(&state, acteur.0).await?))
}

#[utoipa::path(
    post,
    description = "`OpenSupportSubjectPayload` → `ProposalComment`. Ouvre un sujet qui ne concerne aucun dossier : question, problème, compte, suggestion (`platform_support_topic`). Réservé à l'organisation ; un sujet sur un dossier s'ouvre par `POST /proposals/{id}/comments`.",
    path = "/assistance/subjects",
    tag = "Assistance",
    operation_id = "assistance_ouvrir",
    request_body = Object,
    responses(
        (status = 200, description = "ProposalComment", body = Object),
        (status = 403, description = "Pas de dossier déposé, ou équipe d'assistance", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Nature inconnue, titre hors de 2 à 120 caractères, message vide", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn ouvrir(
    requete: HttpRequest,
    state: web::Data<ProgrammeState>,
    acteur: Actor,
    corps: web::Json<OpenSupportSubjectPayload>,
) -> Result<HttpResponse> {
    let ctx = contexte_de(&requete, acteur.0);
    let message = assistance::ouvrir(&state, &ctx, acteur.0, corps.into_inner()).await?;
    Ok(HttpResponse::Ok().json(message))
}

#[utoipa::path(
    post,
    description = "`ReplySupportSubjectPayload` → `ProposalComment`. Répond dans un sujet sans dossier, côté organisation (son auteur) ou côté équipe (portée globale). Rouvre un sujet résolu.",
    path = "/assistance/subjects/{id}/replies",
    tag = "Assistance",
    operation_id = "assistance_repondre",
    params(("id" = Uuid, Path, description = "Message qui ouvre le sujet")),
    request_body = Object,
    responses(
        (status = 200, description = "ProposalComment", body = Object),
        (status = 404, description = "Sujet inexistant, sur un dossier, ou hors de lecture", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Message vide", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn repondre(
    requete: HttpRequest,
    state: web::Data<ProgrammeState>,
    acteur: Actor,
    chemin: web::Path<Uuid>,
    corps: web::Json<ReplySupportSubjectPayload>,
) -> Result<HttpResponse> {
    let ctx = contexte_de(&requete, acteur.0);
    let message = assistance::repondre(
        &state,
        &ctx,
        acteur.0,
        chemin.into_inner(),
        corps.into_inner(),
    )
    .await?;
    Ok(HttpResponse::Ok().json(message))
}

#[utoipa::path(
    post,
    description = "`MarkAssistanceReadPayload` → 204. Marque lus des messages de l'assistance, sur un dossier ou non ; ceux que la personne ne lit pas, ou qu'elle a écrits, sont ignorés.",
    path = "/assistance/read",
    tag = "Assistance",
    operation_id = "assistance_marquer_lus",
    request_body = Object,
    responses(
        (status = 204, description = "Lus"),
        (status = 403, description = "Pas d'assistance pour cette personne", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn marquer_lus(
    requete: HttpRequest,
    state: web::Data<ProgrammeState>,
    acteur: Actor,
    corps: web::Json<MarkAssistanceReadPayload>,
) -> Result<HttpResponse> {
    let ctx = contexte_de(&requete, acteur.0);
    assistance::marquer_lus(&state, &ctx, acteur.0, corps.into_inner()).await?;
    Ok(HttpResponse::NoContent().finish())
}
