//! S'inscrire aux réunions de la Francophonie, et relire ses inscriptions avec
//! les liens de visioconférence auxquels on a droit.

use actix_web::http::header::{CACHE_CONTROL, ETAG};
use actix_web::{web, HttpRequest, HttpResponse};
use kernel::auth::Actor;
use kernel::error::Result;
use serde::Deserialize;
use uuid::Uuid;

use crate::domain::meeting_registrations::MeetingRegistrationPayload;
use crate::service::meeting_registrations as service;
use crate::state::NegotiationState;

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route(
        "/negotiation/me/meeting-registrations",
        web::get().to(mes_inscriptions),
    )
    .route(
        "/negotiation/me/meeting-registrations/{meeting_id}",
        web::put().to(sinscrire),
    )
    .route(
        "/negotiation/me/meeting-registrations/{meeting_id}",
        web::delete().to(se_desinscrire),
    );
}

/// Elle porte des liens de visioconférence : aucun cache ne la garde.
const PRIVE: &str = "private, no-store";

#[derive(Debug, Deserialize)]
pub struct Edition {
    edition: String,
}

#[utoipa::path(
    get,
    description = "`MyMeetingRegistrations` — les inscriptions de la personne connectée aux réunions de l'édition (inscrite, ou en liste d'attente avec sa position) et, dans `video`, les liens de visioconférence auxquels elle a droit : chaque réunion où elle est **inscrite**, et chaque réunion sans inscription si elle a l'accès négociateur. Jamais pour la liste d'attente.\n\n`Cache-Control: private, no-store` ; `ETag` propre à la personne, **304** sur `If-None-Match`.",
    path = "/negotiation/me/meeting-registrations",
    tag = "Guide Négo — réunions de la Francophonie",
    operation_id = "negotiation_mes_inscriptions_reunions",
    params(("edition" = String, Query, description = "Slug de l'édition")),
    responses(
        (status = 200, description = "MyMeetingRegistrations", body = Object),
        (status = 304, description = "Rien n'a changé depuis l'empreinte présentée"),
        (status = 401, description = "Aucune session", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Édition inconnue", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn mes_inscriptions(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    acteur: Actor,
    edition: web::Query<Edition>,
) -> Result<HttpResponse> {
    let liste = service::mes_inscriptions(&state, acteur.0, &edition.edition).await?;
    let empreinte = liste.empreinte(acteur.0);

    if crate::routes::inchange(&requete, &empreinte) {
        return Ok(HttpResponse::NotModified()
            .insert_header((ETAG, empreinte))
            .insert_header((CACHE_CONTROL, PRIVE))
            .finish());
    }

    Ok(HttpResponse::Ok()
        .insert_header((ETAG, empreinte))
        .insert_header((CACHE_CONTROL, PRIVE))
        .json(liste))
}

#[utoipa::path(
    put,
    description = "`MeetingRegistrationPayload` → `MeetingRegistrationState` — s'inscrire, ou rejoindre la liste d'attente quand la réunion est complète ou que la liste n'est pas vide. Réservé à l'accès négociateur, portée globale. **Un `client_ref` neuf par geste.**\n\nMême `client_ref` que la ligne, ou déjà inscrite : **200** et l'état courant, rien d'écrit. Désinscrite et `client_ref` neuf : réinscription, en fin de liste d'attente si elle n'est pas vide.\n\n**409** `NEGOTIATION_MEETING_FULL` (complet, sans liste d'attente), `NEGOTIATION_MEETING_CLOSED` (inscription non demandée, ou hors de la fenêtre), `NEGOTIATION_MEETING_UNAVAILABLE` (annulée, commencée, brouillon).",
    path = "/negotiation/me/meeting-registrations/{meeting_id}",
    tag = "Guide Négo — réunions de la Francophonie",
    operation_id = "negotiation_sinscrire_a_une_reunion",
    params(("meeting_id" = Uuid, Path, description = "Identifiant de la réunion")),
    request_body = Object,
    responses(
        (status = 200, description = "MeetingRegistrationState", body = Object),
        (status = 401, description = "Aucune session", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Sans accès négociateur", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Réunion inconnue", body = crate::routes::openapi::ApiErrorBody),
        (status = 409, description = "Complète, close ou indisponible", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Corps malformé, ou référence déjà servie", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn sinscrire(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    acteur: Actor,
    chemin: web::Path<Uuid>,
    charge: web::Json<MeetingRegistrationPayload>,
) -> Result<HttpResponse> {
    let ctx = crate::routes::contexte_de(&requete, acteur.0);
    let etat = service::inscrire(
        &state,
        &ctx,
        acteur.0,
        chemin.into_inner(),
        charge.client_ref,
    )
    .await?;
    Ok(HttpResponse::Ok().json(etat))
}

#[utoipa::path(
    delete,
    description = "Se désinscrire, ou quitter la liste d'attente. **Idempotent**. La première personne en attente prend la place libérée, dans la même transaction.\n\nRefusé une fois la réunion commencée : **409** `NEGOTIATION_MEETING_UNAVAILABLE`.",
    path = "/negotiation/me/meeting-registrations/{meeting_id}",
    tag = "Guide Négo — réunions de la Francophonie",
    operation_id = "negotiation_se_desinscrire_dune_reunion",
    params(("meeting_id" = Uuid, Path, description = "Identifiant de la réunion")),
    responses(
        (status = 204, description = "Désinscrite"),
        (status = 401, description = "Aucune session", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Réunion inconnue", body = crate::routes::openapi::ApiErrorBody),
        (status = 409, description = "Réunion commencée", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn se_desinscrire(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    acteur: Actor,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let ctx = crate::routes::contexte_de(&requete, acteur.0);
    service::desinscrire(&state, &ctx, acteur.0, chemin.into_inner()).await?;
    Ok(HttpResponse::NoContent().finish())
}
