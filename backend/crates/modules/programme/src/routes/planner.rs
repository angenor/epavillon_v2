//! L'écran du planificateur — `GET /admin/planner`.
//!
//! **Cette route n'écrit pas son préfixe.** `/admin/planner` est composé **une
//! seule fois dans l'API**, à partir de deux modules : `event` y dépose le
//! contrôle préalable et la publication (B3), ce module y dépose l'écran. Deux
//! `web::scope` du même préfixe **ne se complètent pas** — Actix retient le
//! premier et rend 404 sur les routes du second, et le défaut a coûté trois
//! routes muettes sur vingt et une en B2.

use actix_web::{web, HttpResponse};
use kernel::auth::Perimeter;
use kernel::error::{ApiError, Result};
use serde::Deserialize;
use uuid::Uuid;

use crate::domain::ids::EventId;
use crate::service::{perimeter, planner};
use crate::state::ProgrammeState;

/// L'édition demandée. **Elle est exigée** : rendre « tout le périmètre » à
/// défaut mélangerait deux COP dans une même grille.
#[derive(Debug, Deserialize)]
pub struct EditionDemandee {
    event_id: Uuid,
}

/// Ce que ce module dépose sous `/admin/planner`, **sans le préfixe**.
pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("", web::get().to(ecran));
}

/// Le garde de l'écran : **ascendance, périmètre, puis permission de
/// planifier**, sur l'édition résolue en base.
pub(crate) async fn garder(
    state: &ProgrammeState,
    perimetre: &Perimeter,
    cible: perimeter::Cible,
) -> Result<EventId> {
    let event_id = perimeter::edition_dans_le_perimetre(state.pool(), perimetre, cible).await?;

    kernel::auth::require_permission(
        state.pool(),
        perimetre.person_id,
        crate::domain::permissions::SESSION_SCHEDULE,
        kernel::auth::Scope::Event(event_id.as_uuid()),
    )
    .await?;

    Ok(event_id)
}

/// Tout l'écran d'arbitrage, en une réponse.
#[utoipa::path(
    get,
    description = "`PlannerScreen` — **tout l'écran en une réponse, conflits compris** : le fuseau de l'édition et le nom de sa ville, la date de publication du programme, les jours du calendrier, les salles, les journées spéciales, les canaux de diffusion, les séances **placées**, celles **à placer**, et les chevauchements. Les conflits ne sont pas un second appel : une grille affichée avant de savoir ce qui s'y chevauche montre, pendant une seconde, une programmation qui a l'air saine. Le tout est lu dans **une transaction en lecture seule, sur une connexion** — lus à un autre instant, les conflits décriraient une grille que l'écran n'affiche pas.",
    path = "/admin/planner",
    tag = "Planificateur de séances",
    operation_id = "seances_ecran_du_planificateur",
    params(("event_id" = Uuid, Query, description = "Édition dont on compose la grille")),
    responses(
        (status = 200, description = "PlannerScreen", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission de planifier absente, ou périmètre vide", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Édition inexistante **ou hors périmètre** — indiscernables", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn ecran(
    state: web::Data<ProgrammeState>,
    perimetre: Perimeter,
    demande: web::Query<EditionDemandee>,
) -> Result<HttpResponse> {
    let event_id = garder(
        &state,
        &perimetre,
        perimeter::Cible::Edition(EventId(demande.event_id)),
    )
    .await?;

    let ecran = planner::ecran(state.pool(), event_id)
        .await?
        .ok_or_else(ApiError::not_found)?;

    Ok(HttpResponse::Ok().json(ecran))
}
