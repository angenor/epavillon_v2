//! La régie du jour (02/11), `/admin/regie`. Les gestes rendent un
//! `RegieGesture`, de quoi afficher « Défaire » ; l'écran se relit au signal
//! `event:<id>:regie`.

use actix_web::{web, HttpRequest, HttpResponse};
use kernel::auth::Perimeter;
use kernel::error::Result;
use serde::Deserialize;
use time::Date;
use uuid::Uuid;

use crate::domain::regie::{RegieCancelPayload, RegieUndoPayload};
use crate::routes::contexte_de;
use crate::service::{regie, regie_ecran};
use crate::state::ProgrammeState;

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("", web::get().to(ecran))
        .route("/sessions/{sessionId}/presence", web::post().to(presence))
        .route("/sessions/{sessionId}/cancel", web::post().to(annuler))
        .route("/sessions/{sessionId}/live/start", web::post().to(demarrer))
        .route("/sessions/{sessionId}/live/end", web::post().to(terminer))
        .route("/actions/{actionId}/undo", web::post().to(defaire));
}

#[derive(Debug, Deserialize)]
pub struct JourDeRegie {
    event_id: Uuid,
    day: Option<Date>,
}

#[utoipa::path(
    get,
    description = "`RegieScreen` : la régie d'une journée : séances publiées du jour (organisations, présentations déposées, réunion Zoom, présence, annulation et état des courriels aux inscrits), journal des gestes et des messages d'incident publiés ce jour, direct en cours ailleurs (un seul à la fois, toutes éditions), raisons d'annulation. `day` facultatif : aujourd'hui dans le fuseau de l'édition s'il est au programme, sinon le prochain jour, sinon le premier.",
    path = "/admin/regie",
    tag = "Régie du jour",
    operation_id = "regie_ecran",
    params(
        ("event_id" = Uuid, Query, description = "Édition"),
        ("day" = Option<String>, Query, description = "Jour, AAAA-MM-JJ"),
    ),
    responses(
        (status = 200, description = "RegieScreen", body = Object),
        (status = 401, description = "Aucune session", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission de planifier absente sur l'édition, ou périmètre vide", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Édition inexistante ou hors périmètre, indiscernables", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn ecran(
    state: web::Data<ProgrammeState>,
    perimetre: Perimeter,
    demande: web::Query<JourDeRegie>,
) -> Result<HttpResponse> {
    let ecran = regie_ecran::ecran(&state, &perimetre, demande.event_id, demande.day).await?;
    Ok(HttpResponse::Ok().json(ecran))
}

#[utoipa::path(
    post,
    description = "`RegieGesture` : les organisateurs sont là : présence confirmée, inscrite au journal. Refusé (409) pour une activité annulée.",
    path = "/admin/regie/sessions/{sessionId}/presence",
    tag = "Régie du jour",
    operation_id = "regie_confirmer_la_presence",
    params(("sessionId" = Uuid, Path, description = "Séance")),
    responses(
        (status = 200, description = "RegieGesture", body = Object),
        (status = 403, description = "Permission de planifier absente", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Séance inexistante ou hors périmètre", body = crate::routes::openapi::ApiErrorBody),
        (status = 409, description = "La séance a changé entre-temps", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn presence(
    requete: HttpRequest,
    state: web::Data<ProgrammeState>,
    perimetre: Perimeter,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let ctx = contexte_de(&requete, perimetre.person_id);
    let g = regie::confirmer_presence(&state, &ctx, &perimetre, chemin.into_inner()).await?;
    Ok(HttpResponse::Ok().json(g))
}

#[utoipa::path(
    post,
    description = "`RegieCancelPayload` → `RegieGesture`. Annule une activité publiée (programmée, en direct ou reportée) avec une raison de la taxonomie `session_cancellation_reason` et une note interne jamais publiée ; « pas venus » et « ont prévenu » notent les organisateurs absents. `notify_registrants` : les inscrits reçoivent un courriel **cinq minutes plus tard** (`notify_at`), et défaire l'annulation d'ici là le retire avant tout envoi. Le message d'incident se publie à part (`POST /admin/incidents`).",
    path = "/admin/regie/sessions/{sessionId}/cancel",
    tag = "Régie du jour",
    operation_id = "regie_annuler",
    params(("sessionId" = Uuid, Path, description = "Séance")),
    request_body = Object,
    responses(
        (status = 200, description = "RegieGesture", body = Object),
        (status = 403, description = "Permission de planifier absente", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Séance inexistante ou hors périmètre", body = crate::routes::openapi::ApiErrorBody),
        (status = 409, description = "La séance n'est plus annulable (déjà annulée, terminée, non publiée)", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Raison inconnue, note trop longue", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn annuler(
    requete: HttpRequest,
    state: web::Data<ProgrammeState>,
    perimetre: Perimeter,
    chemin: web::Path<Uuid>,
    corps: web::Json<RegieCancelPayload>,
) -> Result<HttpResponse> {
    let ctx = contexte_de(&requete, perimetre.person_id);
    let g = regie::annuler(
        &state,
        &ctx,
        &perimetre,
        chemin.into_inner(),
        corps.into_inner(),
    )
    .await?;
    Ok(HttpResponse::Ok().json(g))
}

#[utoipa::path(
    post,
    description = "`RegieGesture` : met en direct une activité publiée et programmée. **Un seul direct à la fois, toutes éditions confondues** : refus 409 `REGIE_LIVE_ALREADY_RUNNING` tant qu'un autre tourne.",
    path = "/admin/regie/sessions/{sessionId}/live/start",
    tag = "Régie du jour",
    operation_id = "regie_demarrer_le_direct",
    params(("sessionId" = Uuid, Path, description = "Séance")),
    responses(
        (status = 200, description = "RegieGesture", body = Object),
        (status = 403, description = "Permission de planifier absente", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Séance inexistante ou hors périmètre", body = crate::routes::openapi::ApiErrorBody),
        (status = 409, description = "Un autre direct tourne, ou la séance n'est pas programmée", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn demarrer(
    requete: HttpRequest,
    state: web::Data<ProgrammeState>,
    perimetre: Perimeter,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let ctx = contexte_de(&requete, perimetre.person_id);
    let g = regie::demarrer(&state, &ctx, &perimetre, chemin.into_inner()).await?;
    Ok(HttpResponse::Ok().json(g))
}

#[utoipa::path(
    post,
    description = "`RegieGesture` : termine le direct. `resumable_until` : pendant quinze minutes, défaire ce geste reprend le direct (même réunion Zoom, même lien).",
    path = "/admin/regie/sessions/{sessionId}/live/end",
    tag = "Régie du jour",
    operation_id = "regie_terminer_le_direct",
    params(("sessionId" = Uuid, Path, description = "Séance")),
    responses(
        (status = 200, description = "RegieGesture", body = Object),
        (status = 403, description = "Permission de planifier absente", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Séance inexistante ou hors périmètre", body = crate::routes::openapi::ApiErrorBody),
        (status = 409, description = "La séance n'est pas en direct", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn terminer(
    requete: HttpRequest,
    state: web::Data<ProgrammeState>,
    perimetre: Perimeter,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let ctx = contexte_de(&requete, perimetre.person_id);
    let g = regie::terminer(&state, &ctx, &perimetre, chemin.into_inner()).await?;
    Ok(HttpResponse::Ok().json(g))
}

#[utoipa::path(
    post,
    description = "`RegieUndoPayload` → `RegieGesture`. Défait une ligne du journal, même plus tard et par un autre membre de l'équipe : présence remise comme avant ; annulation rétablie (l'activité revient à son créneau avec ses inscrits ; `notify_registrants` écrit « finalement maintenue » à ceux qui ont reçu le courriel d'annulation, un courriel parti ne se reprenant pas) ; direct démarré remis programmé ; direct terminé repris, dans les quinze minutes seulement (`REGIE_RESUME_EXPIRED` après), un seul direct à la fois. Refus 409 si la séance a changé entre-temps ou si le geste est déjà défait.",
    path = "/admin/regie/actions/{actionId}/undo",
    tag = "Régie du jour",
    operation_id = "regie_defaire",
    params(("actionId" = Uuid, Path, description = "Geste du journal")),
    request_body = Object,
    responses(
        (status = 200, description = "RegieGesture", body = Object),
        (status = 403, description = "Permission de planifier absente", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Geste inexistant ou hors périmètre", body = crate::routes::openapi::ApiErrorBody),
        (status = 409, description = "Déjà défait, séance changée, reprise expirée, autre direct en cours", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn defaire(
    requete: HttpRequest,
    state: web::Data<ProgrammeState>,
    perimetre: Perimeter,
    chemin: web::Path<Uuid>,
    corps: Option<web::Json<RegieUndoPayload>>,
) -> Result<HttpResponse> {
    let ctx = contexte_de(&requete, perimetre.person_id);
    let charge = corps.map(web::Json::into_inner).unwrap_or_default();
    let g = regie::defaire(&state, &ctx, &perimetre, chemin.into_inner(), charge).await?;
    Ok(HttpResponse::Ok().json(g))
}
