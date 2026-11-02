//! `/track-registrations` : l'inscription à une journée (02/11), et sa liste
//! nominative sous `/admin/track-registrations`. Chemins plats.
//!
//! Sans compte, la personne est reconnue par le jeton de son lien de gestion,
//! dans l'en-tête `X-Manage-Token` : jamais dans l'adresse, qui finit dans les
//! journaux.

use actix_web::{web, HttpRequest, HttpResponse};
use kernel::auth::{Actor, Perimeter};
use kernel::context::RequestContext;
use kernel::error::{ApiError, Result};
use serde::Deserialize;
use uuid::Uuid;

use crate::domain::track_registration::{DayRegisterPayload, DaySessionsPayload};
use crate::routes::{contexte_de, locale_de, session_facultative};
use crate::service::track_registration::{self, Demandeur, EN_TETE_JETON};
use crate::state::ProgrammeState;

#[derive(Debug, Deserialize)]
pub struct JourneeDemandee {
    track_id: Uuid,
}

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/track-registrations", web::post().to(sinscrire))
        .route("/track-registrations/form", web::get().to(formulaire))
        .route("/track-registrations/mine", web::get().to(la_mienne))
        .route("/track-registrations/manage", web::get().to(gestion))
        .route(
            "/track-registrations/{id}/sessions",
            web::put().to(modifier),
        )
        .route("/track-registrations/{id}/cancel", web::post().to(annuler))
        .route(
            "/admin/track-registrations",
            web::get().to(liste_nominative),
        );
}

fn jeton(requete: &HttpRequest) -> Option<String> {
    requete
        .headers()
        .get(EN_TETE_JETON)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|j| !j.is_empty() && j.len() <= 128)
        .map(str::to_owned)
}

fn contexte(requete: &HttpRequest, personne: Option<Uuid>) -> RequestContext {
    match personne {
        Some(p) => contexte_de(requete, p),
        None => RequestContext::new(RequestContext::generated_request_id(), locale_de(requete)),
    }
}

/// La personne connectée d'abord, à défaut le jeton ; 401 sans l'un ni l'autre.
fn demandeur(requete: &HttpRequest) -> Result<(Demandeur, Option<Uuid>)> {
    if let Some(p) = session_facultative(requete) {
        return Ok((Demandeur::Personne(p), Some(p)));
    }
    jeton(requete)
        .map(|j| (Demandeur::Jeton(j), None))
        .ok_or_else(ApiError::unauthenticated)
}

