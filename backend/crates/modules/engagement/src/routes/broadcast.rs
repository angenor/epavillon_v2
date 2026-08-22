//! La diffusion d'une annonce de plateforme.

use actix_web::{web, HttpRequest, HttpResponse};
use kernel::auth::Actor;
use kernel::error::Result;

use crate::routes::contexte_de;
use crate::service::notifications::{self, BroadcastPayload};
use crate::state::EngagementState;

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/admin/notifications/broadcast", web::post().to(diffuser));
}

/// Diffuser une annonce.
#[utoipa::path(
    post,
    description = "`{ recipients, emailed }` — une notification par destinataire, **groupée par clé** : une même diffusion relayée deux fois n'écrit qu'une ligne par personne.\n\n**Deux audiences et pas une de plus** : toute la plateforme, ou les inscrits d'une édition. Une troisième — « les référents d'organisation », « les négociateurs » — demanderait une définition que rien ne porte aujourd'hui, et l'inventer produirait une liste que personne n'aurait validée.\n\n**Chaque canal est consulté séparément** : qui a coupé le courriel garde l'avis à l'écran, et inversement. L'annonce est de criticité basse — elle se coupe, contrairement à une alerte de sécurité.\n\n`link_path` est un **chemin relatif** : un nom d'hôte de préproduction ne doit pas entrer en base.\n\n**L'expédition est faite dans la requête**, sans travail différé : aucune des cinq tâches du jalon n'en prévoit un, et le geste se fait quelques fois par an.",
    path = "/admin/notifications/broadcast",
    tag = "Notifications",
    operation_id = "engagement_diffuser_une_annonce",
    request_body = BroadcastPayload,
    responses(
        (status = 200, description = "{ recipients, emailed }", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission de diffuser absente", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "VALIDATION_FAILED", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn diffuser(
    state: web::Data<EngagementState>,
    Actor(acteur): Actor,
    requete: HttpRequest,
    payload: web::Json<BroadcastPayload>,
) -> Result<HttpResponse> {
    let ctx = contexte_de(&requete, acteur);
    Ok(HttpResponse::Ok().json(notifications::diffuser(&state, &ctx, acteur, &payload).await?))
}
