//! Les routes publiques des documents de Guide Négo. Lecture ouverte, compte
//! facultatif : ce que voit une personne dépend de son accès négociateur.
//! Chaque lecture listée porte son empreinte et répond `304`.

use actix_web::http::header::{
    ACCEPT_RANGES, CACHE_CONTROL, CONTENT_DISPOSITION, CONTENT_ENCODING, CONTENT_RANGE,
    CONTENT_TYPE, ETAG, RANGE,
};
use actix_web::http::StatusCode;
use actix_web::{web, HttpMessage, HttpRequest, HttpResponse, ResponseError};
use kernel::auth::Actor;
use kernel::context::RequestContext;
use kernel::error::{ApiError, ErrorCode, Result};
use serde::Deserialize;
use uuid::Uuid;

use crate::domain::documents::serialiser_la_lecture;
use crate::domain::plage::Plage;
use crate::service::documents as service;
use crate::state::NegotiationState;

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/negotiation/documents", web::get().to(bibliotheque))
        .route("/negotiation/documents/corrections", web::get().to(notes))
        .route(
            "/negotiation/documents/{id}/reading",
            web::get().to(lecture),
        )
        .route("/negotiation/documents/{id}/file", web::get().to(fichier))
        .route("/negotiation/documents/{id}/file", web::head().to(fichier))
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
    description = "`DocumentReading` — la forme lisible entière : pages, sommaire, `has_text` et `large_text`. Son empreinte est figée tant que le fichier, son extraction et le choix « Texte agrandi » ne changent pas ; les notes n'y sont pas. Réservé sans accès : **403**. Lien externe : **409**.",
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
        .content_type("application/json")
        .body(serialiser_la_lecture(&lecture)))
}

#[utoipa::path(
    get,
    description = "Le PDF d'un document fichier publié, **entier ou par plage** : `Range: bytes=a-b`, `a-` ou `-n` rend **206** et `Content-Range` ; sans `Range`, **200**, le fichier entier (le téléchargement de la copie). Une plage hors du fichier : **416**. `HEAD` rend les mêmes en-têtes sans corps.\n\n**L'accès se vérifie à chaque requête**, morceau compris. Jamais compressé (`Content-Encoding: identity`, `no-transform`) : une réponse partielle compressée fait renoncer le lecteur aux plages. `ETag` fort, **304** sur `If-None-Match`. Réservé : `no-store`.",
    path = "/negotiation/documents/{id}/file",
    tag = "Guide Négo — documents",
    operation_id = "negotiation_document_fichier",
    params(
        ("id" = Uuid, Path, description = "Identifiant du document"),
        ("Range" = Option<String>, Header, description = "Une plage d'octets : bytes=a-b, bytes=a- ou bytes=-n"),
    ),
    responses(
        (status = 200, description = "application/pdf, le fichier entier", content_type = "application/pdf"),
        (status = 206, description = "application/pdf, la plage demandée", content_type = "application/pdf"),
        (status = 304, description = "Rien n'a changé"),
        (status = 403, description = "Document réservé", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Document inconnu ou non publié", body = crate::routes::openapi::ApiErrorBody),
        (status = 409, description = "Un lien ne se lit pas dans l'application", body = crate::routes::openapi::ApiErrorBody),
        (status = 416, description = "La plage demandée est hors du fichier", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn fichier(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let entete_plage = crate::routes::entete(&requete, RANGE.as_str());
    let avec_corps = requete.method() != actix_web::http::Method::HEAD;
    let servi = service::lire_le_fichier(
        &state,
        personne(&requete),
        chemin.into_inner(),
        entete_plage.as_deref(),
        avec_corps,
    )
    .await?;
    let cache = if servi.reserve {
        "private, no-store, no-transform"
    } else {
        "private, max-age=3600, no-transform"
    };
    if let Some(r) = inchange(&requete, &servi.empreinte, cache) {
        return Ok(r);
    }

    let (statut, longueur, bornes) = match servi.plage {
        Plage::HorsDuFichier => {
            let mut refus =
                ApiError::new(ErrorCode::NegotiationDocumentRangeInvalid).error_response();
            poser(
                &mut refus,
                CONTENT_RANGE,
                &format!("bytes */{}", servi.taille),
            );
            poser(&mut refus, ACCEPT_RANGES, "bytes");
            poser(&mut refus, CACHE_CONTROL, cache);
            poser(&mut refus, CONTENT_ENCODING, "identity");
            return Ok(refus);
        }
        Plage::Entier => (StatusCode::OK, servi.taille, None),
        Plage::Partie { debut, fin } => (
            StatusCode::PARTIAL_CONTENT,
            fin - debut + 1,
            Some(format!("bytes {debut}-{fin}/{}", servi.taille)),
        ),
    };
    let mut reponse = HttpResponse::build(statut);
    reponse
        .insert_header((CONTENT_TYPE, "application/pdf"))
        .insert_header((ACCEPT_RANGES, "bytes"))
        .insert_header((ETAG, servi.empreinte))
        .insert_header((CONTENT_DISPOSITION, "inline"))
        .insert_header((CONTENT_ENCODING, "identity"))
        .insert_header((CACHE_CONTROL, cache));
    if let Some(bornes) = bornes {
        reponse.insert_header((CONTENT_RANGE, bornes));
    }
    if avec_corps {
        return Ok(reponse.body(servi.octets));
    }
    // Un HEAD annonce la longueur qu'aurait le corps : actix l'écrit d'après le corps, qu'il
    // n'envoie pas. Une longueur posée à la main serait remplacée par zéro.
    Ok(reponse.body(CorpsAnnonce(longueur)))
}

/// Un corps qui dit sa taille et ne contient rien : ce que rend un `HEAD`.
struct CorpsAnnonce(u64);

impl actix_web::body::MessageBody for CorpsAnnonce {
    type Error = std::convert::Infallible;

    fn size(&self) -> actix_web::body::BodySize {
        actix_web::body::BodySize::Sized(self.0)
    }

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Result<actix_web::web::Bytes, Self::Error>>> {
        std::task::Poll::Ready(None)
    }
}

fn poser(reponse: &mut HttpResponse, nom: actix_web::http::header::HeaderName, valeur: &str) {
    if let Ok(valeur) = actix_web::http::header::HeaderValue::from_str(valeur) {
        reponse.headers_mut().insert(nom, valeur);
    }
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
