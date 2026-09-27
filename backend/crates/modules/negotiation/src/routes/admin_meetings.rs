//! Les réunions de la Francophonie, au back-office.
//!
//! Garde : `Requires<MeetingManage>` **sur la portée globale**, jamais
//! `RequiresAnyScope` (research R9, décision du 21/09). Le rôle `admin` porte la
//! permission sur un événement et `space_lead` sur un espace : une garde plus
//! large les laisserait entrer.

use actix_web::{web, HttpRequest, HttpResponse};
use kernel::auth::Requires;
use kernel::error::Result;
use uuid::Uuid;

use crate::domain::admin_import::EditionQuery;
use crate::domain::admin_meetings::{
    CancelMeetingPayload, FrancophoneMeetingInput, MeetingPavilionPayload,
};
use crate::domain::permissions::MeetingManage;
use crate::service::admin_meetings as service;
use crate::state::NegotiationState;

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/admin/negotiation/meetings", web::get().to(lister))
        .route("/admin/negotiation/meetings", web::post().to(creer))
        .route("/admin/negotiation/meetings/{id}", web::get().to(fiche))
        .route("/admin/negotiation/meetings/{id}", web::put().to(modifier))
        .route(
            "/admin/negotiation/meetings/{id}/publish",
            web::post().to(publier),
        )
        .route(
            "/admin/negotiation/meetings/{id}/cancel",
            web::post().to(annuler),
        )
        .route(
            "/admin/negotiation/meetings/{id}/pavilion",
            web::put().to(lier_au_pavillon),
        )
        .route(
            "/admin/negotiation/meetings/{id}/registrations",
            web::get().to(inscrites),
        )
        .route(
            "/admin/negotiation/pavilion-activities",
            web::get().to(activites_du_pavillon),
        );
}

#[utoipa::path(
    get,
    description = "`AdminFrancophoneMeetings` — les réunions de la Francophonie de l'édition, **brouillons compris**, par début, avec le nombre d'inscrites (`registered_count`) et d'attente (`waitlisted_count`).",
    path = "/admin/negotiation/meetings",
    tag = "Back-office — réunions de la Francophonie",
    operation_id = "admin_negotiation_reunions",
    params(("edition" = String, Query, description = "Slug de l'édition")),
    responses(
        (status = 200, description = "AdminFrancophoneMeetings", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Sans `negotiation.meeting.manage` **sur la portée globale**", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Édition inconnue", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn lister(
    state: web::Data<NegotiationState>,
    _garde: Requires<MeetingManage>,
    edition: web::Query<EditionQuery>,
) -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(service::lister(&state, &edition.edition).await?))
}

