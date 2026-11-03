//! Les courriels programmés (04/11) : onglet « Programmés » de Courriels.

use actix_web::{web, HttpRequest, HttpResponse};
use kernel::auth::Actor;
use kernel::error::Result;
use serde::Deserialize;
use uuid::Uuid;

use crate::domain::programmes::{
    ScheduledMailPreviewPayload, ScheduledMailTexts, ScheduledMailTypeTextPayload,
};
use crate::routes::{contexte_de, locale_de};
use crate::service::programmes;
use crate::state::EngagementState;

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/admin/scheduled-mails", web::get().to(lister))
        .route("/admin/scheduled-mails/batches/{key}", web::get().to(envoi))
        .route(
            "/admin/scheduled-mails/batches/{key}/preview",
            web::post().to(apercu),
        )
        .route(
            "/admin/scheduled-mails/batches/{key}/text",
            web::put().to(ecrire_envoi),
        )
        .route(
            "/admin/scheduled-mails/batches/{key}/text",
            web::delete().to(effacer_envoi),
        )
        .route(
            "/admin/scheduled-mails/types/{key}/text",
            web::put().to(ecrire_type),
        );
}

#[derive(Debug, Deserialize)]
pub struct EditionQuery {
    pub event_id: Uuid,
}

/// Les courriels programmés d'une édition.
#[utoipa::path(
    get,
    description = "`ScheduledMailScreen` : les rappels déjà en file et pas encore partis (rappels de journée, rappels d'activité par décalage), regroupés en types puis en envois. Un envoi réunit les destinataires d'un même rappel pour une même cible à la même heure : (séance, décalage) pour une activité, (journée, décalage, début visé) pour une journée. `observed_at` est l'heure du serveur au relevé : le décompte s'y cale. Les titres sont dans la langue de la requête.\n\nGardé par `engagement.mailing.send` **sur l'édition** et le périmètre d'administration : hors périmètre, l'édition n'existe pas (404) ; dans le périmètre sans la permission, 403.",
    path = "/admin/scheduled-mails",
    tag = "Back-office : envois de courriels",
    operation_id = "engagement_programmes_lister",
    params(("event_id" = Uuid, Query, description = "L'édition")),
    responses(
        (status = 200, description = "ScheduledMailScreen", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente sur l'édition", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Édition inexistante ou hors périmètre", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn lister(
    state: web::Data<EngagementState>,
    Actor(acteur): Actor,
    requete: HttpRequest,
    edition: web::Query<EditionQuery>,
) -> Result<HttpResponse> {
    let locale = locale_de(&requete);
    Ok(
        HttpResponse::Ok()
            .json(programmes::ecran(&state, acteur, edition.event_id, &locale).await?),
    )
}

/// Le panneau d'un envoi.
#[utoipa::path(
    get,
    description = "`ScheduledMailBatch` : l'envoi, le nombre de courriels de son type dans l'édition, le texte du type (écrit, sinon celui par défaut) et le texte propre à l'envoi s'il en a un, en français et en anglais, les variables disponibles et la langue du premier destinataire.\n\nLa clé est celle que rend la liste (`activity.<séance>.<décalage>`, `day.<journée>.<décalage>.<début>`). Gardé sur l'édition de la cible : hors droit, ou déjà parti, l'envoi n'existe pas (404).",
    path = "/admin/scheduled-mails/batches/{key}",
    tag = "Back-office : envois de courriels",
    operation_id = "engagement_programmes_envoi",
    params(("key" = String, Path, description = "La clé de l'envoi")),
    responses(
        (status = 200, description = "ScheduledMailBatch", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente sur l'édition", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Envoi inexistant, parti, ou hors périmètre", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn envoi(
    state: web::Data<EngagementState>,
    Actor(acteur): Actor,
    requete: HttpRequest,
    cle: web::Path<String>,
) -> Result<HttpResponse> {
    let locale = locale_de(&requete);
    Ok(HttpResponse::Ok().json(programmes::envoi(&state, acteur, &cle, &locale).await?))
}

/// Ce qui partira, avec le texte en cours d'écriture.
#[utoipa::path(
    post,
    description = "`ScheduledMailPreviewPayload` → `ScheduledMailPreview` : le courriel rendu pour le premier destinataire de l'envoi, dans la langue demandée, par la fonction même du travail d'envoi (gabarit institutionnel et logo de l'édition, bouton, lien en clair). Les valeurs des variables sont surlignées. **N'écrit rien, n'envoie rien.** Une variable que ce rappel ne fournit pas est refusée en la nommant (422).",
    path = "/admin/scheduled-mails/batches/{key}/preview",
    tag = "Back-office : envois de courriels",
    operation_id = "engagement_programmes_apercu",
    params(("key" = String, Path, description = "La clé de l'envoi")),
    request_body = ScheduledMailPreviewPayload,
    responses(
        (status = 200, description = "ScheduledMailPreview", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente sur l'édition", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Envoi inexistant, parti, ou hors périmètre", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "VALIDATION_FAILED", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn apercu(
    state: web::Data<EngagementState>,
    Actor(acteur): Actor,
    cle: web::Path<String>,
    payload: web::Json<ScheduledMailPreviewPayload>,
) -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(programmes::apercu(&state, acteur, &cle, &payload).await?))
}

/// Le texte propre à un envoi.
#[utoipa::path(
    put,
    description = "`ScheduledMailTexts` → `ScheduledMailBatch` : l'objet et le message de cet envoi seulement, en français et en anglais. Il vaut pour tous ses destinataires et garde sa version si le texte du type change ensuite. S'applique aux travaux pas encore partis, **sans changer leur heure**.",
    path = "/admin/scheduled-mails/batches/{key}/text",
    tag = "Back-office : envois de courriels",
    operation_id = "engagement_programmes_texte_envoi",
    params(("key" = String, Path, description = "La clé de l'envoi")),
    request_body = ScheduledMailTexts,
    responses(
        (status = 200, description = "ScheduledMailBatch", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente sur l'édition", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Envoi inexistant, parti, ou hors périmètre", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "VALIDATION_FAILED", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn ecrire_envoi(
    state: web::Data<EngagementState>,
    Actor(acteur): Actor,
    requete: HttpRequest,
    cle: web::Path<String>,
    payload: web::Json<ScheduledMailTexts>,
) -> Result<HttpResponse> {
    let ctx = contexte_de(&requete, acteur);
    Ok(HttpResponse::Ok()
        .json(programmes::ecrire_envoi(&state, &ctx, acteur, &cle, &payload).await?))
}

/// Revenir au texte du type.
#[utoipa::path(
    delete,
    description = "`ScheduledMailBatch` : efface le texte propre à l'envoi ; ses destinataires recevront le texte du type.",
    path = "/admin/scheduled-mails/batches/{key}/text",
    tag = "Back-office : envois de courriels",
    operation_id = "engagement_programmes_texte_envoi_effacer",
    params(("key" = String, Path, description = "La clé de l'envoi")),
    responses(
        (status = 200, description = "ScheduledMailBatch", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente sur l'édition", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Envoi inexistant, parti, ou hors périmètre", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn effacer_envoi(
    state: web::Data<EngagementState>,
    Actor(acteur): Actor,
    requete: HttpRequest,
    cle: web::Path<String>,
) -> Result<HttpResponse> {
    let ctx = contexte_de(&requete, acteur);
    Ok(HttpResponse::Ok().json(programmes::effacer_envoi(&state, &ctx, acteur, &cle).await?))
}

/// Le texte d'un type, pour l'édition.
#[utoipa::path(
    put,
    description = "`ScheduledMailTypeTextPayload` → `ScheduledMailTexts` : l'objet et le message d'un type (`day`, ou `activity.<décalage en minutes>`) pour toute l'édition, en français et en anglais. Changent tous les envois de ce type pas encore partis, sauf ceux qui ont leur propre texte ; aucun ne change d'heure.\n\nGardé par `engagement.mailing.send` sur l'édition du corps et le périmètre d'administration.",
    path = "/admin/scheduled-mails/types/{key}/text",
    tag = "Back-office : envois de courriels",
    operation_id = "engagement_programmes_texte_type",
    params(("key" = String, Path, description = "La clé du type")),
    request_body = ScheduledMailTypeTextPayload,
    responses(
        (status = 200, description = "ScheduledMailTexts", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente sur l'édition", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Type inconnu, édition inexistante ou hors périmètre", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "VALIDATION_FAILED", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn ecrire_type(
    state: web::Data<EngagementState>,
    Actor(acteur): Actor,
    requete: HttpRequest,
    cle: web::Path<String>,
    payload: web::Json<ScheduledMailTypeTextPayload>,
) -> Result<HttpResponse> {
    let ctx = contexte_de(&requete, acteur);
    Ok(HttpResponse::Ok()
        .json(programmes::ecrire_type(&state, &ctx, acteur, &cle, &payload).await?))
}
