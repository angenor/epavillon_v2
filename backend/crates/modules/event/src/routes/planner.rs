//! Le contrôle préalable et la publication de la programmation.
//!
//! **Ces deux routes n'écrivent pas leur préfixe.** `/admin/planner` sera aussi
//! celui du module Programmation en B5 : le scope est composé **une seule fois
//! dans l'API**, chaque module y versant ses routes. Deux `web::scope` du même
//! préfixe **ne se complètent pas** — Actix retient le premier et rend 404 sur
//! les routes du second. Le défaut a coûté trois routes muettes sur vingt et une
//! en B2 ; l'écrire correctement du premier coup coûte cinq lignes (research.md
//! § R11).
//!
//! **Elles sont gardées par la permission de PLANIFIER**, `programme.session.
//! schedule`, et non par celle de gérer les événements. Le modèle décrit le rôle
//! chargé de la programmation comme celui qui « planifie les créneaux **et
//! publie la programmation** » : le garder autrement empêcherait un chargé de
//! programmation de publier ce que son rôle dit qu'il publie. Une permission est
//! une **chaîne lue en base**, pas un symbole d'un autre crate : `cargo tree -p
//! event` reste sans arête (research.md § R12).

use actix_web::{web, HttpRequest, HttpResponse};
use kernel::auth::{Perimeter, Scope};
use kernel::error::Result;
use serde::Deserialize;
use uuid::Uuid;

use crate::domain::ids::EventId;
use crate::domain::permissions::SESSION_SCHEDULE;
use crate::routes::contexte_de;
use crate::service::{edition_dans_le_perimetre, publication, Cible};
use crate::state::EventState;

/// L'édition visée. Le contrôle préalable la reçoit en paramètre d'adresse, la
/// publication dans son corps — c'est ainsi que le front les appelle.
#[derive(Debug, Deserialize)]
pub struct CibleDePublication {
    pub event_id: Uuid,
}

/// Ce que ce module dépose sous `/admin/planner`, **sans le préfixe**.
pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/readiness", web::get().to(controle))
        .route("/publish", web::post().to(publier));
}

/// Le garde des deux routes : **ascendance, périmètre, puis permission de
/// planifier**.
async fn garder(state: &EventState, perimetre: &Perimeter, event_id: Uuid) -> Result<EventId> {
    let event_id = edition_dans_le_perimetre(
        state.pool(),
        perimetre,
        Cible::Edition(EventId::from(event_id)),
    )
    .await?;

    kernel::auth::require_permission(
        state.pool(),
        perimetre.person_id,
        SESSION_SCHEDULE,
        Scope::Event(event_id.as_uuid()),
    )
    .await?;

    Ok(event_id)
}

/// Ce qui doit être réglé avant de publier.
#[utoipa::path(
    get,
    description = "`PublicationReadinessIssue[]` — **lecture seule**, consultable avant toute tentative : l'équipe voit ce qui bloque avant d'essayer, plutôt que de découvrir la liste après un clic. Conflits détectés et manques : séance sans créneau valide, séance sans lieu ni précision de lieu, diffusion sans canal, intervenant absent. **`occurs_at` est un instant, jamais un intervalle mis en forme** — une chaîne figée en base ne peut ni se traduire ni se situer dans le fuseau de l'édition. Seule la gravité `blocking` retient la publication.",
    path = "/admin/planner/readiness",
    tag = "Planificateur",
    operation_id = "planificateur_controle_de_publication",
    params(("event_id" = uuid::Uuid, Query, description = "Identifiant de l'édition")),
    responses(
        (status = 200, description = "PublicationReadinessIssue[]", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission de planifier absente, ou périmètre vide", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Édition inexistante **ou hors périmètre** — indiscernables", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn controle(
    state: web::Data<EventState>,
    perimetre: Perimeter,
    parametres: web::Query<CibleDePublication>,
) -> Result<HttpResponse> {
    let event_id = garder(&state, &perimetre, parametres.event_id).await?;

    let issues = publication::controle(state.pool(), event_id).await?;

    Ok(HttpResponse::Ok().json(issues))
}

/// Publier la programmation — **le seul contrôle bloquant du module**.
#[utoipa::path(
    post,
    description = "`{ event_id }` → `PublishProgrammeResult`. **Le seul contrôle bloquant du module** : un point de gravité `blocking` rend `blocked: true`, **rien n'est écrit**, et la liste dit quoi régler. Les avertissements accompagnent sans retenir. Une publication qui aboutit estampille l'édition, **annonce** par un événement de domaine, et rend `published_count` — un décompte de désignation, pris sous l'instantané de la transaction, avec le prédicat même que l'annonce porte. **Republier est inoffensif** : la date d'origine ne s'écrase pas et aucun second événement n'est émis. Une édition **sans aucune séance publie**, avec zéro séance et une liste vide : ce n'est pas un conflit.",
    path = "/admin/planner/publish",
    tag = "Planificateur",
    operation_id = "planificateur_publier",
    request_body = Object,
    responses(
        (status = 200, description = "PublishProgrammeResult", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission de planifier absente, ou périmètre vide", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Édition inexistante **ou hors périmètre** — indiscernables", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn publier(
    requete: HttpRequest,
    state: web::Data<EventState>,
    perimetre: Perimeter,
    corps: web::Json<CibleDePublication>,
) -> Result<HttpResponse> {
    let event_id = garder(&state, &perimetre, corps.event_id).await?;

    let ctx = contexte_de(&requete, perimetre.person_id);
    let resultat = publication::publier(&state, &ctx, event_id).await?;

    Ok(HttpResponse::Ok().json(resultat))
}
