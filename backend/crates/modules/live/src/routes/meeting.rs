//! Les réunions Zoom d'une séance : lecture publique, entrée d'une personne
//! inscrite, back-office. Routes plates (voir `admin.rs`).

use actix_web::http::header::{CacheControl, CacheDirective};
use actix_web::{web, HttpRequest, HttpResponse};
use kernel::auth::Perimeter;
use kernel::error::{ApiError, Result};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::meeting::MeetingSyncInput;
use crate::service::meeting;
use crate::state::LiveState;

/// Sous le scope `/sessions`, composé par l'API.
pub fn chemins_de_seance(cfg: &mut web::ServiceConfig) {
    cfg.route("/{session_id}/meeting", web::get().to(publique))
        .route("/{session_id}/meeting/access", web::get().to(acces));
}

/// La réunion d'une journée (02/11), chemins plats : `/track-meetings`
/// n'appartient qu'à ce module.
pub fn chemins_de_journee(cfg: &mut web::ServiceConfig) {
    cfg.route(
        "/track-meetings/{track_id}",
        web::get().to(publique_journee),
    )
    .route(
        "/track-meetings/{track_id}/access",
        web::get().to(acces_journee),
    );
}

pub fn chemins_admin(cfg: &mut web::ServiceConfig) {
    cfg.route(
        "/admin/sessions/{session_id}/meeting",
        web::get().to(lire_admin),
    )
    .route(
        "/admin/sessions/{session_id}/meeting",
        web::post().to(reconstruire),
    )
    .route(
        "/admin/sessions/{session_id}/meeting/push",
        web::post().to(envoyer_maintenant),
    )
    .route(
        "/admin/sessions/{session_id}/meeting/host",
        web::get().to(lien_animateur),
    );
}

