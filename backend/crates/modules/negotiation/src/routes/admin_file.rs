//! La file des experts : `Requires<KnowledgeReview>`, portée globale, partout.
//! Aucune réponse ne porte d'auteur (R9).

use actix_web::{web, HttpRequest, HttpResponse};
use kernel::auth::Requires;
use kernel::error::Result;
use uuid::Uuid;

use crate::domain::admin_file::{AdminFaqReportCloseInput, FiltreFile};
use crate::domain::admin_savoir::AdminGlossaryInput;
use crate::domain::permissions::KnowledgeReview;
use crate::domain::savoir_propositions::AdminProposalRejectInput;
use crate::domain::savoir_questions::{AdminQuestionAnswerInput, AdminQuestionPromoteInput};
use crate::service::savoir_admin::droits;
use crate::service::savoir_file as service;
use crate::service::{savoir_propositions, savoir_questions};
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
        )
        .route(
            "/admin/negotiation/queue/proposals/{id}",
            web::get().to(une_proposition),
        )
        .route(
            "/admin/negotiation/queue/proposals/{id}/accept",
            web::post().to(accepter_une_proposition),
        )
        .route(
            "/admin/negotiation/queue/proposals/{id}/reject",
            web::post().to(rejeter_une_proposition),
        );
}

#[utoipa::path(
    get,
    description = "`ExpertQueue` — par sorte. `reports` : les signalements ouverts, groupés par entrée de FAQ, le plus ancien d'abord, avec les retours « Oui / Non » comptés. `questions` : les questions en attente, la plus ancienne d'abord, puis celles répondues depuis trente jours et pas encore promues. `proposals` : les termes proposés en attente, le plus ancien d'abord, avec le contexte de chaque auteur et les entrées proches du lexique. Et ce qui attend, par sorte. **Aucun auteur.**",
    path = "/admin/negotiation/queue",
    tag = "Back-office — file des experts",
    operation_id = "admin_negotiation_file",
    params(("kind" = Option<String>, Query, description = "`reports` (défaut), `questions` ou `proposals`")),
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

#[utoipa::path(
    get,
    description = "`AdminProposal` — un terme proposé, quel que soit son statut : le contexte de chaque auteur, le plus ancien d'abord, **sans les auteurs** ; les entrées du lexique proches (`similarity` ≥ 0,4, tous statuts, cinq au plus) ; l'entrée née, une fois acceptée ; le motif, une fois refusée.",
    path = "/admin/negotiation/queue/proposals/{id}",
    tag = "Back-office — file des experts",
    operation_id = "admin_negotiation_file_une_proposition",
    params(("id" = Uuid, Path, description = "Identifiant de la proposition")),
    responses(
        (status = 200, description = "AdminProposal", body = Object),
        (status = 403, description = "Sans la permission de vérifier", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Proposition inconnue", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn une_proposition(
    state: web::Data<NegotiationState>,
    _expert: Requires<KnowledgeReview>,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let proposition = savoir_propositions::une(&state, chemin.into_inner()).await?;
    Ok(HttpResponse::Ok().json(proposition))
}

#[utoipa::path(
    post,
    description = "`AdminGlossaryInput` → `AdminGlossaryEntry` — accepte un terme proposé : l'entrée naît **en brouillon**, avec le terme de la proposition si `term` est absent. Ses auteurs reçoivent un courriel **à sa publication**, pas avant. Déjà tranchée : **409** ; un terme qui s'écrit déjà ainsi : `NEGOTIATION_GLOSSARY_SLUG_TAKEN`.",
    path = "/admin/negotiation/queue/proposals/{id}/accept",
    tag = "Back-office — file des experts",
    operation_id = "admin_negotiation_file_accepter_une_proposition",
    params(("id" = Uuid, Path, description = "Identifiant de la proposition")),
    request_body = Object,
    responses(
        (status = 200, description = "AdminGlossaryEntry", body = Object),
        (status = 403, description = "Sans la permission de vérifier", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Proposition inconnue", body = crate::routes::openapi::ApiErrorBody),
        (status = 409, description = "Déjà tranchée, ou terme déjà au lexique", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Texte, famille, source ou lié invalides", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn accepter_une_proposition(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    expert: Requires<KnowledgeReview>,
    chemin: web::Path<Uuid>,
    entree: web::Json<AdminGlossaryInput>,
) -> Result<HttpResponse> {
    let locale = crate::routes::locale_de(&requete);
    let ctx = crate::routes::contexte_de(&requete, expert.person_id);
    let droits = droits(&state, expert.person_id).await?;
    let entree = savoir_propositions::accepter(
        &state,
        &ctx,
        &droits,
        expert.person_id,
        chemin.into_inner(),
        &entree,
        &locale,
    )
    .await?;
    Ok(HttpResponse::Ok().json(entree))
}

#[utoipa::path(
    post,
    description = "`AdminProposalRejectInput` → `AdminProposal` — refuse un terme proposé, avec son motif. Motif vide : **422** ; déjà tranchée : **409**.",
    path = "/admin/negotiation/queue/proposals/{id}/reject",
    tag = "Back-office — file des experts",
    operation_id = "admin_negotiation_file_rejeter_une_proposition",
    params(("id" = Uuid, Path, description = "Identifiant de la proposition")),
    request_body = Object,
    responses(
        (status = 200, description = "AdminProposal", body = Object),
        (status = 403, description = "Sans la permission de vérifier", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Proposition inconnue", body = crate::routes::openapi::ApiErrorBody),
        (status = 409, description = "Déjà tranchée", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Motif vide", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn rejeter_une_proposition(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    expert: Requires<KnowledgeReview>,
    chemin: web::Path<Uuid>,
    entree: web::Json<AdminProposalRejectInput>,
) -> Result<HttpResponse> {
    let ctx = crate::routes::contexte_de(&requete, expert.person_id);
    let proposition =
        savoir_propositions::rejeter(&state, &ctx, expert.person_id, chemin.into_inner(), &entree)
            .await?;
    Ok(HttpResponse::Ok().json(proposition))
}
