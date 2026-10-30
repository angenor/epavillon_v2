//! L'éditeur de la page publique d'un fil (30/10), sous `/admin/tracks/{id}`.
//!
//! Même garde que le fil lui-même : **l'édition vient de l'ascendance du fil**,
//! le périmètre est vérifié avant la permission de gérer les événements (règle
//! n° 8, URL forgée comprise). Une partie ou un moment d'un autre fil rend le
//! même 404 qu'un identifiant inconnu.
//!
//! Ces chemins sont déposés dans le scope `/admin/tracks` qu'ouvre
//! `admin_tabs` : deux scopes du même préfixe ne se complètent pas.

use actix_web::{web, HttpRequest, HttpResponse};
use kernel::auth::Perimeter;
use kernel::error::Result;
use uuid::Uuid;

use crate::domain::ids::TrackId;
use crate::domain::track_page::{TrackMomentPayload, TrackPagePayload, TrackPartPayload};
use crate::routes::admin_tabs::garder;
use crate::routes::contexte_de;
use crate::service::{track_page as service, Cible};
use crate::state::EventState;

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/{id}/page", web::get().to(lire))
        .route("/{id}/page", web::put().to(enregistrer))
        .route("/{id}/parts", web::post().to(creer_partie))
        .route("/{id}/parts/{partId}", web::put().to(modifier_partie))
        .route("/{id}/parts/{partId}", web::delete().to(supprimer_partie))
        .route("/{id}/moments", web::post().to(creer_moment))
        .route("/{id}/moments/{momentId}", web::put().to(modifier_moment))
        .route("/{id}/moments/{momentId}", web::delete().to(supprimer_moment));
}

async fn fil_garde(state: &EventState, perimetre: &Perimeter, id: Uuid) -> Result<TrackId> {
    let track = TrackId::from(id);
    garder(state, perimetre, Cible::Fil(track)).await?;
    Ok(track)
}

