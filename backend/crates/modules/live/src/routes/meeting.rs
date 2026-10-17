//! Les réunions Zoom d'une séance : lecture publique, entrée d'une personne
//! inscrite, back-office. Routes plates (voir `admin.rs`).

use actix_web::http::header::{CacheControl, CacheDirective};
use actix_web::{web, HttpRequest, HttpResponse};
use kernel::auth::{Actor, Perimeter};
use kernel::error::Result;
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

pub fn chemins_admin(cfg: &mut web::ServiceConfig) {
    cfg.route(
        "/admin/sessions/{session_id}/meeting",
        web::get().to(lire_admin),
    )
    .route(
        "/admin/sessions/{session_id}/meeting",
        web::post().to(reconstruire),
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
    description = "`MeetingAccess` : de quoi entrer dans la salle avec le Meeting SDK de Zoom (web, 6.5) : numéro, code d'accès, jeton d'inscrit `tk`, signature du SDK (deux heures), lien personnel en secours.\n\nRéservé à une personne **inscrite, en liste d'attente ou déjà présente** : le jour même, Zoom laisse entrer les trois cents premiers arrivés. **Séance en entrée libre** (sans inscription) : toute personne connectée entre ; sa place chez Zoom est créée au premier appel, pendant la requête, et si Zoom ne répond pas la route dit « pas prête » et l'on réessaie. Réponse `Cache-Control: no-store`.\n\n**Aucune fenêtre horaire** : on entre à toute heure tant que la réunion existe chez Zoom, même terminée (l'animateur peut la relancer), jusqu'à l'annulation de la séance (`LIVE_MEETING_CANCELLED`, 409).\n\nRefus : `LIVE_ZOOM_UNAVAILABLE` (503, Zoom non configuré), `LIVE_MEETING_REGISTRATION_REQUIRED` (403, jamais en entrée libre),  `LIVE_MEETING_NOT_READY` (409, réunion ou inscrit pas encore enregistrés chez Zoom).",
    path = "/sessions/{session_id}/meeting/access",
    tag = "Direct",
    operation_id = "seance_reunion_acces",
    params(("session_id" = Uuid, Path, description = "Séance publiée")),
    responses(
        (status = 200, description = "MeetingAccess", body = Object),
        (status = 401, description = "Aucune session", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "LIVE_MEETING_REGISTRATION_REQUIRED", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Séance inconnue ou non publiée", body = crate::routes::openapi::ApiErrorBody),
        (status = 409, description = "LIVE_MEETING_CANCELLED ou LIVE_MEETING_NOT_READY", body = crate::routes::openapi::ApiErrorBody),
        (status = 503, description = "LIVE_ZOOM_UNAVAILABLE", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn acces(
    state: web::Data<LiveState>,
    requete: HttpRequest,
    acteur: Actor,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let acces = meeting::acces(
        &state,
        &crate::routes::contexte_de(&requete, acteur.0),
        acteur.0,
        chemin.into_inner(),
        OffsetDateTime::now_utc(),
    )
    .await?;
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
