//! Les routes du savoir : le paquet que le téléphone garde et le compte des
//! lectures, ouverts à tous, et les termes favoris de la personne connectée.

use actix_web::http::header::{HeaderValue, CACHE_CONTROL, ETAG, VARY};
use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use kernel::auth::Actor;
use kernel::context::RequestContext;
use kernel::error::Result;
use serde::Deserialize;
use uuid::Uuid;

use crate::domain::savoir::lire_since;
use crate::service::savoir_favoris;
use crate::service::savoir_lectures;
use crate::service::savoir_paquet as service;
use crate::state::NegotiationState;

const PUBLIC: &str = "public, no-cache";

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/negotiation/knowledge", web::get().to(paquet))
        .route(
            "/negotiation/faq/{id}/read",
            web::post().to(lire_une_entree),
        )
        .route("/negotiation/me/glossary-favorites", web::get().to(favoris))
        .route(
            "/negotiation/me/glossary-favorites/{entry_id}",
            web::put().to(poser_un_favori),
        )
        .route(
            "/negotiation/me/glossary-favorites/{entry_id}",
            web::delete().to(retirer_un_favori),
        );
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

#[utoipa::path(
    get,
    description = "`MyGlossaryFavorites` — les identifiants des termes favoris de la personne connectée, parmi les entrées servies (`published` et `to_review`). `ETag` et **304**.",
    path = "/negotiation/me/glossary-favorites",
    tag = "Guide Négo — savoir",
    operation_id = "negotiation_mes_termes_favoris",
    responses(
        (status = 200, description = "MyGlossaryFavorites", body = Object),
        (status = 304, description = "Rien n'a changé depuis l'empreinte présentée"),
        (status = 401, description = "Aucune session", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn favoris(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    acteur: Actor,
) -> Result<HttpResponse> {
    let (favoris, empreinte) = savoir_favoris::favoris(&state, acteur.0).await?;
    if crate::routes::inchange(&requete, &empreinte) {
        return Ok(HttpResponse::NotModified()
            .insert_header((ETAG, empreinte))
            .insert_header(crate::routes::PERSONNEL)
            .finish());
    }
    Ok(HttpResponse::Ok()
        .insert_header((ETAG, empreinte))
        .insert_header(crate::routes::PERSONNEL)
        .json(favoris))
}

#[utoipa::path(
    put,
    description = "Pose un terme favori. **Idempotent**. Entrée inconnue ou en brouillon : **404**.",
    path = "/negotiation/me/glossary-favorites/{entry_id}",
    tag = "Guide Négo — savoir",
    operation_id = "negotiation_poser_un_terme_favori",
    params(("entry_id" = Uuid, Path, description = "Identifiant de l'entrée du lexique")),
    responses(
        (status = 204, description = "Posé"),
        (status = 401, description = "Aucune session", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Entrée inconnue ou en brouillon", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn poser_un_favori(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    acteur: Actor,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let ctx = crate::routes::contexte_de(&requete, acteur.0);
    savoir_favoris::poser_un_favori(&state, &ctx, acteur.0, chemin.into_inner()).await?;
    Ok(HttpResponse::NoContent().finish())
}

#[utoipa::path(
    delete,
    description = "Retire un terme favori. **Idempotent**, même s'il n'existe pas.",
    path = "/negotiation/me/glossary-favorites/{entry_id}",
    tag = "Guide Négo — savoir",
    operation_id = "negotiation_retirer_un_terme_favori",
    params(("entry_id" = Uuid, Path, description = "Identifiant de l'entrée du lexique")),
    responses(
        (status = 204, description = "Retiré"),
        (status = 401, description = "Aucune session", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn retirer_un_favori(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    acteur: Actor,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let ctx = crate::routes::contexte_de(&requete, acteur.0);
    savoir_favoris::retirer_un_favori(&state, &ctx, acteur.0, chemin.into_inner()).await?;
    Ok(HttpResponse::NoContent().finish())
}

#[utoipa::path(
    post,
    description = "Compte une lecture de l'entrée de FAQ, pour le jour de Paris, sans rien retenir de qui lit. Le téléphone l'envoie une fois par entrée et par jour. Entrée inconnue ou en brouillon : **204** quand même, rien n'est compté.",
    path = "/negotiation/faq/{id}/read",
    tag = "Guide Négo — savoir",
    operation_id = "negotiation_lire_une_entree_de_faq",
    params(("id" = Uuid, Path, description = "Identifiant de l'entrée de FAQ")),
    responses((status = 204, description = "Lecture comptée, ou ignorée")),
)]
pub(crate) async fn lire_une_entree(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let ctx = requete
        .extensions()
        .get::<RequestContext>()
        .cloned()
        .unwrap_or_else(|| {
            RequestContext::new(
                RequestContext::generated_request_id(),
                crate::routes::locale_de(&requete),
            )
        });
    savoir_lectures::compter_une_lecture(&state, &ctx, chemin.into_inner()).await?;
    Ok(HttpResponse::NoContent().finish())
}
