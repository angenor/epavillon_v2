//! Le tableau de bord du back-office — **une route**.
//!
//! **Chemin plat, jamais un `web::scope("/admin")`** : le préfixe est partagé
//! avec quatre autres modules, et deux scopes du même préfixe ne se complètent
//! pas.

use actix_web::{web, HttpResponse};
use kernel::auth::Perimeter;
use kernel::error::Result;
use serde::Deserialize;
use uuid::Uuid;

use crate::state::AnalyticsState;

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/admin/dashboard", web::get().to(tableau_de_bord));
}

#[derive(Debug, Deserialize)]
pub struct EcranQuery {
    pub event_id: Uuid,
}

#[utoipa::path(
    get,
    description = "`AdminDashboard` — tout l'écran d'une édition **en une réponse et un instant** : l'édition et son fuseau, son appel, les cinq familles d'alerte, les chiffres, la santé opérationnelle et les messages d'incident actifs.\n\n**Une transaction de lecture, un instant** : `now()` y est constant, et les dix lectures parlent donc du même. C'est la réponse aux « neuf instants de mesure » que le contrat du site interdit.\n\n**Gardée par le périmètre ET par `analytics.dashboard.read` sur l'édition demandée.** Le rôle `programmer` la détient depuis le 27/08 : il lit déjà, écran par écran, tout ce que le tableau de bord agrège — la lui refuser lui retirerait un raccourci, pas un droit.\n\n**Le tableau de bord n'a pas d'issue de contrat** : il s'ouvre, ou il se refuse. Périmètre vide ou permission absente → 403 ; édition hors périmètre → 404, **jamais 403**.",
    path = "/admin/dashboard",
    tag = "Back-office — tableau de bord",
    operation_id = "admin_tableau_de_bord",
    params(("event_id" = Uuid, Query, description = "Édition mesurée")),
    responses(
        (status = 200, description = "AdminDashboard", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Périmètre d'administration vide, ou `analytics.dashboard.read` absente sur l'édition", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Édition inexistante **ou hors périmètre** — indiscernables", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn tableau_de_bord(
    state: web::Data<AnalyticsState>,
    perimetre: Perimeter,
    query: web::Query<EcranQuery>,
) -> Result<HttpResponse> {
    let ecran = crate::service::dashboard::ecran(&state, &perimetre, query.event_id).await?;
    Ok(HttpResponse::Ok().json(ecran))
}
