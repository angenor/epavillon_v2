//! Le scope `/sessions` — la grille, ses écritures, et la fiche d'une séance.
//!
//! # Chemins littéraux avant chemins paramétrés
//!
//! `/sessions/conflicts` (deux segments) et `/sessions/{id}/…` (trois) ne se
//! recouvrent pas : le risque de capture n'existe que lorsque les méthodes **et**
//! le nombre de segments coïncident, comme B4 l'a mesuré. Le découpage est repris
//! quand même, pour que la règle soit tenue par la structure plutôt que par la
//! vigilance.
//!
//! # 🔴 Aucune de ces écritures ne refuse un chevauchement
//!
//! `PlannerMutationResult` ne porte aucun discriminant de refus. Poser deux
//! séances sur le même créneau **réussit**, et la réponse porte le conflit.

use actix_web::{web, HttpRequest, HttpResponse};
use kernel::auth::Perimeter;
use kernel::error::Result;
use serde::Deserialize;
use uuid::Uuid;

use crate::domain::ids::{EventId, SessionId};
use crate::routes::contexte_de;
use crate::routes::planner::garder;
use crate::service::perimeter::Cible;
use crate::service::planner::{
    self, ScheduleSessionPayload, SessionBroadcastPayload, SessionTracksPayload,
};
use crate::state::ProgrammeState;

#[derive(Debug, Deserialize)]
pub struct EditionDemandee {
    event_id: Uuid,
}

/// Les deux lectures d'édition. **Déclarées avant les chemins de séance.**
pub fn chemins_litteraux(cfg: &mut web::ServiceConfig) {
    cfg.route("", web::get().to(liste))
        .route("/conflicts", web::get().to(conflits));
}

/// Ce qui vise **une** séance, désignée par son identifiant.
pub fn chemins_de_seance(cfg: &mut web::ServiceConfig) {
    cfg.route("/{id}/speakers", web::get().to(intervenants))
        .route("/{id}/organizations", web::get().to(organisations))
        .route("/{id}/tracks", web::get().to(fils))
        .route("/{id}/registration-form", web::get().to(formulaire))
        .route("/{id}/registrations", web::post().to(sinscrire))
        .route("/{id}/schedule", web::put().to(placer))
        .route("/{id}/tracks", web::put().to(rattacher))
        .route("/{id}/broadcast", web::put().to(diffuser));
}

// -----------------------------------------------------------------------------
// Les lectures du planificateur
// -----------------------------------------------------------------------------

