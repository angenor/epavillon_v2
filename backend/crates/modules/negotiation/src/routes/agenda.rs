//! « Mon agenda » — trois routes, chacune idempotente.

use actix_web::http::header::ETAG;
use actix_web::{web, HttpRequest, HttpResponse};
use kernel::auth::Actor;
use kernel::error::Result;
use uuid::Uuid;

use crate::domain::agenda::AgendaEntryPayload;
use crate::service::agenda as service;
use crate::state::NegotiationState;

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/negotiation/me/agenda", web::get().to(mon_agenda))
        .route(
            "/negotiation/me/agenda/{session_id}",
            web::put().to(garder_une_session),
        )
        .route(
            "/negotiation/me/agenda/{session_id}",
            web::delete().to(retirer_une_session),
        );
}

#[utoipa::path(
    get,
    description = "`MyAgenda` — les sessions officielles que la personne connectée garde : identifiant, rappel, date d'ajout. Les sessions elles-mêmes se lisent dans `OfficialSessions`.\n\n`remind` est **effectif** : faux dès que la session est annulée, quel que soit ce qui est enregistré. `ETag` et **304**.",
    path = "/negotiation/me/agenda",
    tag = "Guide Négo — sessions officielles",
    operation_id = "negotiation_mon_agenda",
    responses(
        (status = 200, description = "MyAgenda", body = Object),
        (status = 304, description = "Rien n'a changé depuis l'empreinte présentée"),
        (status = 401, description = "Aucune session", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn mon_agenda(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    acteur: Actor,
) -> Result<HttpResponse> {
    let agenda = service::mon_agenda(&state, acteur.0).await?;
    let empreinte = agenda.empreinte();

    if crate::routes::inchange(&requete, &empreinte) {
        return Ok(HttpResponse::NotModified()
            .insert_header((ETAG, empreinte))
            .insert_header(crate::routes::PERSONNEL)
            .finish());
    }

    Ok(HttpResponse::Ok()
        .insert_header((ETAG, empreinte))
        .insert_header(crate::routes::PERSONNEL)
        .json(agenda))
}

#[utoipa::path(
    put,
    description = "`AgendaEntryPayload` — garder une session dans l'agenda, ou changer son rappel. **Idempotent**. Aucun refus pour chevauchement.\n\nSession inconnue ou non importée : **404**. Annulée et absente de l'agenda : **409** ; déjà dans l'agenda, le geste passe. `remind: true` sur une annulée s'enregistre désarmé.",
    path = "/negotiation/me/agenda/{session_id}",
    tag = "Guide Négo — sessions officielles",
    operation_id = "negotiation_garder_une_session",
    params(("session_id" = Uuid, Path, description = "Identifiant de la session")),
    request_body = Object,
    responses(
        (status = 204, description = "Gardée"),
        (status = 401, description = "Aucune session", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Session inconnue ou non importée", body = crate::routes::openapi::ApiErrorBody),
        (status = 409, description = "Session annulée, absente de l'agenda", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Corps malformé", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn garder_une_session(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    acteur: Actor,
    chemin: web::Path<Uuid>,
    charge: web::Json<AgendaEntryPayload>,
) -> Result<HttpResponse> {
    let ctx = crate::routes::contexte_de(&requete, acteur.0);
    service::poser(&state, &ctx, acteur.0, chemin.into_inner(), charge.remind).await?;
    Ok(HttpResponse::NoContent().finish())
}

#[utoipa::path(
    delete,
    description = "Retire une session de l'agenda. **Idempotent**, même si elle n'y était pas.",
    path = "/negotiation/me/agenda/{session_id}",
    tag = "Guide Négo — sessions officielles",
    operation_id = "negotiation_retirer_une_session",
    params(("session_id" = Uuid, Path, description = "Identifiant de la session")),
    responses(
        (status = 204, description = "Retirée"),
        (status = 401, description = "Aucune session", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn retirer_une_session(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    acteur: Actor,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let ctx = crate::routes::contexte_de(&requete, acteur.0);
    service::retirer(&state, &ctx, acteur.0, chemin.into_inner()).await?;
    Ok(HttpResponse::NoContent().finish())
}
