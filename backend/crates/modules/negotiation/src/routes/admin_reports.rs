//! La validation des signalements. Garde `Requires<ReportValidate>` **sur la
//! portée globale**, jamais `RequiresAnyScope` (tranché le 21/09) : le rôle
//! `admin` d'une seule édition n'y entre pas.

use actix_web::{web, HttpRequest, HttpResponse};
use kernel::auth::Requires;
use kernel::error::Result;
use uuid::Uuid;

use crate::domain::admin_reports::{EditionQuery, RejectPayload};
use crate::domain::permissions::ReportValidate;
use crate::service::admin_reports as service;
use crate::state::NegotiationState;

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/admin/negotiation/reports", web::get().to(file))
        .route(
            "/admin/negotiation/reports/{id}/validate",
            web::post().to(valider),
        )
        .route(
            "/admin/negotiation/reports/{id}/undo",
            web::post().to(annuler),
        )
        .route(
            "/admin/negotiation/reports/{id}/reject",
            web::post().to(refuser),
        )
        .route(
            "/admin/negotiation/reports/{id}/withdraw",
            web::post().to(retirer),
        );
}

#[utoipa::path(
    get,
    description = "`ReportQueue` — les signalements à traiter de l'édition, **les plus anciens d'abord**, puis ceux tranchés aujourd'hui (fuseau de la COP), le plus récent d'abord.\n\nChaque élément porte l'autrice (nom, pays), la session, et `source_now` : ce que dit la source officielle à l'instant, avec son heure de lecture. `status` est celui de la base : `validated` dès la validation, `published_at` dit si c'est affiché.",
    path = "/admin/negotiation/reports",
    tag = "Back-office — signalements",
    operation_id = "admin_negotiation_reports_file",
    params(("edition" = String, Query, description = "Slug de l'édition")),
    responses(
        (status = 200, description = "ReportQueue", body = Object),
        (status = 401, description = "Aucune session", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Sans `negotiation.report.validate` **sur la portée globale**", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Édition inconnue", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn file(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    _garde: Requires<ReportValidate>,
    edition: web::Query<EditionQuery>,
) -> Result<HttpResponse> {
    let locale = crate::routes::locale_de(&requete);
    Ok(HttpResponse::Ok().json(service::file(&state, &edition.edition, &locale).await?))
}

#[utoipa::path(
    post,
    description = "→ `ReportQueueItem` — valide le signalement : `status: 'validated'`, `published_at: null`. Écrit le décideur, l'heure et la source du moment ; pose la publication **trente secondes plus tard**, horloge de la base.\n\n**Rien n'est public avant la publication** : ni l'encart, ni la réunion non annoncée, ni « Validé » chez l'autrice, ni un avis. Déjà tranché : **409**.",
    path = "/admin/negotiation/reports/{id}/validate",
    tag = "Back-office — signalements",
    operation_id = "admin_negotiation_reports_valider",
    params(("id" = Uuid, Path, description = "Identifiant du signalement")),
    responses(
        (status = 200, description = "ReportQueueItem", body = Object),
        (status = 401, description = "Aucune session", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Sans la permission sur la portée globale", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Signalement inconnu", body = crate::routes::openapi::ApiErrorBody),
        (status = 409, description = "Déjà tranché", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn valider(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    garde: Requires<ReportValidate>,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let ctx = crate::routes::contexte_de(&requete, garde.person_id);
    let item = service::valider(&state, &ctx, garde.person_id, chemin.into_inner()).await?;
    Ok(HttpResponse::Ok().json(item))
}

#[utoipa::path(
    post,
    description = "→ `ReportQueueItem` — annule une validation **tant que rien n'est publié**, sans borne de temps : le signalement redevient à traiter, et la publication posée ne fera rien. Rejoué, rend l'état sans rien écrire.\n\nDéjà publié, refusé, ou heurtant un nouveau signalement en attente de la même autrice : **409** `NEGOTIATION_REPORT_UNDO_EXPIRED`.",
    path = "/admin/negotiation/reports/{id}/undo",
    tag = "Back-office — signalements",
    operation_id = "admin_negotiation_reports_annuler",
    params(("id" = Uuid, Path, description = "Identifiant du signalement")),
    responses(
        (status = 200, description = "ReportQueueItem", body = Object),
        (status = 401, description = "Aucune session", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Sans la permission sur la portée globale", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Signalement inconnu", body = crate::routes::openapi::ApiErrorBody),
        (status = 409, description = "Trop tard : déjà affiché", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn annuler(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    garde: Requires<ReportValidate>,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let ctx = crate::routes::contexte_de(&requete, garde.person_id);
    Ok(HttpResponse::Ok().json(service::annuler(&state, &ctx, chemin.into_inner()).await?))
}

#[utoipa::path(
    post,
    description = "`RejectPayload` → `ReportQueueItem` — « Ne pas retenir », avec l'un des trois motifs et une précision facultative (600 caractères). L'autrice voit l'un et l'autre, et reçoit un avis. Déjà tranché : **409**.",
    path = "/admin/negotiation/reports/{id}/reject",
    tag = "Back-office — signalements",
    operation_id = "admin_negotiation_reports_refuser",
    params(("id" = Uuid, Path, description = "Identifiant du signalement")),
    request_body = Object,
    responses(
        (status = 200, description = "ReportQueueItem", body = Object),
        (status = 400, description = "Motif inconnu ou précision trop longue — le champ est nommé", body = crate::routes::openapi::ApiErrorBody),
        (status = 401, description = "Aucune session", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Sans la permission sur la portée globale", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Signalement inconnu", body = crate::routes::openapi::ApiErrorBody),
        (status = 409, description = "Déjà tranché", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn refuser(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    garde: Requires<ReportValidate>,
    chemin: web::Path<Uuid>,
    charge: web::Json<RejectPayload>,
) -> Result<HttpResponse> {
    let ctx = crate::routes::contexte_de(&requete, garde.person_id);
    let item =
        service::refuser(&state, &ctx, garde.person_id, chemin.into_inner(), &charge).await?;
    Ok(HttpResponse::Ok().json(item))
}

#[utoipa::path(
    post,
    description = "→ `ReportQueueItem` — retire un encart affiché, ou la réunion non annoncée née du signalement. Idempotent. Pas encore affiché : **409** — c'est l'annulation qui convient.",
    path = "/admin/negotiation/reports/{id}/withdraw",
    tag = "Back-office — signalements",
    operation_id = "admin_negotiation_reports_retirer",
    params(("id" = Uuid, Path, description = "Identifiant du signalement")),
    responses(
        (status = 200, description = "ReportQueueItem", body = Object),
        (status = 401, description = "Aucune session", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Sans la permission sur la portée globale", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Signalement inconnu", body = crate::routes::openapi::ApiErrorBody),
        (status = 409, description = "Rien d'affiché à retirer", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn retirer(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    garde: Requires<ReportValidate>,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let ctx = crate::routes::contexte_de(&requete, garde.person_id);
    Ok(HttpResponse::Ok().json(service::retirer(&state, &ctx, chemin.into_inner()).await?))
}
