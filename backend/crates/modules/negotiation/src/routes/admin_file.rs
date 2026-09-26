//! La file des experts : `Requires<KnowledgeReview>`, portée globale, partout.
//! Aucune réponse ne porte d'auteur (R9).

use actix_web::{web, HttpRequest, HttpResponse};
use kernel::auth::Requires;
use kernel::error::Result;
use uuid::Uuid;

use crate::domain::admin_file::{AdminFaqReportCloseInput, FiltreFile};
use crate::domain::permissions::KnowledgeReview;
use crate::domain::savoir_questions::{AdminQuestionAnswerInput, AdminQuestionPromoteInput};
use crate::service::savoir_admin::droits;
use crate::service::savoir_file as service;
use crate::service::savoir_questions;
use crate::state::NegotiationState;

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/admin/negotiation/queue", web::get().to(file))
        .route(
            "/admin/negotiation/queue/reports/{id}/close",
            web::post().to(clore_un_signalement),
        )
        .route(
            "/admin/negotiation/queue/questions/{id}/answer",
            web::post().to(repondre_a_une_question),
        )
        .route(
            "/admin/negotiation/queue/questions/{id}/promote",
            web::post().to(promouvoir_une_question),
        );
}

#[utoipa::path(
    get,
    description = "`ExpertQueue` — par sorte. `reports` : les signalements ouverts, groupés par entrée de FAQ, le plus ancien d'abord, avec les retours « Oui / Non » comptés. `questions` : les questions en attente, la plus ancienne d'abord, puis celles répondues depuis trente jours et pas encore promues. Et ce qui attend, par sorte. **Aucun auteur.**",
    path = "/admin/negotiation/queue",
    tag = "Back-office — file des experts",
    operation_id = "admin_negotiation_file",
    params(("kind" = Option<String>, Query, description = "`reports` (défaut) ou `questions`")),
    responses(
        (status = 200, description = "ExpertQueue", body = Object),
        (status = 401, description = "Aucune session", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Sans la permission de vérifier", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Sorte inconnue", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn file(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    _expert: Requires<KnowledgeReview>,
    filtre: web::Query<FiltreFile>,
) -> Result<HttpResponse> {
    let locale = crate::routes::locale_de(&requete);
    let file = service::file(&state, filtre.kind.as_deref(), &locale).await?;
    Ok(HttpResponse::Ok().json(file))
}

#[utoipa::path(
    post,
    description = "`AdminFaqReportCloseInput` → `AdminFaqReport` — clôt le signalement avec son issue : `revised`, `confirmed` ou `dismissed`. **L'entrée n'est jamais modifiée** : la corriger ou la mettre « À revoir » passe par sa fiche. Déjà clos : **409**.",
    path = "/admin/negotiation/queue/reports/{id}/close",
    tag = "Back-office — file des experts",
    operation_id = "admin_negotiation_file_clore_un_signalement",
    params(("id" = Uuid, Path, description = "Identifiant du signalement")),
    request_body = Object,
    responses(
        (status = 200, description = "AdminFaqReport", body = Object),
        (status = 403, description = "Sans la permission de vérifier", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Signalement inconnu", body = crate::routes::openapi::ApiErrorBody),
        (status = 409, description = "Déjà clos", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Issue inconnue", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn clore_un_signalement(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    expert: Requires<KnowledgeReview>,
    chemin: web::Path<Uuid>,
    entree: web::Json<AdminFaqReportCloseInput>,
) -> Result<HttpResponse> {
    let ctx = crate::routes::contexte_de(&requete, expert.person_id);
    let clos =
        service::clore_un_signalement(&state, &ctx, expert.person_id, chemin.into_inner(), &entree)
            .await?;
    Ok(HttpResponse::Ok().json(clos))
}

#[utoipa::path(
    post,
    description = "`AdminQuestionAnswerInput` → `AdminQuestion` — répond à une question en attente. Met en file, dans la même transaction, le courriel qui prévient son auteure ; la réponse paraît dans « Mes questions ». Déjà répondue : **409**.",
    path = "/admin/negotiation/queue/questions/{id}/answer",
    tag = "Back-office — file des experts",
    operation_id = "admin_negotiation_file_repondre_a_une_question",
    params(("id" = Uuid, Path, description = "Identifiant de la question")),
    request_body = Object,
    responses(
        (status = 200, description = "AdminQuestion", body = Object),
        (status = 403, description = "Sans la permission de vérifier", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Question inconnue", body = crate::routes::openapi::ApiErrorBody),
        (status = 409, description = "Déjà répondue", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Réponse vide", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn repondre_a_une_question(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    expert: Requires<KnowledgeReview>,
    chemin: web::Path<Uuid>,
    entree: web::Json<AdminQuestionAnswerInput>,
) -> Result<HttpResponse> {
    let locale = crate::routes::locale_de(&requete);
    let ctx = crate::routes::contexte_de(&requete, expert.person_id);
    let question = savoir_questions::repondre(
        &state,
        &ctx,
        expert.person_id,
        chemin.into_inner(),
        &entree,
        &locale,
    )
    .await?;
    Ok(HttpResponse::Ok().json(question))
}

#[utoipa::path(
    post,
    description = "`AdminQuestionPromoteInput` → `AdminFaqEntry` — fait d'une question répondue un brouillon de FAQ dans la rubrique choisie : sa question et sa réponse, **sans auteur**, reliées par `origin_question_id` ; la question passe `added_to_faq`. L'expert réécrit le brouillon avant publication. Sans consentement : `NEGOTIATION_QUESTION_NO_CONSENT` ; pas encore répondue : **422** ; déjà promue : **409**.",
    path = "/admin/negotiation/queue/questions/{id}/promote",
    tag = "Back-office — file des experts",
    operation_id = "admin_negotiation_file_promouvoir_une_question",
    params(("id" = Uuid, Path, description = "Identifiant de la question")),
    request_body = Object,
    responses(
        (status = 200, description = "AdminFaqEntry", body = Object),
        (status = 403, description = "Sans la permission de vérifier", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Question inconnue", body = crate::routes::openapi::ApiErrorBody),
        (status = 409, description = "Déjà promue", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Sans consentement, pas encore répondue, ou rubrique inconnue", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn promouvoir_une_question(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    expert: Requires<KnowledgeReview>,
    chemin: web::Path<Uuid>,
    entree: web::Json<AdminQuestionPromoteInput>,
) -> Result<HttpResponse> {
    let locale = crate::routes::locale_de(&requete);
    let ctx = crate::routes::contexte_de(&requete, expert.person_id);
    let droits = droits(&state, expert.person_id).await?;
    let entree =
        savoir_questions::promouvoir(&state, &ctx, &droits, chemin.into_inner(), &entree, &locale)
            .await?;
    Ok(HttpResponse::Ok().json(entree))
}
