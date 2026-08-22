//! La table blanche, telle qu'un écran la lit.
//!
//! **Elle n'est pas gardée par une entité**, parce qu'elle n'en nomme aucune :
//! elle dit ce qu'un rôle attend — un type, un poids, une forme —, pas ce qui a
//! été déposé. Une session suffit.

use actix_web::{web, HttpResponse};
use kernel::auth::Actor;
use kernel::error::Result;
use serde::Deserialize;

use crate::service::attach;
use crate::state::MediaState;

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/roles", web::get().to(roles));
}

#[derive(Debug, Deserialize)]
pub struct EntiteQuery {
    pub owner_schema: String,
    pub owner_table: String,
}

/// Les règles d'une entité.
#[utoipa::path(
    get,
    description = "`AttachableRoleRule[]` **+** `expected_aspect_ratio` et `aspect_ratio_tolerance` — les deux champs que le modèle déclare et que le contrat du front ne porte pas encore. **Sans eux, l'écran ne peut pas annoncer la forme attendue** : il l'apprend par le refus, après que le fichier a traversé le réseau. Leur ajout côté front est inscrit aux obligations de B7.\n\nLe rapport est le quotient largeur ÷ hauteur — `3.5556` pour un 32:9, `1.0000` pour un carré — et il traverse **en texte** : `numeric(6,4)` n'a pas de représentant flottant exact, et un rapport affiché doit l'être tel qu'il est déclaré.\n\n**Les rôles inactifs sont rendus, avec leur drapeau** : les masquer laisserait croire qu'un rôle n'a jamais existé, là où il a été fermé.",
    path = "/media/roles",
    tag = "Média — rattachements",
    operation_id = "media_roles",
    params(
        ("owner_schema" = String, Query, description = "Schéma de l'entité porteuse"),
        ("owner_table" = String, Query, description = "Table de l'entité porteuse"),
    ),
    responses(
        (status = 200, description = "AttachableRoleRule[]", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn roles(
    state: web::Data<MediaState>,
    _acteur: Actor,
    entite: web::Query<EntiteQuery>,
) -> Result<HttpResponse> {
    let regles = attach::roles(&state, &entite.owner_schema, &entite.owner_table).await?;
    Ok(HttpResponse::Ok().json(regles))
}
