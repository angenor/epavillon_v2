//! Les sessions officielles d'une édition — lecture publique (FR-042).

use actix_web::http::header::{CACHE_CONTROL, ETAG};
use actix_web::{web, HttpRequest, HttpResponse};
use kernel::error::Result;
use serde::Deserialize;

use crate::service::sessions as service;
use crate::state::NegotiationState;

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/negotiation/sessions", web::get().to(sessions));
}

/// Publique, mais changeante d'une lecture à l'autre : revalidée à chaque fois.
const REVALIDER: &str = "public, no-cache";

#[derive(Debug, Deserialize)]
pub struct Edition {
    edition: String,
}

#[utoipa::path(
    get,
    description = "`OfficialSessions` — toutes les sessions de négociation officielles de l'édition, en une réponse : chaque fiche se lit hors connexion dès la première lecture.\n\n`state` dit si l'import sert : **coupé** (`cut`), `sessions` est vide quel que soit l'âge des lignes en base, et `cut_reason` dit pourquoi — `disabled` (import éteint ou jamais lu) ou `unreachable` (source muette au-delà du seuil). `official_programme_url` renvoie alors au programme officiel.\n\nChaque session porte son titre anglais, qui fait foi, et sa traduction automatique quand elle existe ; `previous` n'existe que si l'heure ou la salle a changé ; `read_at` est la dernière lecture où elle figurait.\n\n`ETag` sur le corps, heure du serveur exclue ; **304** sur `If-None-Match`.",
    path = "/negotiation/sessions",
    tag = "Guide Négo — sessions officielles",
    operation_id = "negotiation_sessions",
    params(("edition" = String, Query, description = "Slug de l'édition")),
    responses(
        (status = 200, description = "OfficialSessions", body = Object),
        (status = 304, description = "Rien n'a changé depuis l'empreinte présentée"),
        (status = 404, description = "Édition inconnue, ou sans import", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn sessions(
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
