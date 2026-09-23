//! Les routes publiques des documents de Guide Négo. Lecture ouverte, compte
//! facultatif : ce que voit une personne dépend de son accès négociateur.
//! Chaque lecture listée porte son empreinte et répond `304`.

use actix_web::http::header::{CACHE_CONTROL, CONTENT_TYPE, ETAG};
use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use kernel::auth::Actor;
use kernel::context::RequestContext;
use kernel::error::Result;
use serde::Deserialize;
use uuid::Uuid;

use crate::service::documents as service;
use crate::state::NegotiationState;

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/negotiation/documents", web::get().to(bibliotheque))
        .route("/negotiation/documents/corrections", web::get().to(notes))
        .route(
            "/negotiation/documents/{id}/reading",
            web::get().to(lecture),
        )
        .route(
            "/negotiation/documents/{id}/pages/{index}/image",
            web::get().to(image),
        )
        .route(
            "/negotiation/documents/{id}/downloads",
            web::post().to(telecharge),
        )
        .route("/negotiation/me/bookmarks", web::get().to(favoris))
        .route(
            "/negotiation/me/bookmarks/{document_id}",
            web::put().to(poser_un_favori),
        )
        .route(
            "/negotiation/me/bookmarks/{document_id}",
            web::delete().to(retirer_un_favori),
        );
}

/// La personne connectée, si elle l'est : ces lectures sont ouvertes à tous.
fn personne(requete: &HttpRequest) -> Option<Uuid> {
    requete
        .extensions()
        .get::<RequestContext>()
        .and_then(|c| c.actor_id)
}

fn inchange(requete: &HttpRequest, empreinte: &str, cache: &'static str) -> Option<HttpResponse> {
    crate::routes::inchange(requete, empreinte).then(|| {
        HttpResponse::NotModified()
            .insert_header((ETAG, empreinte.to_owned()))
            .insert_header((CACHE_CONTROL, cache))
            .finish()
    })
}

#[derive(Debug, Deserialize)]
pub struct Recherche {
    q: Option<String>,
}

#[utoipa::path(
    get,
    description = "`DocumentLibrary` — la liste entière des documents publiés, avec les libellés des seules valeurs citées. Un document réservé paraît à tous, mais sans résumé ni thématiques pour qui n'a pas l'accès négociateur.\n\nAvec `q` : `DocumentTextHits`, la recherche dans le texte des documents, sans tenir compte des accents ; un réservé sans accès n'y rend que son identifiant.\n\nLa liste porte un `ETag` calculé sur ce que la personne voit, et rend **304** sur `If-None-Match`.",
    path = "/negotiation/documents",
    tag = "Guide Négo — documents",
    operation_id = "negotiation_documents",
    params(("q" = Option<String>, Query, description = "Un mot du texte des documents")),
    responses(
        (status = 200, description = "DocumentLibrary, ou DocumentTextHits avec q", body = Object),
        (status = 304, description = "Rien n'a changé depuis l'empreinte présentée"),
        (status = 422, description = "Recherche vide", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn bibliotheque(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    recherche: web::Query<Recherche>,
) -> Result<HttpResponse> {
    let qui = personne(&requete);
    if let Some(q) = &recherche.q {
        let trouvailles = service::rechercher(&state, qui, q).await?;
        return Ok(HttpResponse::Ok()
            .insert_header(crate::routes::PERSONNEL)
            .json(trouvailles));
    }
    let locale = crate::routes::locale_de(&requete);
    let (bibliotheque, empreinte) = service::bibliotheque(&state, qui, &locale).await?;
    if let Some(r) = inchange(&requete, &empreinte, "private, no-cache") {
        return Ok(r);
    }
    Ok(HttpResponse::Ok()
        .insert_header((ETAG, empreinte))
        .insert_header(crate::routes::PERSONNEL)
        .json(bibliotheque))
}

#[utoipa::path(
    get,
    description = "`CorrectionNoteList` — les notes de correction vivantes de tous les documents publiés. Une seule lecture tient à jour toutes les copies gardées ; celles d'un document réservé ne vont qu'à qui a l'accès. `ETag` et **304**.",
    path = "/negotiation/documents/corrections",
    tag = "Guide Négo — documents",
    operation_id = "negotiation_documents_corrections",
    responses(
        (status = 200, description = "CorrectionNoteList", body = Object),
        (status = 304, description = "Rien n'a changé depuis l'empreinte présentée"),
    )
)]
pub(crate) async fn notes(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
) -> Result<HttpResponse> {
    let locale = crate::routes::locale_de(&requete);
    let (notes, empreinte) = service::notes(&state, personne(&requete), &locale).await?;
    if let Some(r) = inchange(&requete, &empreinte, "private, no-cache") {
        return Ok(r);
    }
    Ok(HttpResponse::Ok()
        .insert_header((ETAG, empreinte))
        .insert_header(crate::routes::PERSONNEL)
        .json(notes))
}

