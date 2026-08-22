//! **L'ingestion des retours du fournisseur** — hors session, et fermée par
//! défaut.
//!
//! # Deux jetons et non un
//!
//! `MAIL_RELAY_TOKEN` est le secret de **sortie** : l'API se fait reconnaître du
//! site quand elle lui remet un message. `MAIL_WEBHOOK_TOKEN` est celui
//! d'**entrée** : le site se fait reconnaître de l'API quand il remonte ce que
//! le fournisseur a dit d'un courriel. Les confondre ferait d'un jeton de sortie
//! un jeton d'entrée, donc ouvrirait la porte à qui a seulement lu la
//! configuration du relais (R30).
//!
//! # Le jeton absent ferme la route, il ne l'ouvre pas
//!
//! Elle n'est **pas montée** : elle rend 404, comme un module éteint. Une porte
//! d'ingestion sans secret vaut mieux fermée — et un défaut de configuration se
//! remarque bien plus vite quand la route disparaît que quand elle accepte tout.

use actix_web::{web, HttpRequest, HttpResponse};
use kernel::error::{ApiError, Result};
use kernel::RequestContext;

use crate::service::deliverability::{self, MailEvent};
use crate::state::EngagementState;

/// Monte la route **si et seulement si** le jeton est configuré.
pub fn configurer(cfg: &mut web::ServiceConfig, jeton_configure: bool) {
    if jeton_configure {
        cfg.route("/internal/mail-events", web::post().to(ingerer));
    }
}

/// Ingérer un lot d'annonces.
#[utoipa::path(
    post,
    description = "`{ applied, ignored }` — les annonces de remise, de rebond et de plainte du fournisseur mettent la trace d'expédition à jour.\n\n**Une annonce rejouée est IGNORÉE, jamais dupliquée** : le fournisseur rejoue volontiers, et rendre une erreur le ferait recommencer sans fin. Une annonce dont la trace est introuvable est ignorée de la même façon — l'identifiant du fournisseur est la seule chose qui les relie, et une trace effacée par la purge de partition n'est pas un incident.\n\n**Un rebond dur ou une plainte inscrivent l'adresse sur la liste de suppression** : c'est le seul geste qui protège la réputation du domaine sans intervention humaine. Un rebond souple, non — une boîte pleine n'est pas une adresse morte.\n\n**Hors session**, authentifiée par un jeton porteur. Non montée si le jeton n'est pas configuré : elle rend alors 404, comme un module éteint.",
    path = "/internal/mail-events",
    tag = "Délivrabilité",
    operation_id = "engagement_ingerer_les_retours_de_courriel",
    request_body = Vec<MailEvent>,
    responses(
        (status = 200, description = "{ applied, ignored }", body = Object),
        (status = 401, description = "Jeton porteur absent ou faux", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Jeton non configuré : la route n'est pas montée", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("bearer" = []))
)]
pub(crate) async fn ingerer(
    state: web::Data<EngagementState>,
    requete: HttpRequest,
    payload: web::Json<Vec<MailEvent>>,
) -> Result<HttpResponse> {
    exiger_le_jeton(&state, &requete)?;

    // Le contexte n'a pas d'acteur : personne n'a agi, un fournisseur a parlé.
    let ctx = RequestContext::background("engagement.mail-events");
    Ok(HttpResponse::Ok().json(deliverability::ingerer(&state, &ctx, &payload).await?))
}

/// **La comparaison est à temps constant** : un jeton comparé octet par octet
/// se devine, un caractère à la fois, en mesurant le temps de réponse.
fn exiger_le_jeton(state: &EngagementState, requete: &HttpRequest) -> Result<()> {
    let attendu = state
        .config()
        .mail
        .webhook_token
        .as_ref()
        .ok_or_else(ApiError::not_found)?;

    let presente = requete
        .headers()
        .get(actix_web::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or_default();

    if kernel::crypto::constant_time_eq(presente, attendu.expose()) {
        Ok(())
    } else {
        Err(ApiError::unauthenticated())
    }
}