#[utoipa::path(
    get,
    description = "`{ form, fields }` : le formulaire d'une journée, celui de son édition, à défaut celui de la plateforme. Lecture publique. **404** pour un fil inconnu, non publié, ou qui n'est ni une journée spéciale ni un atelier.",
    path = "/track-registrations/form",
    tag = "Inscriptions",
    operation_id = "journee_formulaire",
    params(("track_id" = Uuid, Query, description = "Journée")),
    responses(
        (status = 200, description = "Formulaire et champs actifs", body = Object),
        (status = 404, description = "Journée inconnue ou non publiée", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn formulaire(
    state: web::Data<ProgrammeState>,
    demande: web::Query<JourneeDemandee>,
) -> Result<HttpResponse> {
    let formulaire = track_registration::formulaire(&state, demande.track_id).await?;
    Ok(HttpResponse::Ok().json(formulaire))
}

#[utoipa::path(
    post,
    description = "`DayRegisterPayload` → `DayRegistrationResult`. S'inscrire à une journée : un seul direct pour toute la journée, moments compris ; les activités gardées (`session_ids`) servent aux rappels et aux statistiques, elles ne changent rien à l'accès. Trois issues en 200 : inscrit, déjà inscrit, journée passée. **Sans session**, l'inscription n'aboutit que si le formulaire de l'édition admet l'anonyme ; la personne reçoit alors par courriel le lien du direct et un lien pour changer ses activités ou se désinscrire. Déjà inscrite sans compte : rien n'est rendu, le lien repart par courriel.",
    path = "/track-registrations",
    tag = "Inscriptions",
    operation_id = "journee_sinscrire",
    request_body = DayRegisterPayload,
    responses(
        (status = 200, description = "DayRegistrationResult", body = Object),
        (status = 401, description = "REGISTRATION_ACCOUNT_REQUIRED : le formulaire n'admet pas l'inscription sans compte", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Journée inconnue ou non publiée", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Réponse invalide, consentement manquant, activité hors de la journée", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn sinscrire(
    requete: HttpRequest,
    state: web::Data<ProgrammeState>,
    corps: web::Json<DayRegisterPayload>,
) -> Result<HttpResponse> {
    let lecteur = session_facultative(&requete);
    let ctx = contexte(&requete, lecteur);
    let ip = requete.peer_addr().map(|a| a.ip());
    let issue =
        track_registration::sinscrire(&state, &ctx, lecteur, ip, corps.into_inner()).await?;
    Ok(HttpResponse::Ok().json(issue))
}

#[utoipa::path(
    get,
    description = "`DayRegistration` : l'inscription vivante de la personne connectée à cette journée. **404** sans inscription. Réponse `Cache-Control: private, no-store`.",
    path = "/track-registrations/mine",
    tag = "Inscriptions",
    operation_id = "journee_la_mienne",
    params(("track_id" = Uuid, Query, description = "Journée")),
    responses(
        (status = 200, description = "DayRegistration", body = Object),
        (status = 401, description = "Aucune session", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Pas inscrite", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn la_mienne(
    state: web::Data<ProgrammeState>,
    acteur: Actor,
    demande: web::Query<JourneeDemandee>,
) -> Result<HttpResponse> {
    let inscription = track_registration::la_mienne(&state, demande.track_id, acteur.0).await?;
    Ok(HttpResponse::Ok()
        .insert_header(("Cache-Control", "private, no-store"))
        .json(inscription))
}

#[utoipa::path(
    get,
    description = "`DayRegistrationManage` : l'inscription du porteur du lien de gestion (en-tête `X-Manage-Token`), avec de quoi nommer la journée. **404** pour un jeton inconnu ou remplacé par un envoi plus récent. Réponse `Cache-Control: private, no-store`.",
    path = "/track-registrations/manage",
    tag = "Inscriptions",
    operation_id = "journee_gestion",
    params(("X-Manage-Token" = String, Header, description = "Jeton du lien reçu par courriel")),
    responses(
        (status = 200, description = "DayRegistrationManage", body = Object),
        (status = 401, description = "Aucun jeton", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Jeton inconnu", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn gestion(
    requete: HttpRequest,
    state: web::Data<ProgrammeState>,
) -> Result<HttpResponse> {
    let jeton = jeton(&requete).ok_or_else(ApiError::unauthenticated)?;
    let gestion = track_registration::gestion(&state, &jeton).await?;
    Ok(HttpResponse::Ok()
        .insert_header(("Cache-Control", "private, no-store"))
        .json(gestion))
}

#[utoipa::path(
    put,
    description = "`DaySessionsPayload` → `DayRegistration` : la nouvelle liste des activités gardées ; les autres sont retirées, sans courriel. Par la personne inscrite (session) ou le porteur du lien de gestion (`X-Manage-Token`). Refus : `REGISTRATION_LOCKED` (inscription annulée), 422 pour une activité hors de la journée, **404** pour l'inscription d'autrui.",
    path = "/track-registrations/{id}/sessions",
    tag = "Inscriptions",
    operation_id = "journee_modifier",
    params(("id" = Uuid, Path, description = "Inscription à la journée")),
    request_body = DaySessionsPayload,
    responses(
        (status = 200, description = "DayRegistration", body = Object),
        (status = 401, description = "Ni session ni jeton", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Inscription inconnue, ou celle d'autrui", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "REGISTRATION_LOCKED ou activité hors de la journée", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn modifier(
    requete: HttpRequest,
    state: web::Data<ProgrammeState>,
    id: web::Path<Uuid>,
    corps: web::Json<DaySessionsPayload>,
) -> Result<HttpResponse> {
    let (qui, personne) = demandeur(&requete)?;
    let ctx = contexte(&requete, personne);
    let inscription =
        track_registration::modifier(&state, &ctx, id.into_inner(), qui, &corps.session_ids)
            .await?;
    Ok(HttpResponse::Ok().json(inscription))
}

#[utoipa::path(
    post,
    description = "`DayRegistration` : se désinscrire de la journée ; ses activités gardées sont retirées avec elle. Par la personne inscrite ou le porteur du lien de gestion. Rejouable : une inscription déjà annulée est rendue telle quelle.",
    path = "/track-registrations/{id}/cancel",
    tag = "Inscriptions",
    operation_id = "journee_annuler",
    params(("id" = Uuid, Path, description = "Inscription à la journée")),
    responses(
        (status = 200, description = "DayRegistration", body = Object),
        (status = 401, description = "Ni session ni jeton", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Inscription inconnue, ou celle d'autrui", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn annuler(
    requete: HttpRequest,
    state: web::Data<ProgrammeState>,
    id: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let (qui, personne) = demandeur(&requete)?;
    let ctx = contexte(&requete, personne);
    let inscription = track_registration::annuler(&state, &ctx, id.into_inner(), qui).await?;
    Ok(HttpResponse::Ok().json(inscription))
}

#[utoipa::path(
    get,
    description = "`DayRegistrationRow[]` : les inscrits d'une journée, avec leur nombre d'activités gardées. `programme.registration.manage` sur l'édition de la journée, dans le périmètre d'administration.",
    path = "/admin/track-registrations",
    tag = "Inscriptions",
    operation_id = "journee_liste_nominative",
    params(("track_id" = Uuid, Query, description = "Journée")),
    responses(
        (status = 200, description = "DayRegistrationRow[]", body = Object),
        (status = 401, description = "Aucune session", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Journée inconnue ou hors périmètre", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn liste_nominative(
    state: web::Data<ProgrammeState>,
    perimetre: Perimeter,
    demande: web::Query<JourneeDemandee>,
) -> Result<HttpResponse> {
    let lignes = track_registration::liste_nominative(&state, &perimetre, demande.track_id).await?;
    Ok(HttpResponse::Ok().json(lignes))
}
