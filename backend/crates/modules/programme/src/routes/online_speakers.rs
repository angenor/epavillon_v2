//! Les intervenants en ligne (08/11) : le lien d'inscription, sans compte
//! (jeton dans l'en-tête `X-Speaker-Link`, jamais dans l'adresse), et les
//! gestes de la régie du jour sous `/admin/regie/sessions/{id}/speakers`.

use actix_web::{web, HttpRequest, HttpResponse};
use kernel::auth::Perimeter;
use kernel::context::RequestContext;
use kernel::error::{ApiError, Result};
use uuid::Uuid;

use crate::domain::online_speakers::{OnlineSpeakerInput, EN_TETE_LIEN};
use crate::routes::{contexte_de, locale_de};
use crate::service::online_speakers as service;
use crate::state::ProgrammeState;

/// Chemins publics, plats : le module Live dépose `/speaker-links/zoom` à côté.
pub fn publics(cfg: &mut web::ServiceConfig) {
    cfg.route("/speaker-links", web::get().to(invitation))
        .route("/speaker-links/speakers", web::post().to(sinscrire));
}

/// Sous `/admin/regie`.
pub fn regie(cfg: &mut web::ServiceConfig) {
    cfg.route("/sessions/{sessionId}/speakers", web::get().to(liste))
        .route("/sessions/{sessionId}/speakers", web::post().to(inscrire))
        .route(
            "/sessions/{sessionId}/speakers/zoom-links",
            web::post().to(renvoyer_les_liens),
        )
        .route(
            "/sessions/{sessionId}/speakers/owner-link",
            web::post().to(envoyer_au_porteur),
        )
        .route(
            "/sessions/{sessionId}/speakers/{speakerId}/list",
            web::post().to(afficher),
        );
}

fn jeton(requete: &HttpRequest) -> Result<String> {
    requete
        .headers()
        .get(EN_TETE_LIEN)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|j| !j.is_empty() && j.len() <= 128)
        .map(str::to_owned)
        .ok_or_else(ApiError::unauthenticated)
}

