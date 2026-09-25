//! Signaler un changement, et relire ses signalements.

use actix_web::http::header::ETAG;
use actix_web::{web, HttpRequest, HttpResponse};
use kernel::auth::Actor;
use kernel::error::Result;
use serde::Deserialize;

use crate::domain::reports::ReportPayload;
use crate::service::reports::{self as service, Envoi};
use crate::state::NegotiationState;

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/negotiation/reports", web::post().to(signaler))
        .route("/negotiation/me/reports", web::get().to(mes_signalements));
}

#[derive(Debug, Deserialize)]
pub struct Edition {
    edition: String,
}

#[utoipa::path(
    post,
    description = "`ReportPayload` → `MyReport` — signaler un changement sur une session officielle, ou une réunion non annoncée (`reason = 'unannounced'`, sans session). Réservé à l'accès négociateur, portée globale.\n\n`time`, `venue`, `cancelled`, `other` exigent `session_id`, une session importée de l'édition ; `unannounced` exige `what` et `day` (AAAA-MM-JJ). Un champ étranger au motif est ignoré. `detail` : 600 caractères au plus.\n\n**Rejoué avec le même `client_ref`** : **200** et le même signalement, jamais une seconde ligne. Un signalement de la même personne attend déjà sur cette session : **409**.\n\nRien n'est rendu public ici : la session officielle ne change pas, et aucun avis ne part.",
    path = "/negotiation/reports",
    tag = "Guide Négo — signalements",
    operation_id = "negotiation_signaler",
    request_body = Object,
    responses(
        (status = 201, description = "MyReport", body = Object),
        (status = 200, description = "MyReport — référence déjà reçue", body = Object),
        (status = 400, description = "Champ requis manquant ou invalide — le champ est nommé", body = crate::routes::openapi::ApiErrorBody),
        (status = 401, description = "Aucune session", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Sans accès négociateur", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Édition ou session inconnue", body = crate::routes::openapi::ApiErrorBody),
        (status = 409, description = "Un signalement attend déjà sur cette session", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn signaler(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    acteur: Actor,
    charge: web::Json<ReportPayload>,
) -> Result<HttpResponse> {
    let ctx = crate::routes::contexte_de(&requete, acteur.0);
    Ok(
        match service::signaler(&state, &ctx, acteur.0, &charge).await? {
            Envoi::Cree(r) => HttpResponse::Created().json(r),
            Envoi::Rejoue(r) => HttpResponse::Ok().json(r),
        },
    )
}

#[utoipa::path(
    get,
    description = "`MyReports` — les signalements de la personne connectée sur l'édition, plus récent d'abord.\n\n`status` vaut `validated` **seulement une fois publié** ; entre la validation et la publication, il reste `submitted`. Aucun nom de décideur. `ETag` et **304**.",
    path = "/negotiation/me/reports",
    tag = "Guide Négo — signalements",
    operation_id = "negotiation_mes_signalements",
    params(("edition" = String, Query, description = "Slug de l'édition")),
    responses(
        (status = 200, description = "MyReports", body = Object),
        (status = 304, description = "Rien n'a changé depuis l'empreinte présentée"),
        (status = 401, description = "Aucune session", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Édition inconnue", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn mes_signalements(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    acteur: Actor,
    edition: web::Query<Edition>,
) -> Result<HttpResponse> {
    let liste = service::mes_signalements(&state, acteur.0, &edition.edition).await?;
    let empreinte = liste.empreinte();

    if crate::routes::inchange(&requete, &empreinte) {
        return Ok(HttpResponse::NotModified()
            .insert_header((ETAG, empreinte))
            .insert_header(crate::routes::PERSONNEL)
            .finish());
    }

    Ok(HttpResponse::Ok()
        .insert_header((ETAG, empreinte))
        .insert_header(crate::routes::PERSONNEL)
        .json(liste))
}
