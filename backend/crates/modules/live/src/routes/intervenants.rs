//! Le lien Zoom d'un intervenant en ligne (08/11), à l'écran : après son
//! inscription par le lien de l'activité, ou par la régie. Routes plates.

use actix_web::{web, HttpRequest, HttpResponse};
use kernel::auth::Perimeter;
use kernel::error::{ApiError, Result};
use serde::Deserialize;
use uuid::Uuid;

use crate::service::intervenants;
use crate::state::LiveState;

const EN_TETE_LIEN: &str = "x-speaker-link";

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/speaker-links/zoom", web::post().to(lien_par_le_jeton))
        .route(
            "/admin/sessions/{session_id}/speakers/{speaker_id}/zoom",
            web::get().to(lien_pour_la_regie),
        );
}

#[derive(Debug, Deserialize)]
pub struct IntervenantDemande {
    speaker_id: Uuid,
}

#[utoipa::path(
    post,
    description = "`SpeakerZoomAccess` : le lien Zoom personnel d'un intervenant qui vient de s'inscrire par le lien de l'activité (en-tête `X-Speaker-Link`, corps `{ speaker_id }`). Il ouvre l'application Zoom. Réponse `Cache-Control: private, no-store`.\n\nRefus : `LIVE_MEETING_NOT_READY` (409, place pas encore enregistrée chez Zoom : réessayer), `LIVE_ZOOM_UNAVAILABLE` (503), 404 pour un lien plus valable ou un intervenant d'une autre activité.",
    path = "/speaker-links/zoom",
    tag = "Direct",
    operation_id = "lien_intervenant_zoom",
    params(("X-Speaker-Link" = String, Header, description = "Jeton du lien d'inscription")),
    request_body = Object,
    responses(
        (status = 200, description = "SpeakerZoomAccess", body = Object),
        (status = 401, description = "Aucun jeton", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Lien ou intervenant inconnu", body = crate::routes::openapi::ApiErrorBody),
        (status = 409, description = "Place pas encore enregistrée chez Zoom", body = crate::routes::openapi::ApiErrorBody),
        (status = 503, description = "Zoom non configuré", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn lien_par_le_jeton(
    requete: HttpRequest,
    state: web::Data<LiveState>,
    corps: web::Json<IntervenantDemande>,
) -> Result<HttpResponse> {
    let jeton = requete
        .headers()
        .get(EN_TETE_LIEN)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|j| !j.is_empty() && j.len() <= 128)
        .ok_or_else(ApiError::unauthenticated)?;
    let acces = intervenants::lien_par_le_jeton(&state, jeton, corps.speaker_id).await?;
    Ok(HttpResponse::Ok()
        .insert_header(("Cache-Control", "private, no-store"))
        .json(acces))
}

#[utoipa::path(
    get,
    description = "`SpeakerZoomAccess` : le lien Zoom personnel d'un intervenant de la séance, pour la régie qui vient de l'inscrire. Exige `live.meeting.manage`. Réponse `Cache-Control: private, no-store`.\n\nRefus : `LIVE_MEETING_NOT_READY` (409 : réessayer), `LIVE_ZOOM_UNAVAILABLE` (503).",
    path = "/admin/sessions/{session_id}/speakers/{speaker_id}/zoom",
    tag = "Direct",
    operation_id = "regie_lien_zoom_intervenant",
    params(
        ("session_id" = Uuid, Path, description = "Séance"),
        ("speaker_id" = Uuid, Path, description = "Intervenant de la séance"),
    ),
    responses(
        (status = 200, description = "SpeakerZoomAccess", body = Object),
        (status = 403, description = "Permission absente", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Séance ou intervenant inconnu", body = crate::routes::openapi::ApiErrorBody),
        (status = 409, description = "Place pas encore enregistrée chez Zoom", body = crate::routes::openapi::ApiErrorBody),
        (status = 503, description = "Zoom non configuré", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn lien_pour_la_regie(
    state: web::Data<LiveState>,
    perimetre: Perimeter,
    chemin: web::Path<(Uuid, Uuid)>,
) -> Result<HttpResponse> {
    let (session_id, speaker_id) = chemin.into_inner();
    let acces =
        intervenants::lien_pour_la_regie(&state, &perimetre, session_id, speaker_id).await?;
    Ok(HttpResponse::Ok()
        .insert_header(("Cache-Control", "private, no-store"))
        .json(acces))
}