#[utoipa::path(
    get,
    description = "`SpeakerLinkInvite` : l'activité du lien d'inscription des intervenants en ligne (en-tête `X-Speaker-Link`). **404** pour un jeton inconnu, une activité finie, annulée ou non diffusée. Réponse `Cache-Control: private, no-store`.",
    path = "/speaker-links",
    tag = "Intervenants en ligne",
    operation_id = "lien_intervenant_invitation",
    params(("X-Speaker-Link" = String, Header, description = "Jeton du lien d'inscription")),
    responses(
        (status = 200, description = "SpeakerLinkInvite", body = Object),
        (status = 401, description = "Aucun jeton", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Lien inconnu ou plus valable", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn invitation(
    requete: HttpRequest,
    state: web::Data<ProgrammeState>,
) -> Result<HttpResponse> {
    let invitation = service::invitation(&state, &jeton(&requete)?).await?;
    Ok(HttpResponse::Ok()
        .insert_header(("Cache-Control", "private, no-store"))
        .json(invitation))
}

#[utoipa::path(
    post,
    description = "`OnlineSpeakerInput` → `OnlineSpeakerCreated`. Inscription d'un intervenant en ligne par le lien de l'activité (en-tête `X-Speaker-Link`) : il devient intervenant, **masqué** sur la page jusqu'à la validation de la régie, et son lien Zoom personnel part par courriel dès que sa place existe chez Zoom ; `POST /speaker-links/zoom` le rend à l'écran. Une adresse déjà intervenante de l'activité reprend sa ligne. `consent` obligatoire.",
    path = "/speaker-links/speakers",
    tag = "Intervenants en ligne",
    operation_id = "lien_intervenant_inscription",
    params(("X-Speaker-Link" = String, Header, description = "Jeton du lien d'inscription")),
    request_body = Object,
    responses(
        (status = 200, description = "OnlineSpeakerCreated", body = Object),
        (status = 401, description = "Aucun jeton", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Lien inconnu ou plus valable", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Champ manquant ou invalide, accord absent", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn sinscrire(
    requete: HttpRequest,
    state: web::Data<ProgrammeState>,
    charge: web::Json<OnlineSpeakerInput>,
) -> Result<HttpResponse> {
    let ctx = RequestContext::new(RequestContext::generated_request_id(), locale_de(&requete));
    let cree = service::sinscrire_par_le_lien(&state, &ctx, &jeton(&requete)?, charge.into_inner())
        .await?;
    Ok(HttpResponse::Ok().json(cree))
}

#[utoipa::path(
    get,
    description = "`RegieSpeakers` : les intervenants d'une activité, leur adresse, leur origine (`dossier`, `regie`, `lien`), leur affichage sur la page et l'envoi de leur lien Zoom ; le lien d'inscription et les adresses du porteur.",
    path = "/admin/regie/sessions/{sessionId}/speakers",
    tag = "Régie du jour",
    operation_id = "regie_intervenants",
    params(("sessionId" = Uuid, Path, description = "Séance")),
    responses(
        (status = 200, description = "RegieSpeakers", body = Object),
        (status = 403, description = "Permission de planifier absente", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Séance inexistante ou hors périmètre", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn liste(
    state: web::Data<ProgrammeState>,
    perimetre: Perimeter,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let liste = service::liste(&state, &perimetre, chemin.into_inner()).await?;
    Ok(HttpResponse::Ok().json(liste))
}

#[utoipa::path(
    post,
    description = "`OnlineSpeakerInput` → `OnlineSpeakerCreated`. La régie inscrit un intervenant en ligne : affiché d'office sur la page, son lien Zoom part aussitôt ; `GET /admin/sessions/{sessionId}/speakers/{speakerId}/zoom` le rend à l'écran.",
    path = "/admin/regie/sessions/{sessionId}/speakers",
    tag = "Régie du jour",
    operation_id = "regie_inscrire_un_intervenant",
    params(("sessionId" = Uuid, Path, description = "Séance")),
    request_body = Object,
    responses(
        (status = 200, description = "OnlineSpeakerCreated", body = Object),
        (status = 403, description = "Permission de planifier absente", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Séance inexistante ou hors périmètre", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Champ manquant ou invalide, activité annulée ou non diffusée", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn inscrire(
    requete: HttpRequest,
    state: web::Data<ProgrammeState>,
    perimetre: Perimeter,
    chemin: web::Path<Uuid>,
    charge: web::Json<OnlineSpeakerInput>,
) -> Result<HttpResponse> {
    let ctx = contexte_de(&requete, perimetre.person_id);
    let cree = service::inscrire_par_la_regie(
        &state,
        &ctx,
        &perimetre,
        chemin.into_inner(),
        charge.into_inner(),
    )
    .await?;
    Ok(HttpResponse::Ok().json(cree))
}

#[utoipa::path(
    post,
    description = "`RegieSpeakers` : « Afficher sur la page » pour un intervenant inscrit par le lien.",
    path = "/admin/regie/sessions/{sessionId}/speakers/{speakerId}/list",
    tag = "Régie du jour",
    operation_id = "regie_afficher_un_intervenant",
    params(
        ("sessionId" = Uuid, Path, description = "Séance"),
        ("speakerId" = Uuid, Path, description = "Intervenant de la séance"),
    ),
    responses(
        (status = 200, description = "RegieSpeakers", body = Object),
        (status = 403, description = "Permission de planifier absente", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Séance ou intervenant inconnu", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn afficher(
    requete: HttpRequest,
    state: web::Data<ProgrammeState>,
    perimetre: Perimeter,
    chemin: web::Path<(Uuid, Uuid)>,
) -> Result<HttpResponse> {
    let (session_id, speaker_id) = chemin.into_inner();
    let ctx = contexte_de(&requete, perimetre.person_id);
    let liste = service::afficher(&state, &ctx, &perimetre, session_id, speaker_id).await?;
    Ok(HttpResponse::Ok().json(liste))
}

#[utoipa::path(
    post,
    description = "`SpeakerLinksSent` : renvoie à chaque intervenant qui a une adresse son lien Zoom personnel (place créée chez Zoom s'il le faut), copie aux adresses du réglage `engagement.speaker_links.cc`.",
    path = "/admin/regie/sessions/{sessionId}/speakers/zoom-links",
    tag = "Régie du jour",
    operation_id = "regie_renvoyer_les_liens_zoom",
    params(("sessionId" = Uuid, Path, description = "Séance")),
    responses(
        (status = 200, description = "SpeakerLinksSent", body = Object),
        (status = 403, description = "Permission de planifier absente", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Séance inexistante ou hors périmètre", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Aucun intervenant joignable, activité annulée ou non diffusée", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn renvoyer_les_liens(
    requete: HttpRequest,
    state: web::Data<ProgrammeState>,
    perimetre: Perimeter,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let ctx = contexte_de(&requete, perimetre.person_id);
    let envoi = service::renvoyer_les_liens(&state, &ctx, &perimetre, chemin.into_inner()).await?;
    Ok(HttpResponse::Ok().json(envoi))
}

#[utoipa::path(
    post,
    description = "`SpeakerLinksSent` : envoie au porteur (déposant et personne à contacter de chaque dossier) le lien d'inscription des intervenants en ligne, copie aux adresses du réglage `engagement.speaker_links.cc`.",
    path = "/admin/regie/sessions/{sessionId}/speakers/owner-link",
    tag = "Régie du jour",
    operation_id = "regie_envoyer_le_lien_au_porteur",
    params(("sessionId" = Uuid, Path, description = "Séance")),
    responses(
        (status = 200, description = "SpeakerLinksSent", body = Object),
        (status = 403, description = "Permission de planifier absente", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Séance inexistante ou hors périmètre", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Aucun porteur, activité annulée ou non diffusée", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn envoyer_au_porteur(
    requete: HttpRequest,
    state: web::Data<ProgrammeState>,
    perimetre: Perimeter,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let ctx = contexte_de(&requete, perimetre.person_id);
    let envoi = service::envoyer_au_porteur(&state, &ctx, &perimetre, chemin.into_inner()).await?;
    Ok(HttpResponse::Ok().json(envoi))
}