/// Les séances d'une édition, placées ou non.
#[utoipa::path(
    get,
    description = "`PlannerSession[]` — les séances de l'édition, **placées ou non**. Une séance dont la salle est nulle est au panneau « à placer » ; c'est la seule chose qui l'y range, et elle existe bel et bien. Chaque ligne porte tout ce qu'un bloc affiche, **déjà joint** : salle, organisation avec son sigle et son code pays, numéro de dossier, note consolidée, durée et créneau souhaités, contraintes de programmation, journées spéciales, thématiques avec leur libellé et leur couleur, nombre d'intervenants.",
    path = "/sessions",
    tag = "Planificateur de séances",
    operation_id = "seances_liste",
    params(("event_id" = Uuid, Query, description = "Édition dont on liste les séances")),
    responses(
        (status = 200, description = "PlannerSession[]", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission de planifier absente, ou périmètre vide", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Édition inexistante **ou hors périmètre**", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn liste(
    state: web::Data<ProgrammeState>,
    perimetre: Perimeter,
    demande: web::Query<EditionDemandee>,
) -> Result<HttpResponse> {
    let event_id = garder(
        &state,
        &perimetre,
        Cible::Edition(EventId(demande.event_id)),
    )
    .await?;

    Ok(HttpResponse::Ok().json(planner::seances(state.pool(), event_id).await?))
}

/// Les chevauchements — **signalés, jamais bloqués**.
#[utoipa::path(
    get,
    description = "`ScheduleConflict[]` — `programme.detect_conflicts()`, **telle quelle**, sans filtrer ni requalifier les gravités. `blocking` : matériellement impossible — un seul stand par édition, un seul direct sur la plateforme, une salle physique à la fois. `warning` : gênant mais possible — un intervenant attendu à deux endroits, une organisation programmée deux fois ; l'équipe juge. **Aucun de ces conflits n'empêche une écriture** : le seul garde-fou dur est la publication du programme.",
    path = "/sessions/conflicts",
    tag = "Planificateur de séances",
    operation_id = "seances_conflits",
    params(("event_id" = Uuid, Query, description = "Édition dont on recense les chevauchements")),
    responses(
        (status = 200, description = "ScheduleConflict[]", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission de planifier absente, ou périmètre vide", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Édition inexistante **ou hors périmètre**", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn conflits(
    state: web::Data<ProgrammeState>,
    perimetre: Perimeter,
    demande: web::Query<EditionDemandee>,
) -> Result<HttpResponse> {
    let event_id = garder(
        &state,
        &perimetre,
        Cible::Edition(EventId(demande.event_id)),
    )
    .await?;

    Ok(HttpResponse::Ok().json(planner::conflits(state.pool(), event_id).await?))
}

// -----------------------------------------------------------------------------
// La fiche d'une séance
// -----------------------------------------------------------------------------

#[utoipa::path(
    get,
    description = "`SessionSpeaker[]` — les intervenants du jour, recopiés du dossier à la programmation puis modifiables : ceux qui étaient annoncés ne sont pas toujours ceux qui viennent.",
    path = "/sessions/{id}/speakers",
    tag = "Planificateur de séances",
    operation_id = "seances_intervenants",
    params(("id" = Uuid, Path, description = "Identifiant de la séance")),
    responses(
        (status = 200, description = "SessionSpeaker[]", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission de planifier absente, ou périmètre vide", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Séance inexistante **ou hors périmètre**", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn intervenants(
    state: web::Data<ProgrammeState>,
    perimetre: Perimeter,
    id: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let seance = SessionId(id.into_inner());
    garder(&state, &perimetre, Cible::Seance(seance)).await?;

    Ok(HttpResponse::Ok()
        .json(crate::repo::session_parts::intervenants(state.pool(), seance).await?))
}

#[utoipa::path(
    get,
    description = "`SessionOrganization[]` — le porteur principal et ses co-organisations. La ligne du porteur est posée **par déclencheur** et jamais par le service.",
    path = "/sessions/{id}/organizations",
    tag = "Planificateur de séances",
    operation_id = "seances_organisations",
    params(("id" = Uuid, Path, description = "Identifiant de la séance")),
    responses(
        (status = 200, description = "SessionOrganization[]", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission de planifier absente, ou périmètre vide", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Séance inexistante **ou hors périmètre**", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn organisations(
    state: web::Data<ProgrammeState>,
    perimetre: Perimeter,
    id: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let seance = SessionId(id.into_inner());
    garder(&state, &perimetre, Cible::Seance(seance)).await?;

    Ok(HttpResponse::Ok()
        .json(crate::repo::session_parts::organisations(state.pool(), seance).await?))
}

#[utoipa::path(
    get,
    description = "`SessionTrack[]` — les journées spéciales auxquelles la séance est rattachée, **avec qui les a posées**. La composition d'un fil est un choix éditorial qu'il arrive d'expliquer à une organisation qui s'étonne de ne pas y figurer.",
    path = "/sessions/{id}/tracks",
    tag = "Planificateur de séances",
    operation_id = "seances_fils",
    params(("id" = Uuid, Path, description = "Identifiant de la séance")),
    responses(
        (status = 200, description = "SessionTrack[]", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission de planifier absente, ou périmètre vide", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Séance inexistante **ou hors périmètre**", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn fils(
    state: web::Data<ProgrammeState>,
    perimetre: Perimeter,
    id: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let seance = SessionId(id.into_inner());
    garder(&state, &perimetre, Cible::Seance(seance)).await?;

    Ok(HttpResponse::Ok().json(crate::repo::session_parts::fils(state.pool(), seance).await?))
}

// -----------------------------------------------------------------------------
// Les trois écritures
// -----------------------------------------------------------------------------

/// Placer, déplacer, redimensionner, retirer.
#[utoipa::path(
    put,
    description = "`ScheduleSessionPayload` → `PlannerMutationResult`. **Une seule écriture pour quatre gestes** : la base n'en distingue pas, ce sont `room_id`, `starts_at` et `ends_at`. Une salle nulle **renvoie la séance au panneau** — ce n'est pas une suppression. **Jamais refusée pour chevauchement** : poser deux séances sur le même créneau réussit, et la réponse porte le conflit. La journée de rattachement est facultative : non fournie, elle est **remise à nul** pour que la base la redéduise, sans quoi une séance déplacée du 12 au 14 novembre resterait rangée au 12, en silence. La réponse porte les conflits de **toute l'édition** : un déplacement peut résoudre le conflit d'un bloc situé à l'autre bout de la semaine.",
    path = "/sessions/{id}/schedule",
    tag = "Planificateur de séances",
    operation_id = "seances_placer",
    params(("id" = Uuid, Path, description = "Identifiant de la séance")),
    request_body = ScheduleSessionPayload,
    responses(
        (status = 200, description = "PlannerMutationResult", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission de planifier absente, ou périmètre vide", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Séance inexistante **ou hors périmètre**", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Valeur déduite envoyée, créneau invalide, ou salle d'une autre édition", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn placer(
    requete: HttpRequest,
    state: web::Data<ProgrammeState>,
    perimetre: Perimeter,
    id: web::Path<Uuid>,
    corps: web::Json<ScheduleSessionPayload>,
) -> Result<HttpResponse> {
    let seance = SessionId(id.into_inner());
    let event_id = garder(&state, &perimetre, Cible::Seance(seance)).await?;

    let ctx = contexte_de(&requete, perimetre.person_id);
    let resultat = planner::placer(&state, &ctx, event_id, seance, corps.into_inner()).await?;

    Ok(HttpResponse::Ok().json(resultat))
}

/// Rattacher aux journées spéciales.
#[utoipa::path(
    put,
    description = "`SessionTracksPayload` → `PlannerMutationResult`. **Manuel et indépendant de la date** : toutes les activités du 12 novembre ne relèvent pas de la « Journée finance durable ». La liste envoyée **remplace** la précédente, et la base retient qui a rattaché quoi. Un fil d'une **autre édition** est refusé par un déclencheur du modèle, traduit ici en code stable.",
    path = "/sessions/{id}/tracks",
    tag = "Planificateur de séances",
    operation_id = "seances_rattacher",
    params(("id" = Uuid, Path, description = "Identifiant de la séance")),
    request_body = SessionTracksPayload,
    responses(
        (status = 200, description = "PlannerMutationResult", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission de planifier absente, ou périmètre vide", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Séance inexistante **ou hors périmètre**", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Journée spéciale d'une autre édition, ou inexistante", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn rattacher(
    requete: HttpRequest,
    state: web::Data<ProgrammeState>,
    perimetre: Perimeter,
    id: web::Path<Uuid>,
    corps: web::Json<SessionTracksPayload>,
) -> Result<HttpResponse> {
    let seance = SessionId(id.into_inner());
    let event_id = garder(&state, &perimetre, Cible::Seance(seance)).await?;

    let ctx = contexte_de(&requete, perimetre.person_id);
    let resultat =
        planner::rattacher_les_fils(&state, &ctx, event_id, seance, corps.into_inner()).await?;

    Ok(HttpResponse::Ok().json(resultat))
}

/// Marquer une séance diffusée, avec son canal.
#[utoipa::path(
    put,
    description = "`SessionBroadcastPayload` → `PlannerMutationResult`. **Le canal EST saisissable** quand la diffusion est activée : la base ne pose le canal par défaut de l'édition que lorsque la colonne est nulle — elle complète, elle n'écrase jamais —, et l'écran laisse le choix quand l'édition a plusieurs canaux. **Retirer la diffusion en désignant un canal est refusé** : c'est le seul cas où la base efface une valeur choisie sans le dire. Deux directs simultanés **s'écrivent** et remontent en gravité bloquante : la règle « un seul direct » est signalée, jamais imposée à l'écriture.",
    path = "/sessions/{id}/broadcast",
    tag = "Planificateur de séances",
    operation_id = "seances_diffusion",
    params(("id" = Uuid, Path, description = "Identifiant de la séance")),
    request_body = SessionBroadcastPayload,
    responses(
        (status = 200, description = "PlannerMutationResult", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission de planifier absente, ou périmètre vide", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Séance inexistante **ou hors périmètre**", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Canal désigné sans diffusion, canal désactivé ou d'une autre édition", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn diffuser(
    requete: HttpRequest,
    state: web::Data<ProgrammeState>,
    perimetre: Perimeter,
    id: web::Path<Uuid>,
    corps: web::Json<SessionBroadcastPayload>,
) -> Result<HttpResponse> {
    let seance = SessionId(id.into_inner());
    let event_id = garder(&state, &perimetre, Cible::Seance(seance)).await?;

    let ctx = contexte_de(&requete, perimetre.person_id);
    let resultat = planner::diffuser(&state, &ctx, event_id, seance, corps.into_inner()).await?;

    Ok(HttpResponse::Ok().json(resultat))
}

// -----------------------------------------------------------------------------
// Les deux chemins d'inscription qui vivent sous `/sessions`
//
// Ils appartiennent à US8 et sont servis par `routes/registrations.rs` : c'est
// le chemin qui les range ici, pas leur sujet.
// -----------------------------------------------------------------------------

pub(crate) use crate::routes::registrations::{formulaire, sinscrire};
