//! Les routes publiques du savoir : le paquet que le téléphone garde.

use actix_web::http::header::{HeaderValue, CACHE_CONTROL, ETAG, VARY};
use actix_web::{web, HttpRequest, HttpResponse};
use kernel::error::Result;
use serde::Deserialize;

use crate::domain::savoir::lire_since;
use crate::service::savoir_paquet as service;
use crate::state::NegotiationState;

const PUBLIC: &str = "public, no-cache";

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/negotiation/knowledge", web::get().to(paquet));
}

#[derive(Debug, Deserialize)]
pub struct Depuis {
    since: Option<String>,
}

#[utoipa::path(
    get,
    description = "`KnowledgeBundle` — la FAQ, le parcours et le lexique publiés (`published` et `to_review`), les deux vocabulaires et « les plus lues », dans la langue demandée. `complete: true`.\n\nAvec `since` (le `served_at` d'une lecture précédente) : les entrées changées depuis `since − 5 min`, et dans `removed` celles qui ne sont plus publiées ; `complete: false`. Parcours, vocabulaires et `most_read` reviennent entiers.\n\n`ETag` calculé par `negotiation.knowledge_fingerprint()` et la langue ; **304** sur `If-None-Match`. `Cache-Control: public, no-cache`.",
    path = "/negotiation/knowledge",
    tag = "Guide Négo — savoir",
    operation_id = "negotiation_knowledge",
    params(("since" = Option<String>, Query, description = "`served_at` d'une lecture précédente, en RFC 3339")),
    responses(
        (status = 200, description = "KnowledgeBundle", body = Object),
        (status = 304, description = "Rien n'a changé depuis l'empreinte présentée"),
        (status = 422, description = "`since` illisible", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn paquet(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    depuis: web::Query<Depuis>,
) -> Result<HttpResponse> {
    let since = depuis.since.as_deref().map(lire_since).transpose()?;
    let locale = crate::routes::locale_de(&requete);

    let empreinte = service::empreinte(&state, &locale).await?;
    if crate::routes::inchange(&requete, &empreinte) {
        return Ok(HttpResponse::NotModified()
            .insert_header((ETAG, empreinte))
            .insert_header((CACHE_CONTROL, PUBLIC))
            .insert_header((VARY, HeaderValue::from_static("Accept-Language")))
            .finish());
    }

    let (paquet, empreinte) = service::paquet(&state, &locale, since).await?;
    Ok(HttpResponse::Ok()
        .insert_header((ETAG, empreinte))
        .insert_header((CACHE_CONTROL, PUBLIC))
        .insert_header((VARY, HeaderValue::from_static("Accept-Language")))
        .json(paquet))
}
