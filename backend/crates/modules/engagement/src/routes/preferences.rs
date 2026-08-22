//! Les préférences de notification : lire et écrire.

use actix_web::{web, HttpRequest, HttpResponse};
use kernel::auth::Actor;
use kernel::error::Result;

use crate::domain::notification::NotificationPreferencePayload;
use crate::routes::{contexte_de, locale_de};
use crate::service::notifications;
use crate::state::EngagementState;

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/notification-preferences", web::get().to(lire))
        .route("/notification-preferences", web::put().to(ecrire));
}

/// Les préférences, catalogue compris.
#[utoipa::path(
    get,
    description = "`NotificationPreferenceRow[]` — **le catalogue croisé avec les arbitrages**, canal par canal, jamais les seuls arbitrages : une liste vide ferait croire qu'aucun avis n'est servi, alors que l'absence de ligne signifie « les canaux par défaut du type ».\n\n**`is_overridable` est le champ qui compte.** Une préférence posée sur un type **critique** — sécurité du compte, annulation de séance — est enregistrée telle quelle, mais elle n'oppose rien. Sans ce champ, l'écran afficherait un interrupteur éteint pour un avis qui part quand même, et la personne croirait s'être désabonnée.",
    path = "/notification-preferences",
    tag = "Notifications",
    operation_id = "engagement_preferences_de_notification",
    responses(
        (status = 200, description = "NotificationPreferenceRow[]", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn lire(
    state: web::Data<EngagementState>,
    Actor(acteur): Actor,
    requete: HttpRequest,
) -> Result<HttpResponse> {
    let locale = locale_de(&requete);
    Ok(HttpResponse::Ok().json(notifications::preferences(&state, acteur, &locale).await?))
}

/// Écrire un lot d'arbitrages.
#[utoipa::path(
    put,
    description = "`NotificationPreferenceRow[]` — le lot est écrit, et **la liste entière est rendue** : l'écran affiche l'état d'après sans second appel, et une préférence sans effet se voit immédiatement.\n\n**Une préférence sur un type critique est enregistrée**, jamais refusée : refuser laisserait l'écran sans réponse à donner, et l'interrupteur reviendrait à sa position sans explication. C'est la lecture qui dit qu'elle n'oppose rien.\n\nUn **type inconnu** est refusé ici, alors que l'envoi le refuse en silence : une ligne orpheline ne serait jamais relue, et la personne croirait avoir coupé quelque chose.",
    path = "/notification-preferences",
    tag = "Notifications",
    operation_id = "engagement_ecrire_preferences_de_notification",
    request_body = Vec<NotificationPreferencePayload>,
    responses(
        (status = 200, description = "NotificationPreferenceRow[]", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "VALIDATION_FAILED · ENGAGEMENT_NOTIFICATION_TYPE_UNKNOWN", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn ecrire(
    state: web::Data<EngagementState>,
    Actor(acteur): Actor,
    requete: HttpRequest,
    payload: web::Json<Vec<NotificationPreferencePayload>>,
) -> Result<HttpResponse> {
    let locale = locale_de(&requete);
    let ctx = contexte_de(&requete, acteur);
    let lignes =
        notifications::ecrire_les_preferences(&state, &ctx, acteur, &locale, &payload).await?;
    Ok(HttpResponse::Ok().json(lignes))
}