/// Lire les réglages de la page d'un fil.
#[utoipa::path(
    get,
    description = "`TrackPageSettings` : ce que la page publique du fil affiche au-delà du fil lui-même. Lieu (`venue_id` nul : le pavillon de l'édition), accroche, publics cibles (un texte multilingue par public), mention de soutien, encart mis en avant (surtitre, titre, texte, libellé et adresse du lien), parties du déroulé, moments sans activité, partenaires résolus (nom, sigle, logo, rôle et son libellé) et la **composition** : chaque activité rattachée, publiée ou non, avec sa partie, son temps fort et son rang. Les médias (photo du bandeau, visuel de l'encart, documents) se posent par le module Médias sur `event.programme_tracks`.",
    path = "/admin/tracks/{id}/page",
    tag = "Back-office — événements",
    operation_id = "admin_fil_page_lire",
    params(("id" = uuid::Uuid, Path, description = "Identifiant du fil")),
    responses(
        (status = 200, description = "TrackPageSettings", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente, ou périmètre d'administration vide", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Fil inexistant **ou hors périmètre**, indiscernables", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn lire(
    state: web::Data<EventState>,
    perimetre: Perimeter,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let track = fil_garde(&state, &perimetre, chemin.into_inner()).await?;
    Ok(HttpResponse::Ok().json(service::lire(state.pool(), track).await?))
}

/// Enregistrer les réglages de la page et ses partenaires.
#[utoipa::path(
    put,
    description = "`TrackPagePayload` → `TrackPageSettings`. Écriture **totale** des colonnes de la page et **remplacement** de la liste des partenaires (l'ordre envoyé est l'ordre affiché ; une organisation citée deux fois garde sa première place ; `role_code` relève de la taxonomie `track_partner_role`, `partner` par défaut). Le lieu doit appartenir à l'édition du fil. Le lien de l'encart est une adresse absolue ou un chemin du site (`/…`), donné avec son libellé. Parties et moments ont leurs propres routes.",
    path = "/admin/tracks/{id}/page",
    tag = "Back-office — événements",
    operation_id = "admin_fil_page_enregistrer",
    params(("id" = uuid::Uuid, Path, description = "Identifiant du fil")),
    request_body = Object,
    responses(
        (status = 200, description = "TrackPageSettings", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente, ou périmètre d'administration vide", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Fil inexistant **ou hors périmètre**, indiscernables", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Lieu d'une autre édition, organisation ou rôle inconnu, public sans français, lien sans libellé", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn enregistrer(
    requete: HttpRequest,
    state: web::Data<EventState>,
    perimetre: Perimeter,
    chemin: web::Path<Uuid>,
    corps: web::Json<TrackPagePayload>,
) -> Result<HttpResponse> {
    let track = fil_garde(&state, &perimetre, chemin.into_inner()).await?;
    let ctx = contexte_de(&requete, perimetre.person_id);
    let reglages = service::enregistrer(&state, &ctx, track, corps.into_inner()).await?;
    Ok(HttpResponse::Ok().json(reglages))
}

/// Ajouter une partie au déroulé.
#[utoipa::path(
    post,
    description = "`TrackPartPayload` → `TrackPageSettings`. Nom court (`label`, « Matin » ; nul : « Partie N ») ou intitulé (`title`) exigé, résumé d'une ligne facultatif, rang.",
    path = "/admin/tracks/{id}/parts",
    tag = "Back-office — événements",
    operation_id = "admin_fil_partie_creer",
    params(("id" = uuid::Uuid, Path, description = "Identifiant du fil")),
    request_body = Object,
    responses(
        (status = 200, description = "TrackPageSettings", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente, ou périmètre d'administration vide", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Fil inexistant **ou hors périmètre**, indiscernables", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Ni nom ni intitulé", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn creer_partie(
    requete: HttpRequest,
    state: web::Data<EventState>,
    perimetre: Perimeter,
    chemin: web::Path<Uuid>,
    corps: web::Json<TrackPartPayload>,
) -> Result<HttpResponse> {
    let track = fil_garde(&state, &perimetre, chemin.into_inner()).await?;
    let ctx = contexte_de(&requete, perimetre.person_id);
    let reglages = service::creer_partie(&state, &ctx, track, corps.into_inner()).await?;
    Ok(HttpResponse::Ok().json(reglages))
}

/// Modifier une partie.
#[utoipa::path(
    put,
    description = "`TrackPartPayload` → `TrackPageSettings`. Écriture totale de la partie. Une partie d'un autre fil est introuvable.",
    path = "/admin/tracks/{id}/parts/{partId}",
    tag = "Back-office — événements",
    operation_id = "admin_fil_partie_modifier",
    params(
        ("id" = uuid::Uuid, Path, description = "Identifiant du fil"),
        ("partId" = uuid::Uuid, Path, description = "Identifiant de la partie"),
    ),
    request_body = Object,
    responses(
        (status = 200, description = "TrackPageSettings", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente, ou périmètre d'administration vide", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Fil ou partie inexistants **ou hors périmètre**, indiscernables", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Ni nom ni intitulé", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn modifier_partie(
    requete: HttpRequest,
    state: web::Data<EventState>,
    perimetre: Perimeter,
    chemin: web::Path<(Uuid, Uuid)>,
    corps: web::Json<TrackPartPayload>,
) -> Result<HttpResponse> {
    let (id, part) = chemin.into_inner();
    let track = fil_garde(&state, &perimetre, id).await?;
    let ctx = contexte_de(&requete, perimetre.person_id);
    let reglages = service::modifier_partie(&state, &ctx, track, part, corps.into_inner()).await?;
    Ok(HttpResponse::Ok().json(reglages))
}

/// Retirer une partie.
#[utoipa::path(
    delete,
    description = "`TrackPageSettings`. Les moments et les activités de la partie **restent dans le fil**, hors partie.",
    path = "/admin/tracks/{id}/parts/{partId}",
    tag = "Back-office — événements",
    operation_id = "admin_fil_partie_supprimer",
    params(
        ("id" = uuid::Uuid, Path, description = "Identifiant du fil"),
        ("partId" = uuid::Uuid, Path, description = "Identifiant de la partie"),
    ),
    responses(
        (status = 200, description = "TrackPageSettings", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente, ou périmètre d'administration vide", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Fil ou partie inexistants **ou hors périmètre**, indiscernables", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn supprimer_partie(
    requete: HttpRequest,
    state: web::Data<EventState>,
    perimetre: Perimeter,
    chemin: web::Path<(Uuid, Uuid)>,
) -> Result<HttpResponse> {
    let (id, part) = chemin.into_inner();
    let track = fil_garde(&state, &perimetre, id).await?;
    let ctx = contexte_de(&requete, perimetre.person_id);
    let reglages = service::supprimer_partie(&state, &ctx, track, part).await?;
    Ok(HttpResponse::Ok().json(reglages))
}

/// Ajouter un moment sans activité.
#[utoipa::path(
    post,
    description = "`TrackMomentPayload` → `TrackPageSettings`. Accueil, déjeuner, pause, clôture : **pas une activité** (ni inscription, ni conflit, ni programmation publique). Début obligatoire, fin facultative et postérieure, libellé multilingue, précision et partie facultatives. Heures absolues (RFC 3339), affichées dans le fuseau de l'édition.",
    path = "/admin/tracks/{id}/moments",
    tag = "Back-office — événements",
    operation_id = "admin_fil_moment_creer",
    params(("id" = uuid::Uuid, Path, description = "Identifiant du fil")),
    request_body = Object,
    responses(
        (status = 200, description = "TrackPageSettings", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente, ou périmètre d'administration vide", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Fil inexistant **ou hors périmètre**, indiscernables", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Libellé absent, fin avant le début, partie d'un autre fil", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn creer_moment(
    requete: HttpRequest,
    state: web::Data<EventState>,
    perimetre: Perimeter,
    chemin: web::Path<Uuid>,
    corps: web::Json<TrackMomentPayload>,
) -> Result<HttpResponse> {
    let track = fil_garde(&state, &perimetre, chemin.into_inner()).await?;
    let ctx = contexte_de(&requete, perimetre.person_id);
    let reglages = service::creer_moment(&state, &ctx, track, corps.into_inner()).await?;
    Ok(HttpResponse::Ok().json(reglages))
}

/// Modifier un moment.
#[utoipa::path(
    put,
    description = "`TrackMomentPayload` → `TrackPageSettings`. Écriture totale du moment. Un moment d'un autre fil est introuvable.",
    path = "/admin/tracks/{id}/moments/{momentId}",
    tag = "Back-office — événements",
    operation_id = "admin_fil_moment_modifier",
    params(
        ("id" = uuid::Uuid, Path, description = "Identifiant du fil"),
        ("momentId" = uuid::Uuid, Path, description = "Identifiant du moment"),
    ),
    request_body = Object,
    responses(
        (status = 200, description = "TrackPageSettings", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente, ou périmètre d'administration vide", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Fil ou moment inexistants **ou hors périmètre**, indiscernables", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Libellé absent, fin avant le début, partie d'un autre fil", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn modifier_moment(
    requete: HttpRequest,
    state: web::Data<EventState>,
    perimetre: Perimeter,
    chemin: web::Path<(Uuid, Uuid)>,
    corps: web::Json<TrackMomentPayload>,
) -> Result<HttpResponse> {
    let (id, moment) = chemin.into_inner();
    let track = fil_garde(&state, &perimetre, id).await?;
    let ctx = contexte_de(&requete, perimetre.person_id);
    let reglages =
        service::modifier_moment(&state, &ctx, track, moment, corps.into_inner()).await?;
    Ok(HttpResponse::Ok().json(reglages))
}

/// Retirer un moment.
#[utoipa::path(
    delete,
    description = "`TrackPageSettings`.",
    path = "/admin/tracks/{id}/moments/{momentId}",
    tag = "Back-office — événements",
    operation_id = "admin_fil_moment_supprimer",
    params(
        ("id" = uuid::Uuid, Path, description = "Identifiant du fil"),
        ("momentId" = uuid::Uuid, Path, description = "Identifiant du moment"),
    ),
    responses(
        (status = 200, description = "TrackPageSettings", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente, ou périmètre d'administration vide", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Fil ou moment inexistants **ou hors périmètre**, indiscernables", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn supprimer_moment(
    requete: HttpRequest,
    state: web::Data<EventState>,
    perimetre: Perimeter,
    chemin: web::Path<(Uuid, Uuid)>,
) -> Result<HttpResponse> {
    let (id, moment) = chemin.into_inner();
    let track = fil_garde(&state, &perimetre, id).await?;
    let ctx = contexte_de(&requete, perimetre.person_id);
    let reglages = service::supprimer_moment(&state, &ctx, track, moment).await?;
    Ok(HttpResponse::Ok().json(reglages))
}