#[utoipa::path(
    get,
    description = "`DocumentReading` — la forme lisible entière : pages, sommaire, mode. Son empreinte est figée tant que le fichier, son extraction et le mode ne changent pas ; les notes n'y sont pas. Réservé sans accès : **403**. Lien externe : **409**.",
    path = "/negotiation/documents/{id}/reading",
    tag = "Guide Négo — documents",
    operation_id = "negotiation_document_lecture",
    params(("id" = Uuid, Path, description = "Identifiant du document")),
    responses(
        (status = 200, description = "DocumentReading", body = Object),
        (status = 304, description = "La copie gardée est la bonne"),
        (status = 403, description = "Document réservé", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Document inconnu ou non publié", body = crate::routes::openapi::ApiErrorBody),
        (status = 409, description = "Un lien ne se lit pas dans l'application", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn lecture(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let (lecture, empreinte, reserve) =
        service::lecture(&state, personne(&requete), chemin.into_inner()).await?;
    let cache = if reserve {
        "private, no-cache"
    } else {
        "public, no-cache"
    };
    if let Some(r) = inchange(&requete, &empreinte, cache) {
        return Ok(r);
    }
    Ok(HttpResponse::Ok()
        .insert_header((ETAG, empreinte))
        .insert_header((CACHE_CONTROL, cache))
        .json(lecture))
}

#[utoipa::path(
    get,
    description = "L'image JPEG d'une page, lue dans le bucket privé et servie par l'API après vérification de l'accès. Figée : son empreinte ne change qu'avec le fichier.",
    path = "/negotiation/documents/{id}/pages/{index}/image",
    tag = "Guide Négo — documents",
    operation_id = "negotiation_document_image",
    params(
        ("id" = Uuid, Path, description = "Identifiant du document"),
        ("index" = i32, Path, description = "Page du document, à partir de 1"),
    ),
    responses(
        (status = 200, description = "image/jpeg", content_type = "image/jpeg"),
        (status = 304, description = "Rien n'a changé"),
        (status = 403, description = "Document réservé", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Document ou page inconnus", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn image(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    chemin: web::Path<(Uuid, i32)>,
) -> Result<HttpResponse> {
    let (id, index) = chemin.into_inner();
    let image = service::image(&state, personne(&requete), id, index).await?;
    Ok(servir_image(&requete, image))
}

/// Une image de page, avec son empreinte et son cache : privé pour un réservé.
pub(crate) fn servir_image(requete: &HttpRequest, image: service::ImageDePage) -> HttpResponse {
    let cache = if image.reservee {
        "private, max-age=604800"
    } else {
        "public, max-age=604800"
    };
    if let Some(r) = inchange(requete, &image.empreinte, cache) {
        return r;
    }
    HttpResponse::Ok()
        .insert_header((ETAG, image.empreinte))
        .insert_header((CACHE_CONTROL, cache))
        .insert_header((CONTENT_TYPE, "image/jpeg"))
        .body(image.octets)
}

#[utoipa::path(
    post,
    description = "Compte un téléchargement réussi. **Aucun compte exigé**, et rien de la personne n'est gardé. Réservé sans accès : **403**.",
    path = "/negotiation/documents/{id}/downloads",
    tag = "Guide Négo — documents",
    operation_id = "negotiation_document_telecharge",
    params(("id" = Uuid, Path, description = "Identifiant du document")),
    responses(
        (status = 204, description = "Compté"),
        (status = 403, description = "Document réservé", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Document inconnu ou non publié", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn telecharge(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    service::compter_un_telechargement(&state, personne(&requete), chemin.into_inner()).await?;
    Ok(HttpResponse::NoContent().finish())
}

#[utoipa::path(
    get,
    description = "`DocumentBookmarkList` — les favoris de la personne connectée : identifiants et dates. `ETag` et **304**.",
    path = "/negotiation/me/bookmarks",
    tag = "Guide Négo — documents",
    operation_id = "negotiation_mes_favoris",
    responses(
        (status = 200, description = "DocumentBookmarkList", body = Object),
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
    let (favoris, empreinte) = service::favoris(&state, acteur.0).await?;
    if let Some(r) = inchange(&requete, &empreinte, "private, no-cache") {
        return Ok(r);
    }
    Ok(HttpResponse::Ok()
        .insert_header((ETAG, empreinte))
        .insert_header(crate::routes::PERSONNEL)
        .json(favoris))
}

#[utoipa::path(
    put,
    description = "Pose un favori. **Idempotent** : poser deux fois ne crée rien de plus. Document inconnu ou non publié : **404**.",
    path = "/negotiation/me/bookmarks/{document_id}",
    tag = "Guide Négo — documents",
    operation_id = "negotiation_poser_un_favori",
    params(("document_id" = Uuid, Path, description = "Identifiant du document")),
    responses(
        (status = 204, description = "Posé"),
        (status = 401, description = "Aucune session", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Document inconnu ou non publié", body = crate::routes::openapi::ApiErrorBody),
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
    service::poser_un_favori(&state, &ctx, acteur.0, chemin.into_inner()).await?;
    Ok(HttpResponse::NoContent().finish())
}

#[utoipa::path(
    delete,
    description = "Retire un favori. **Idempotent**, même s'il n'existe pas.",
    path = "/negotiation/me/bookmarks/{document_id}",
    tag = "Guide Négo — documents",
    operation_id = "negotiation_retirer_un_favori",
    params(("document_id" = Uuid, Path, description = "Identifiant du document")),
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
    service::retirer_un_favori(&state, &ctx, acteur.0, chemin.into_inner()).await?;
    Ok(HttpResponse::NoContent().finish())
}