#[utoipa::path(
    get,
    description = "`PublicSessionMeeting` : la salle Zoom de la séance, sans aucun secret. `opens_at` vaut le début moins trente minutes, `closes_at` la fin constatée par Zoom ou, à défaut, la fin prévue plus quatre heures (maintenant plus quinze minutes si la réunion dure encore au-delà).\n\n`opens_at` et `closes_at` sont **indicatifs** : la salle est ouverte à toute heure tant que la réunion existe chez Zoom, et jusqu'à l'annulation de la séance. Avant l'arrivée de l'animateur, Zoom fait patienter en salle d'attente.\n\n**404** quand la séance est inconnue, non publiée, ou n'a pas de réunion programmée, en cours ou terminée : les trois se lisent pareil. **Aucune garde.**",
    path = "/sessions/{session_id}/meeting",
    tag = "Direct",
    operation_id = "seance_reunion_publique",
    params(("session_id" = Uuid, Path, description = "Séance publiée")),
    responses(
        (status = 200, description = "PublicSessionMeeting", body = Object),
        (status = 404, description = "Séance inconnue, non publiée, ou sans réunion", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn publique(
    state: web::Data<LiveState>,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let reunion = meeting::publique(&state, chemin.into_inner(), OffsetDateTime::now_utc()).await?;
    Ok(HttpResponse::Ok().json(reunion))
}

#[utoipa::path(
    get,
    description = "`MeetingAccess` : de quoi entrer dans la salle avec le Meeting SDK de Zoom (web, 6.5) : numéro, code d'accès, jeton d'inscrit `tk`, signature du SDK (deux heures), lien personnel en secours.\n\nRéservé à une personne **inscrite, en liste d'attente ou déjà présente** : le jour même, Zoom laisse entrer les trois cents premiers arrivés. **Séance en entrée libre** (sans inscription) : toute personne connectée entre ; sa place chez Zoom est créée au premier appel, pendant la requête, et si Zoom ne répond pas la route dit « pas prête » et l'on réessaie. **Inscription sans compte** : le porteur du lien reçu par courriel présente son jeton (en-tête `X-Manage-Token`), qui prime sur la session ; un jeton inconnu ou remplacé rend 403. Réponse `Cache-Control: no-store`.\n\n**Aucune fenêtre horaire** : on entre à toute heure tant que la réunion existe chez Zoom, même terminée (l'animateur peut la relancer), jusqu'à l'annulation de la séance (`LIVE_MEETING_CANCELLED`, 409).\n\nRefus : `LIVE_ZOOM_UNAVAILABLE` (503, Zoom non configuré), `LIVE_MEETING_REGISTRATION_REQUIRED` (403, jamais en entrée libre),  `LIVE_MEETING_NOT_READY` (409, réunion ou inscrit pas encore enregistrés chez Zoom).",
    path = "/sessions/{session_id}/meeting/access",
    tag = "Direct",
    operation_id = "seance_reunion_acces",
    params(
        ("session_id" = Uuid, Path, description = "Séance publiée"),
        ("X-Manage-Token" = Option<String>, Header, description = "Jeton du lien reçu par courriel, inscription sans compte"),
    ),
    responses(
        (status = 200, description = "MeetingAccess", body = Object),
        (status = 401, description = "Ni session ni jeton", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "LIVE_MEETING_REGISTRATION_REQUIRED", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Séance inconnue ou non publiée", body = crate::routes::openapi::ApiErrorBody),
        (status = 409, description = "LIVE_MEETING_CANCELLED ou LIVE_MEETING_NOT_READY", body = crate::routes::openapi::ApiErrorBody),
        (status = 503, description = "LIVE_ZOOM_UNAVAILABLE", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn acces(
    state: web::Data<LiveState>,
    requete: HttpRequest,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let session_id = chemin.into_inner();
    let maintenant = OffsetDateTime::now_utc();
    let acces = if let Some(jeton) = jeton_du_lien(&requete) {
        let ctx = contexte_anonyme(&requete);
        meeting::acces_par_jeton(&state, &ctx, session_id, &jeton, maintenant).await?
    } else {
        let personne = personne_connectee(&requete).ok_or_else(ApiError::unauthenticated)?;
        let ctx = crate::routes::contexte_de(&requete, personne);
        meeting::acces(&state, &ctx, personne, session_id, maintenant).await?
    };
    Ok(HttpResponse::Ok()
        .insert_header(CacheControl(vec![CacheDirective::NoStore]))
        .json(acces))
}

#[utoipa::path(
    get,
    description = "`AdminSessionMeeting` : la réunion Zoom de la séance, son état de synchronisation et ses compteurs d'inscrits (enregistrés chez Zoom, en échec). Exige `live.meeting.manage` sur l'édition. **404** sans réunion.",
    path = "/admin/sessions/{session_id}/meeting",
    tag = "Back-office : réunions Zoom",
    operation_id = "admin_seance_reunion_lire",
    params(("session_id" = Uuid, Path, description = "Séance")),
    responses(
        (status = 200, description = "AdminSessionMeeting", body = Object),
        (status = 401, description = "Aucune session", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Périmètre vide ou permission absente", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Séance inconnue, hors périmètre, ou sans réunion", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn lire_admin(
    state: web::Data<LiveState>,
    perimetre: Perimeter,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let tableau = meeting::lire_admin(&state, &perimetre, chemin.into_inner()).await?;
    Ok(HttpResponse::Ok().json(tableau))
}

#[utoipa::path(
    post,
    description = "`MeetingSyncInput` → `AdminSessionMeeting`. Crée la réunion Zoom de la séance, ou rattache celle dont on donne le numéro (`external_id`), puis inscrit chez Zoom chaque inscription inscrite, en attente ou présente, et relance ce qui avait échoué. Corps `{}` pour créer. Rejouable sans dommage : la création chez Zoom part en file.\n\nExige `live.meeting.manage` sur l'édition. `LIVE_ZOOM_UNAVAILABLE` (503) si Zoom n'est pas configuré, `LIVE_MEETING_EXTERNAL_ID_TAKEN` (409) si la réunion Zoom pilote déjà une autre séance.",
    path = "/admin/sessions/{session_id}/meeting",
    tag = "Back-office : réunions Zoom",
    operation_id = "admin_seance_reunion_synchroniser",
    params(("session_id" = Uuid, Path, description = "Séance")),
    responses(
        (status = 200, description = "AdminSessionMeeting", body = Object),
        (status = 401, description = "Aucune session", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Périmètre vide ou permission absente", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Séance inconnue ou hors périmètre", body = crate::routes::openapi::ApiErrorBody),
        (status = 409, description = "LIVE_MEETING_EXTERNAL_ID_TAKEN", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Numéro de réunion mal formé", body = crate::routes::openapi::ApiErrorBody),
        (status = 503, description = "LIVE_ZOOM_UNAVAILABLE", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn reconstruire(
    state: web::Data<LiveState>,
    requete: HttpRequest,
    perimetre: Perimeter,
    chemin: web::Path<Uuid>,
    corps: web::Json<MeetingSyncInput>,
) -> Result<HttpResponse> {
    let entree = corps.into_inner();
    let tableau = meeting::reconstruire(
        &state,
        &crate::routes::contexte_de(&requete, perimetre.person_id),
        &perimetre,
        chemin.into_inner(),
        &entree,
    )
    .await?;
    Ok(HttpResponse::Ok().json(tableau))
}

#[utoipa::path(
    post,
    description = "`AdminSessionMeeting`, après « Envoyer à Zoom maintenant ». Un déplacement fait partir l'horaire chez Zoom cinq minutes plus tard (`sync_due_at`), pour qu'une activité déplacée plusieurs fois ne compte qu'une modification dans le quota de Zoom (cent par jour). Ce geste annule l'attente et met la synchronisation en file tout de suite ; il vaut aussi pendant l'attente d'un quota, au risque d'un nouveau refus. Ne touche ni aux inscrits ni au rattachement. Sans corps.\n\nExige `live.meeting.manage` sur l'édition. **404** sans réunion. `LIVE_ZOOM_UNAVAILABLE` (503) si Zoom n'est pas configuré.",
    path = "/admin/sessions/{session_id}/meeting/push",
    tag = "Back-office : réunions Zoom",
    operation_id = "admin_seance_reunion_envoyer",
    params(("session_id" = Uuid, Path, description = "Séance")),
    responses(
        (status = 200, description = "AdminSessionMeeting", body = Object),
        (status = 401, description = "Aucune session", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Périmètre vide ou permission absente", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Séance inconnue, hors périmètre, ou sans réunion", body = crate::routes::openapi::ApiErrorBody),
        (status = 503, description = "LIVE_ZOOM_UNAVAILABLE", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn envoyer_maintenant(
    state: web::Data<LiveState>,
    requete: HttpRequest,
    perimetre: Perimeter,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let tableau = meeting::envoyer_maintenant(
        &state,
        &crate::routes::contexte_de(&requete, perimetre.person_id),
        &perimetre,
        chemin.into_inner(),
    )
    .await?;
    Ok(HttpResponse::Ok().json(tableau))
}

#[utoipa::path(
    get,
    description = "`MeetingHostLink`, le lien d'animateur de la réunion Zoom (« Ouvrir Zoom (animateur) » de la régie du jour). Redemandé à Zoom à chaque appel (le lien expire), le dernier connu à défaut. Ne se met jamais en cache : `Cache-Control: no-store`.\n\nExige `live.meeting.manage` sur l'édition. `LIVE_MEETING_NOT_READY` (409) sans réunion créée chez Zoom, `LIVE_ZOOM_UNAVAILABLE` (503) si Zoom n'est pas configuré.",
    path = "/admin/sessions/{session_id}/meeting/host",
    tag = "Back-office : réunions Zoom",
    operation_id = "admin_seance_reunion_lien_animateur",
    params(("session_id" = Uuid, Path, description = "Séance")),
    responses(
        (status = 200, description = "MeetingHostLink", body = Object),
        (status = 401, description = "Aucune session", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Périmètre vide ou permission absente", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Séance inconnue ou hors périmètre", body = crate::routes::openapi::ApiErrorBody),
        (status = 409, description = "LIVE_MEETING_NOT_READY", body = crate::routes::openapi::ApiErrorBody),
        (status = 503, description = "LIVE_ZOOM_UNAVAILABLE", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn lien_animateur(
    state: web::Data<LiveState>,
    perimetre: Perimeter,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let lien = meeting::lien_animateur(&state, &perimetre, chemin.into_inner()).await?;
    Ok(HttpResponse::Ok()
        .insert_header((actix_web::http::header::CACHE_CONTROL, "no-store"))
        .json(lien))
}

#[utoipa::path(
    get,
    description = "`PublicSessionMeeting` : la réunion d'une journée publiée (journée spéciale ou atelier), un seul direct de l'accueil à la clôture. Repères pris sur le déroulé de la journée, indicatifs comme pour une séance. **404** sans réunion. **Aucune garde.**",
    path = "/track-meetings/{track_id}",
    tag = "Direct",
    operation_id = "journee_reunion_publique",
    params(("track_id" = Uuid, Path, description = "Journée publiée")),
    responses(
        (status = 200, description = "PublicSessionMeeting", body = Object),
        (status = 404, description = "Journée inconnue, non publiée, ou sans réunion", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn publique_journee(
    state: web::Data<LiveState>,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let reunion =
        meeting::publique_journee(&state, chemin.into_inner(), OffsetDateTime::now_utc()).await?;
    Ok(HttpResponse::Ok().json(reunion))
}

#[utoipa::path(
    get,
    description = "`MeetingAccess` : l'entrée dans la réunion d'une journée, pour la personne connectée inscrite à la journée **ou** le porteur du lien reçu par courriel (en-tête `X-Manage-Token`, inscription sans compte). `join_url` est le lien Zoom personnel : la page du lien du courriel y renvoie dès qu'il est prêt. La première entrée est notée sur l'inscription à la journée. Réponse `Cache-Control: no-store`.\n\nRefus : `LIVE_ZOOM_UNAVAILABLE` (503), `LIVE_MEETING_REGISTRATION_REQUIRED` (403, ni inscription ni jeton valable), `LIVE_MEETING_NOT_READY` (409, place pas encore enregistrée chez Zoom : réessayer).",
    path = "/track-meetings/{track_id}/access",
    tag = "Direct",
    operation_id = "journee_reunion_acces",
    params(
        ("track_id" = Uuid, Path, description = "Journée publiée"),
        ("X-Manage-Token" = Option<String>, Header, description = "Jeton du lien reçu par courriel, sans session"),
    ),
    responses(
        (status = 200, description = "MeetingAccess", body = Object),
        (status = 401, description = "Ni session ni jeton", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "LIVE_MEETING_REGISTRATION_REQUIRED", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Journée inconnue ou non publiée", body = crate::routes::openapi::ApiErrorBody),
        (status = 409, description = "LIVE_MEETING_NOT_READY", body = crate::routes::openapi::ApiErrorBody),
        (status = 503, description = "LIVE_ZOOM_UNAVAILABLE", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn acces_journee(
    state: web::Data<LiveState>,
    requete: HttpRequest,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let (porteur, ctx) = match personne_connectee(&requete) {
        Some(p) => (
            meeting::Porteur::Personne(p),
            crate::routes::contexte_de(&requete, p),
        ),
        None => {
            let jeton = jeton_du_lien(&requete).ok_or_else(ApiError::unauthenticated)?;
            (meeting::Porteur::Jeton(jeton), contexte_anonyme(&requete))
        }
    };
    let acces = meeting::acces_journee(
        &state,
        &ctx,
        chemin.into_inner(),
        porteur,
        OffsetDateTime::now_utc(),
    )
    .await?;
    Ok(HttpResponse::Ok()
        .insert_header(CacheControl(vec![CacheDirective::NoStore]))
        .json(acces))
}

fn personne_connectee(requete: &HttpRequest) -> Option<Uuid> {
    use actix_web::HttpMessage;
    requete
        .extensions()
        .get::<kernel::context::RequestContext>()
        .and_then(|c| c.actor_id)
}

fn jeton_du_lien(requete: &HttpRequest) -> Option<String> {
    requete
        .headers()
        .get("x-manage-token")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|j| !j.is_empty() && j.len() <= 128)
        .map(str::to_owned)
}

fn contexte_anonyme(requete: &HttpRequest) -> kernel::context::RequestContext {
    kernel::context::RequestContext::new(
        kernel::context::RequestContext::generated_request_id(),
        crate::routes::locale_de(requete),
    )
}
