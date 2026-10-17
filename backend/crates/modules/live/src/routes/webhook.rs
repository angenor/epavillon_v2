//! `POST /webhooks/zoom` : hors session, authentifiée par la signature de Zoom,
//! montée seulement si `ZOOM_WEBHOOK_SECRET_TOKEN` est renseigné (sinon 404,
//! comme un module éteint). Zoom n'annonce aucune origine : le contrôle
//! d'origine la laisse passer (R30).

use actix_web::{web, HttpRequest, HttpResponse};
use kernel::error::Result;
use time::OffsetDateTime;

use crate::service::webhook::{self, EnTetes};
use crate::state::LiveState;

pub fn configurer(cfg: &mut web::ServiceConfig, monte: bool) {
    if monte {
        cfg.route("/webhooks/zoom", web::post().to(recevoir));
    }
}

#[utoipa::path(
    post,
    description = "`{}`, ou `{ plainToken, encryptedToken }` pour `endpoint.url_validation`. Les notifications de Zoom : début et fin de réunion, entrée et sortie d'une personne (première présence notée sur l'inscription).\n\nVérifiée par `x-zm-signature` et `x-zm-request-timestamp` (moins de cinq minutes), à temps constant. Journalisée avant traitement, traitée en file, dédoublonnée par empreinte. **Hors session.** Non montée sans `ZOOM_WEBHOOK_SECRET_TOKEN` : elle rend alors 404.",
    path = "/webhooks/zoom",
    tag = "Direct",
    operation_id = "zoom_notifications",
    request_body(content = String, content_type = "application/json", description = "Notification Zoom brute : la signature porte sur ces octets"),
    responses(
        (status = 200, description = "{}", body = Object),
        (status = 401, description = "Signature absente, fausse ou périmée", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Jeton non configuré : la route n'est pas montée", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn recevoir(
    state: web::Data<LiveState>,
    requete: HttpRequest,
    corps: web::Bytes,
) -> Result<HttpResponse> {
    let en_tete = |nom: &str| requete.headers().get(nom).and_then(|v| v.to_str().ok());
    let reponse = webhook::recevoir(
        &state,
        &EnTetes {
            signature: en_tete("x-zm-signature"),
            horodatage: en_tete("x-zm-request-timestamp"),
        },
        &corps,
        OffsetDateTime::now_utc(),
    )
    .await?;
    Ok(HttpResponse::Ok().json(reponse.unwrap_or_else(|| serde_json::json!({}))))
}
