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
use serde::Deserialize;
use uuid::Uuid;

use crate::repo::{active, streams};
use crate::state::LiveState;

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/events/{event_id}/incidents", web::get().to(actifs));
}

/// Sous le scope `/sessions`, que l'API compose avec `programme` et `engagement`.
pub fn chemins_de_seance(cfg: &mut web::ServiceConfig) {
    cfg.route("/{session_id}/streams", web::get().to(flux))
        .route(
            "/{session_id}/incidents",
            web::get().to(actifs_de_la_seance),
        );
}

#[derive(Debug, Deserialize)]
pub struct FiltreFil {
    pub track_id: Option<Uuid>,
}

#[utoipa::path(
    get,
    description = "`ActiveIncident[]` : les messages actifs de l'édition, **le plus grave en tête**, dans l'ordre où `live.active_incidents_for_event()` les rend. Les cinq portées y remontent : édition, journée, activité, organisation **qui y anime**, et les messages globaux.\n\nAvec `track_id` (page d'une journée spéciale) : les messages globaux et de l'édition, plus ceux qui visent une activité publiée du fil, sa journée ou son organisation porteuse (`live.active_incidents_for_sessions()`).\n\n**Chaque ligne porte `target_label` déjà résolu** par le modèle (« Atelier de négociation », « Journée finance », le nom légal d'une organisation) : le bandeau nomme son sujet, et un message de portée `session` reste lisible sur une page qui parle de trente activités.\n\n**Aucune garde**, et **jamais 404** : une édition inconnue rend une liste vide. Le site n'en affiche que trois, le reste replié en « +N », la règle des pastilles de la charte ; l'API, elle, rend tout.",
    path = "/events/{event_id}/incidents",
    tag = "Direct",
    operation_id = "evenement_incidents_actifs",
    params(
        ("event_id" = Uuid, Path, description = "Édition affichée"),
        ("track_id" = Option<Uuid>, Query, description = "Fil (journée spéciale) dont on affiche la page"),
    ),
    responses((status = 200, description = "ActiveIncident[]", body = Object))
)]
pub(crate) async fn actifs(
    state: web::Data<LiveState>,
    chemin: web::Path<Uuid>,
    filtre: web::Query<FiltreFil>,
) -> Result<HttpResponse> {
    let event_id = chemin.into_inner();
    let lignes = match filtre.track_id {
        Some(track_id) => active::pour_le_fil(state.pool(), event_id, track_id).await?,
        None => active::pour_ledition(state.pool(), event_id).await?,
    };
    Ok(HttpResponse::Ok().json(lignes))
}

#[utoipa::path(
    get,
    description = "`ActiveIncident[]` : les messages actifs qui concernent l'activité, **le plus grave en tête** : globaux, de son édition, de sa journée, d'elle-même, de son organisation porteuse (`live.active_incidents_for_sessions()`). Page d'une activité et sa salle.\n\n**Aucune garde**, et **jamais 404** : une activité inconnue ou non publiée rend une liste vide.",
    path = "/sessions/{session_id}/incidents",
    tag = "Direct",
    operation_id = "seance_incidents_actifs",
    params(("session_id" = Uuid, Path, description = "Activité affichée")),
    responses((status = 200, description = "ActiveIncident[]", body = Object))
)]
pub(crate) async fn actifs_de_la_seance(
    state: web::Data<LiveState>,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let lignes = active::pour_la_seance(state.pool(), chemin.into_inner()).await?;
    Ok(HttpResponse::Ok().json(lignes))
}

#[utoipa::path(
    get,
    description = "`PublicSessionStream[]` — les flux **en cours** de la séance, lus dans `live.current_streams` : un par langue d'interprétation, **le flux principal en tête**, puis par langue. `embed_url` est construite par le modèle (`live.build_embed_url()`), à défaut `watch_url`.\n\n**Liste vide** quand rien n'est diffusé. **Aucune garde.** Une séance inconnue et une séance non publiée rendent **le même 404** : distinguer les deux dirait qu'une séance existe sans être encore annoncée.",
    path = "/sessions/{session_id}/streams",
    tag = "Direct",
    operation_id = "seance_flux_en_cours",
    params(("session_id" = Uuid, Path, description = "Séance publiée")),
    responses(
        (status = 200, description = "PublicSessionStream[]", body = Object),
        (status = 404, description = "Séance inconnue **ou non publiée** — indiscernables", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn flux(
    state: web::Data<LiveState>,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let lignes = streams::en_cours(state.pool(), chemin.into_inner()).await?;
    Ok(HttpResponse::Ok().json(lignes))
}
