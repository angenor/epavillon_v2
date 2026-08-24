//! Ce que ce module sert **hors du back-office**.
//!
//! Une seule route pour l'instant : le bandeau d'ouverture de l'accueil. Elle
//! n'exige aucune session — la vitrine est ce que voit un visiteur anonyme en
//! arrivant.

use actix_web::{web, HttpResponse};
use kernel::error::Result;

use crate::domain::showcase::HomeShowcase;
use crate::repo::showcase;
use crate::state::ContentState;

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/home", web::get().to(vitrine));
}

/// La vitrine de l'accueil.
#[utoipa::path(
    get,
    description = "`Pick<HomeScreen, 'hero'>` — les diapositives du bandeau d'ouverture, **dans l'ordre de défilement** (`sort_order`, puis `id`). Le reste de l'accueil est servi par les modules qui en répondent : les éditions par `GET /events/public`, les prochaines séances par `GET /schedule`, et les chiffres du programme voyagent avec chaque ligne d'édition. **Le filtre de diffusion vient du modèle** : `content.v_showcase` ne rend qu'une diapositive publiée et dans sa fenêtre — il n'est recopié dans aucun écran. Tableau **vide** possible : la page d'accueil reste entière et s'ouvre alors sur l'appel à propositions.",
    path = "/home",
    tag = "Vitrine",
    operation_id = "vitrine_de_l_accueil",
    responses((status = 200, description = "Pick<HomeScreen, 'hero'>", body = Object))
)]
pub(crate) async fn vitrine(state: web::Data<ContentState>) -> Result<HttpResponse> {
    let hero = showcase::bandeau(state.pool()).await?;

    Ok(HttpResponse::Ok().json(HomeShowcase { hero }))
}
