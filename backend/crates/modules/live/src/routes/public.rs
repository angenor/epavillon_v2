//! La lecture publique — **un bandeau d'incident est public par nature**.
//!
//! Aucune garde, aucune session : protéger ce texte reviendrait à protéger ce
//! qui est précisément fait pour être vu de qui regarde la programmation.
//!
//! **Une édition inconnue rend une liste vide, jamais 404.** C'est délibéré :
//! cette route ne doit pas devenir un moyen de savoir si une édition existe, et
//! un bandeau absent se lit exactement comme une édition sans incident — ce qui
//! est le cas normal.
//!
//! **Route plate, aucun scope.** Le module `event` déclare ses routes `/events/…`
//! à plat, et `/events/{slug}` porte un segment là où celle-ci en porte deux :
//! les motifs ne se recouvrent pas.

use actix_web::{web, HttpResponse};
use kernel::error::Result;
use uuid::Uuid;

use crate::repo::active;
use crate::state::LiveState;

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/events/{event_id}/incidents", web::get().to(actifs));
}

#[utoipa::path(
    get,
    description = "`ActiveIncident[]` — les messages actifs de l'édition, **le plus grave en tête**, dans l'ordre où `live.active_incidents_for_event()` les rend. Les cinq portées y remontent : édition, journée, activité, organisation **qui y anime**, et les messages globaux.\n\n**Chaque ligne porte `target_label` déjà résolu** par le modèle — « Atelier de négociation », « Journée finance », le nom légal d'une organisation : le bandeau nomme son sujet, et un message de portée `session` reste lisible sur une page qui parle de trente activités.\n\n**Aucune garde**, et **jamais 404** : une édition inconnue rend une liste vide. Le site n'en affiche que trois, le reste replié en « +N » — c'est la règle des pastilles de la charte ; l'API, elle, rend tout.",
    path = "/events/{event_id}/incidents",
    tag = "Direct",
    operation_id = "evenement_incidents_actifs",
    params(("event_id" = Uuid, Path, description = "Édition affichée")),
    responses((status = 200, description = "ActiveIncident[]", body = Object))
)]
pub(crate) async fn actifs(
    state: web::Data<LiveState>,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let lignes = active::pour_ledition(state.pool(), chemin.into_inner()).await?;
    Ok(HttpResponse::Ok().json(lignes))
}
