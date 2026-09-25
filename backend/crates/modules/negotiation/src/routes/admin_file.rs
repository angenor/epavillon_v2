//! La file des experts : `Requires<KnowledgeReview>`, portée globale, partout.
//! Aucune réponse ne porte d'auteur (R9).

use actix_web::{web, HttpRequest, HttpResponse};
use kernel::auth::Requires;
use kernel::error::Result;
use uuid::Uuid;

use crate::domain::admin_file::{AdminFaqReportCloseInput, FiltreFile};
use crate::domain::permissions::KnowledgeReview;
use crate::service::savoir_file as service;
use crate::state::NegotiationState;

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/admin/negotiation/queue", web::get().to(file))
        .route(
            "/admin/negotiation/queue/reports/{id}/close",
            web::post().to(clore_un_signalement),
        );
}

#[utoipa::path(
    get,
    description = "`ExpertQueue` — les signalements ouverts, groupés par entrée de FAQ, le plus ancien d'abord, avec les retours « Oui / Non » comptés ; et ce qui attend, par sorte. **Aucun auteur.** Seule la sorte `reports` est servie à ce jour.",
    path = "/admin/negotiation/queue",
    tag = "Back-office — file des experts",
    operation_id = "admin_negotiation_file",
    params(("kind" = Option<String>, Query, description = "`reports` (défaut)")),
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