#[utoipa::path(
    post,
    description = "`FrancophoneMeetingInput` → `AdminFrancophoneMeeting` — saisit un brouillon. Le serveur pose l'espace `climat`, le slug `<édition>-<nature>-<suffixe>`, l'édition, son fuseau et le `kind` tiré de la nature.\n\nUn invariant refusé sort en `NEGOTIATION_MEETING_INVALID`, qui nomme le champ. Un chevauchement avec une autre réunion n'est jamais refusé.",
    path = "/admin/negotiation/meetings",
    tag = "Back-office — réunions de la Francophonie",
    operation_id = "admin_negotiation_reunion_creer",
    request_body = Object,
    responses(
        (status = 201, description = "AdminFrancophoneMeeting", body = Object),
        (status = 400, description = "Réunion incomplète : `field` nomme le champ", body = crate::routes::openapi::ApiErrorBody),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Sans la permission sur la portée globale", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Édition inconnue", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn creer(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    garde: Requires<MeetingManage>,
    corps: web::Json<FrancophoneMeetingInput>,
) -> Result<HttpResponse> {
    let contexte = crate::routes::contexte_de(&requete, garde.person_id);
    let reunion = service::creer(&state, &contexte, corps.into_inner()).await?;
    Ok(HttpResponse::Created().json(reunion))
}

#[utoipa::path(
    get,
    description = "`AdminFrancophoneMeeting` — une réunion, quel que soit son état.",
    path = "/admin/negotiation/meetings/{id}",
    tag = "Back-office — réunions de la Francophonie",
    operation_id = "admin_negotiation_reunion",
    params(("id" = Uuid, Path, description = "Identifiant de la réunion")),
    responses(
        (status = 200, description = "AdminFrancophoneMeeting", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Sans la permission sur la portée globale", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Réunion inconnue", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn fiche(
    state: web::Data<NegotiationState>,
    _garde: Requires<MeetingManage>,
    id: web::Path<Uuid>,
) -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(service::fiche(&state, id.into_inner()).await?))
}

#[utoipa::path(
    put,
    description = "`FrancophoneMeetingInput` → `AdminFrancophoneMeeting` — pose la réunion entière ; `edition` est ignorée. **Relever ou retirer la capacité promeut la liste d'attente** dans la même transaction. Aucune garde sur `updated_at` : chaque inscription réécrit la réunion.",
    path = "/admin/negotiation/meetings/{id}",
    tag = "Back-office — réunions de la Francophonie",
    operation_id = "admin_negotiation_reunion_modifier",
    params(("id" = Uuid, Path, description = "Identifiant de la réunion")),
    request_body = Object,
    responses(
        (status = 200, description = "AdminFrancophoneMeeting", body = Object),
        (status = 400, description = "Réunion incomplète : `field` nomme le champ", body = crate::routes::openapi::ApiErrorBody),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Sans la permission sur la portée globale", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Réunion inconnue", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn modifier(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    garde: Requires<MeetingManage>,
    id: web::Path<Uuid>,
    corps: web::Json<FrancophoneMeetingInput>,
) -> Result<HttpResponse> {
    let contexte = crate::routes::contexte_de(&requete, garde.person_id);
    let reunion = service::modifier(&state, &contexte, id.into_inner(), corps.into_inner()).await?;
    Ok(HttpResponse::Ok().json(reunion))
}

#[utoipa::path(
    post,
    description = "`AdminFrancophoneMeeting` — brouillon → publiée. Une réunion en ligne sans lien, sur place sans lieu, ou sans nature sort en `NEGOTIATION_MEETING_INVALID` qui nomme le champ ; une réunion annulée, en `NEGOTIATION_MEETING_UNAVAILABLE`. Déjà publiée : rien ne change.",
    path = "/admin/negotiation/meetings/{id}/publish",
    tag = "Back-office — réunions de la Francophonie",
    operation_id = "admin_negotiation_reunion_publier",
    params(("id" = Uuid, Path, description = "Identifiant de la réunion")),
    responses(
        (status = 200, description = "AdminFrancophoneMeeting", body = Object),
        (status = 400, description = "Réunion incomplète : `field` nomme le champ", body = crate::routes::openapi::ApiErrorBody),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Sans la permission sur la portée globale", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Réunion inconnue", body = crate::routes::openapi::ApiErrorBody),
        (status = 409, description = "Réunion annulée", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn publier(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    garde: Requires<MeetingManage>,
    id: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let contexte = crate::routes::contexte_de(&requete, garde.person_id);
    Ok(HttpResponse::Ok().json(service::publier(&state, &contexte, id.into_inner()).await?))
}

#[utoipa::path(
    post,
    description = "`CancelMeetingPayload` → `AdminFrancophoneMeeting` — annule, avec un motif. Déjà annulée : rien ne change.",
    path = "/admin/negotiation/meetings/{id}/cancel",
    tag = "Back-office — réunions de la Francophonie",
    operation_id = "admin_negotiation_reunion_annuler",
    params(("id" = Uuid, Path, description = "Identifiant de la réunion")),
    request_body = Object,
    responses(
        (status = 200, description = "AdminFrancophoneMeeting", body = Object),
        (status = 400, description = "Motif absent", body = crate::routes::openapi::ApiErrorBody),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Sans la permission sur la portée globale", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Réunion inconnue", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn annuler(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    garde: Requires<MeetingManage>,
    id: web::Path<Uuid>,
    corps: web::Json<CancelMeetingPayload>,
) -> Result<HttpResponse> {
    let contexte = crate::routes::contexte_de(&requete, garde.person_id);
    let reunion = service::annuler(&state, &contexte, id.into_inner(), &corps.reason).await?;
    Ok(HttpResponse::Ok().json(reunion))
}

#[utoipa::path(
    put,
    description = "`MeetingPavilionPayload` → `AdminFrancophoneMeeting` — lie la réunion à une activité du Pavillon **de la même édition**, ou retire le lien (`null`). Une activité d'une autre édition sort en `NEGOTIATION_MEETING_INVALID` (`pavilion_session_id`). Rien n'est écrit dans le programme.",
    path = "/admin/negotiation/meetings/{id}/pavilion",
    tag = "Back-office — réunions de la Francophonie",
    operation_id = "admin_negotiation_reunion_pavillon",
    params(("id" = Uuid, Path, description = "Identifiant de la réunion")),
    request_body = Object,
    responses(
        (status = 200, description = "AdminFrancophoneMeeting", body = Object),
        (status = 400, description = "Activité d'une autre édition", body = crate::routes::openapi::ApiErrorBody),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Sans la permission sur la portée globale", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Réunion inconnue", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn lier_au_pavillon(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    garde: Requires<MeetingManage>,
    id: web::Path<Uuid>,
    corps: web::Json<MeetingPavilionPayload>,
) -> Result<HttpResponse> {
    let contexte = crate::routes::contexte_de(&requete, garde.person_id);
    let reunion = service::lier_au_pavillon(
        &state,
        &contexte,
        id.into_inner(),
        corps.pavilion_session_id,
    )
    .await?;
    Ok(HttpResponse::Ok().json(reunion))
}

#[utoipa::path(
    get,
    description = "`AdminMeetingRegistrations` — les inscrites et la liste d'attente (nom, pays, date d'inscription, position), désinscrites exclues.",
    path = "/admin/negotiation/meetings/{id}/registrations",
    tag = "Back-office — réunions de la Francophonie",
    operation_id = "admin_negotiation_reunion_inscrites",
    params(("id" = Uuid, Path, description = "Identifiant de la réunion")),
    responses(
        (status = 200, description = "AdminMeetingRegistrations", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Sans la permission sur la portée globale", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Réunion inconnue", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn inscrites(
    state: web::Data<NegotiationState>,
    _garde: Requires<MeetingManage>,
    id: web::Path<Uuid>,
) -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(service::inscrites(&state, id.into_inner()).await?))
}

#[utoipa::path(
    get,
    description = "`PavilionActivityOption[]` — les activités du programme de l'édition, par début, pour le sélecteur. Lecture seule de `programme.sessions`.",
    path = "/admin/negotiation/pavilion-activities",
    tag = "Back-office — réunions de la Francophonie",
    operation_id = "admin_negotiation_activites_du_pavillon",
    params(("edition" = String, Query, description = "Slug de l'édition")),
    responses(
        (status = 200, description = "PavilionActivityOption[]", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Sans la permission sur la portée globale", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Édition inconnue", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn activites_du_pavillon(
    state: web::Data<NegotiationState>,
    _garde: Requires<MeetingManage>,
    edition: web::Query<EditionQuery>,
) -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(service::activites_du_pavillon(&state, &edition.edition).await?))
}
