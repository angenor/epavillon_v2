//! L'accord « Notifications » d'« À propos ».

use actix_web::{web, HttpRequest, HttpResponse};
use kernel::auth::Actor;
use kernel::error::Result;

use crate::domain::notifications::NotificationSettingsPayload;
use crate::service::notifications;
use crate::state::NegotiationState;

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/negotiation/me/notifications", web::get().to(mon_reglage))
        .route("/negotiation/me/notifications", web::put().to(regler));
}

#[utoipa::path(
    get,
    description = "`NotificationSettings` — l'accord « Notifications » : `email` dit si les courriels de changement partent. **Sans accord enregistré, allumé.** `version` est celle de la politique de confidentialité servie (`GET /legal/privacy`). Éteint, l'application prévient toujours ; seul le courriel s'arrête.",
    path = "/negotiation/me/notifications",
    tag = "Guide Négo — notifications",
    operation_id = "negotiation_mon_reglage_de_notifications",
    responses(
        (status = 200, description = "NotificationSettings", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn mon_reglage(
    state: web::Data<NegotiationState>,
    acteur: Actor,
) -> Result<HttpResponse> {
    Ok(HttpResponse::Ok()
        .insert_header(crate::routes::PERSONNEL)
        .json(notifications::reglage(&state, acteur.0).await?))
}

#[utoipa::path(
    put,
    description = "`NotificationSettingsPayload` → `NotificationSettings` — allumer ou éteindre les courriels. Chaque bascule écrit une preuve dans les consentements, avec la version servie ; rejouer la même valeur n'écrit rien.",
    path = "/negotiation/me/notifications",
    tag = "Guide Négo — notifications",
    operation_id = "negotiation_regler_les_notifications",
    request_body = Object,
    responses(
        (status = 200, description = "NotificationSettings", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Corps malformé", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn regler(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    acteur: Actor,
    charge: web::Json<NotificationSettingsPayload>,
) -> Result<HttpResponse> {
    let ctx = crate::routes::contexte_de(&requete, acteur.0);
    Ok(HttpResponse::Ok()
        .insert_header(crate::routes::PERSONNEL)
        .json(notifications::regler(&state, &ctx, acteur.0, charge.email).await?))
}
