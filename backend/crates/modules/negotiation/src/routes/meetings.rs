//! Les réunions de la Francophonie d'une édition — lecture publique.

use actix_web::http::header::{CACHE_CONTROL, ETAG};
use actix_web::{web, HttpRequest, HttpResponse};
use kernel::error::Result;
use serde::Deserialize;

use crate::service::meetings as service;
use crate::state::NegotiationState;

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/negotiation/meetings", web::get().to(reunions));
}

const REVALIDER: &str = "public, no-cache";

#[derive(Debug, Deserialize)]
pub struct Edition {
    edition: String,
}

#[utoipa::path(
    get,
    description = "`FrancophoneMeetings` — les réunions de la Francophonie publiées de l'édition (ateliers préparatoires, concertations), triées par début. Un brouillon n'y figure jamais.\n\n**Aucun lien de visioconférence** : `has_video` dit seulement qu'il existe ; le lien est servi aux inscrites par `GET /negotiation/me/meeting-registrations`. `organizer` vaut « IFDD » ou le nom de l'organisation. « Complet » : `registered_count` atteint `capacity` ; « Terminée » : `end_at` passé.\n\n`ETag` sur le corps, heures de lecture et du serveur exclues ; **304** sur `If-None-Match`.",
    path = "/negotiation/meetings",
    tag = "Guide Négo — réunions de la Francophonie",
    operation_id = "negotiation_reunions",
    params(("edition" = String, Query, description = "Slug de l'édition")),
    responses(
        (status = 200, description = "FrancophoneMeetings", body = Object),
        (status = 304, description = "Rien n'a changé depuis l'empreinte présentée"),
        (status = 404, description = "Édition inconnue", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn reunions(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    edition: web::Query<Edition>,
) -> Result<HttpResponse> {
    let reponse = service::de_ledition(&state, &edition.edition).await?;
    let empreinte = reponse.empreinte();

    if crate::routes::inchange(&requete, &empreinte) {
        return Ok(HttpResponse::NotModified()
            .insert_header((ETAG, empreinte))
            .insert_header((CACHE_CONTROL, REVALIDER))
            .finish());
    }

    Ok(HttpResponse::Ok()
        .insert_header((ETAG, empreinte))
        .insert_header((CACHE_CONTROL, REVALIDER))
        .json(reponse))
}
