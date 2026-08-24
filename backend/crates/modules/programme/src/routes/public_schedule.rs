//! La programmation publique — **deux routes, aucune session exigée**.
//!
//! Elles ne prennent aucun périmètre et ne gardent rien : une programmation
//! publiée est publique. C'est le pendant, côté séances, des éditions publiques
//! de B3.

use actix_web::{web, HttpResponse};
use kernel::error::Result;
use serde::Deserialize;
use uuid::Uuid;

use crate::domain::ids::EventId;
use crate::service::public_schedule;
use crate::state::ProgrammeState;

#[derive(Debug, Deserialize)]
pub struct EditionDemandee {
    /// **Facultative.** Absente, ce sont les séances à venir de toutes les
    /// éditions — ce que compose l'accueil du site, qui n'a pas d'édition à
    /// nommer.
    event_id: Option<Uuid>,
    limit: Option<i64>,
}

/// Plafond de la lecture sans édition. Assez large pour que l'accueil compose
/// sa liste, assez étroit pour qu'une page publique ne puisse pas demander la
/// programmation entière de toutes les COP.
const LIMITE_PAR_DEFAUT: i64 = 50;
const LIMITE_MAX: i64 = 200;

/// Les deux chemins publics. **Ils ne vivent pas sous `/sessions`** : le premier
/// est `/schedule`, le second passe par l'édition et l'adresse d'URL de la
/// séance — c'est ainsi que le site les écrit.
pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/schedule", web::get().to(programmation))
        .route("/events/{event_id}/sessions/{slug}", web::get().to(seance));
}

/// La programmation d'une édition.
#[utoipa::path(
    get,
    description = "`PublicScheduleRow[]` — `programme.v_public_schedule`, **telle quelle**, et **sans session**. Une ligne = un bloc du calendrier : salle, organisation avec son sigle et son pays, journées spéciales, thématiques avec libellé et couleur, image de couverture — celle de la séance, **à défaut celle du dossier d'origine** —, état temporel calculé en base, nombre d'inscrits. Une édition dont le programme n'est pas paru rend une liste **vide**, jamais une erreur. **`event_id` est facultative** : absente, ce sont les séances `upcoming` et `ongoing` de TOUTES les éditions, dans l'ordre du temps — ce que compose l'accueil, qui n'a pas d'édition à nommer. La lecture est alors plafonnée.",
    path = "/schedule",
    tag = "Programmation publique",
    operation_id = "programmation_publique",
    params(
        ("event_id" = Option<Uuid>, Query, description = "Édition dont on lit le programme. Absente : les séances à venir de toutes les éditions"),
        ("limit" = Option<i64>, Query, description = "Plafond du nombre de lignes (défaut 50, maximum 200)"),
    ),
    responses((status = 200, description = "PublicScheduleRow[]", body = Object))
)]
pub(crate) async fn programmation(
    state: web::Data<ProgrammeState>,
    demande: web::Query<EditionDemandee>,
) -> Result<HttpResponse> {
    // Le plafond ne s'applique qu'à la lecture SANS édition : la programmation
    // d'une édition se rend entière, c'est un calendrier qu'on affiche en bloc.
    let limite = match (demande.event_id, demande.limit) {
        (Some(_), aucune) => aucune,
        (None, choisie) => Some(choisie.unwrap_or(LIMITE_PAR_DEFAUT).clamp(1, LIMITE_MAX)),
    };
    let lignes =
        public_schedule::programmation(state.pool(), demande.event_id.map(EventId), limite).await?;

    Ok(HttpResponse::Ok().json(lignes))
}

/// Le détail d'une séance publiée.
#[utoipa::path(
    get,
    description = "`{ session, speakers, organizations }` — la séance **publiée** désignée par son adresse d'URL dans son édition, avec ses intervenants et ses organisations. **Une adresse inconnue et une séance non publiée rendent le même 404** : distinguer les deux dirait au public qu'une séance existe sans être encore annoncée.",
    path = "/events/{event_id}/sessions/{slug}",
    tag = "Programmation publique",
    operation_id = "programmation_seance_publique",
    params(
        ("event_id" = Uuid, Path, description = "Édition"),
        ("slug" = String, Path, description = "Adresse d'URL de la séance"),
    ),
    responses(
        (status = 200, description = "Séance publiée, intervenants et organisations", body = Object),
        (status = 404, description = "Adresse inconnue **ou séance non publiée** — indiscernables", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn seance(
    state: web::Data<ProgrammeState>,
    chemin: web::Path<(Uuid, String)>,
) -> Result<HttpResponse> {
    let (event_id, slug) = chemin.into_inner();
    let seance = public_schedule::seance(state.pool(), EventId(event_id), &slug).await?;

    Ok(HttpResponse::Ok().json(seance))
}
