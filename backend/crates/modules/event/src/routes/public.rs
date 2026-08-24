//! Ce que ce module sert **hors du back-office**.
//!
//! **L'ordre d'enregistrement compte** : un chemin littéral déclaré après un
//! chemin paramétré de même forme est capturé par lui — `/events/public` serait
//! lu comme l'adresse d'URL `public` (research.md § R11). Les lectures publiques
//! arrivent avec la phase qui les livre ; la première route posée ici est celle
//! du sélecteur, qui n'a pas d'homologue paramétré.

use actix_web::{web, HttpResponse};
use kernel::auth::Actor;
use kernel::error::Result;

use crate::domain::ids::EventId;
use crate::repo::editions;
use crate::repo::public as repo_public;
use crate::service::public_read;
use crate::state::EventState;
use uuid::Uuid;

pub fn configurer(cfg: &mut web::ServiceConfig) {
    // **`/events/public` AVANT `/events/{slug}`.** Actix retient la première
    // route dont le motif correspond : déclarée après, la première serait lue
    // comme l'adresse d'URL « public » et rendrait `null` (research.md § R11).
    cfg.route("/events/public", web::get().to(editions_publiques))
        .route("/events", web::get().to(selecteur_des_editions))
        .route("/event-series", web::get().to(series))
        .route("/events/{id}/days", web::get().to(journees))
        .route("/events/{id}/tracks", web::get().to(fils))
        .route("/events/{id}/venues", web::get().to(lieux))
        .route("/events/{id}/rooms", web::get().to(salles))
        .route("/events/{id}/channels", web::get().to(canaux))
        .route("/events/{id}/call", web::get().to(appel))
        .route("/events/{id}/images", web::get().to(images))
        .route("/events/{slug}", web::get().to(edition_publique));
}

/// Le sélecteur d'édition du back-office.
#[utoipa::path(
    get,
    description = "`EventEdition[]` — les éditions que l'appelant administre, pour le sélecteur du back-office. **Filtrée par le périmètre, et non refusée** : un périmètre vide rend une **liste vide**, et c'est le store qui décide de l'écran. C'est la seule route de ce module où périmètre vide n'est pas un refus, parce que le contrat du front le veut ainsi — toutes les autres lectures du back-office rendent 403. Décroissante sur la date de début.",
    path = "/events",
    tag = "Back-office — événements",
    operation_id = "editions_du_perimetre",
    responses(
        (status = 200, description = "EventEdition[]", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn selecteur_des_editions(
    state: web::Data<EventState>,
    acteur: Actor,
) -> Result<HttpResponse> {
    // `administered_events` et non `require_perimeter` : ici, un périmètre vide
    // n'est pas un refus. C'est écrit une fois, à l'endroit où la différence se
    // joue, pour qu'on ne prenne pas ce choix pour un oubli.
    let perimetre = kernel::auth::administered_events(state.pool(), acteur.0).await?;
    let editions = editions::selecteur(state.pool(), &perimetre).await?;

    Ok(HttpResponse::Ok().json(editions))
}

/// Les éditions publiques.
#[utoipa::path(
    get,
    description = "`PublicEditionRow[]` — la ligne de `event.v_public_editions`, et **non** `EventEdition`, la ligne nue de la table : elle porte en plus la série et le pays résolus, les trois déclinaisons d'image, l'état temporel, l'appel résolu et le volume du programme publié. Les éditions publiques, décroissantes sur la date de début. **Le critère de publicité vient du modèle** : ni brouillon, ni annulée. Il n'est recopié dans aucun écran, ce qui referme l'écart n° 26 — une édition **annoncée** dont le programme n'est pas publié en fait partie, car sa page existe et c'est là qu'on dépose un dossier. Chaque ligne porte sa série et son pays résolus, ses **trois déclinaisons d'image**, son état temporel, son appel résolu et le volume de son programme publié. **Ce chemin est déclaré AVANT `/events/{slug}`** : sans cela, `public` serait lu comme une adresse d'URL.",
    path = "/events/public",
    tag = "Événements",
    operation_id = "editions_publiques",
    responses((status = 200, description = "PublicEditionRow[]", body = Object))
)]
pub(crate) async fn editions_publiques(state: web::Data<EventState>) -> Result<HttpResponse> {
    let editions = public_read::editions(state.pool()).await?;

    Ok(HttpResponse::Ok().json(editions))
}

/// La page d'une édition, par son adresse d'URL.
#[utoipa::path(
    get,
    description = "`EventEdition | null` — **une requête, deux vues**. L'édition, sa série, son pays, ses **trois déclinaisons d'image** résolues, son état temporel, son appel et l'échéance **effective** (prolongation comprise), plus le volume du programme publié, joint **par la gauche** : une édition annoncée sans aucune séance publiée reste visible. `null` pour un brouillon, une annulée ou une adresse inconnue — **les trois sont indiscernables**, sans quoi l'adresse d'une édition en préparation se devinerait.",
    path = "/events/{slug}",
    tag = "Événements",
    operation_id = "edition_publique",
    params(("slug" = String, Path, description = "Adresse d'URL de l'édition")),
    responses((status = 200, description = "EventEdition | null", body = Object))
)]
pub(crate) async fn edition_publique(
    state: web::Data<EventState>,
    chemin: web::Path<String>,
) -> Result<HttpResponse> {
    let edition = public_read::edition_par_slug(state.pool(), &chemin.into_inner()).await?;

    Ok(HttpResponse::Ok().json(edition))
}

/// Les séries, avec leur décompte d'éditions.
#[utoipa::path(
    get,
    description = "`EventSeries[]` — les séries avec leur **genre** et leur décompte d'éditions. C'est `kind` qui distingue une COP d'un cycle de webinaires, jamais une liste d'adresses recopiée dans un composant. Le décompte est joint **par la gauche** : une série sans édition reste visible, à zéro.",
    path = "/event-series",
    tag = "Événements",
    operation_id = "series_devenements",
    responses((status = 200, description = "EventSeries[]", body = Object))
)]
pub(crate) async fn series(state: web::Data<EventState>) -> Result<HttpResponse> {
    let series = repo_public::series(state.pool()).await?;

    Ok(HttpResponse::Ok().json(series))
}

/// Le calendrier d'une édition.
#[utoipa::path(
    get,
    description = "`EventDay[]` — le calendrier d'une édition, une ligne par jour, croissant. Une journée spéciale n'est **pas** un jour du calendrier : elle vit dans les fils de programmation.",
    path = "/events/{id}/days",
    tag = "Événements",
    operation_id = "journees_publiques",
    params(("id" = uuid::Uuid, Path, description = "Identifiant de l'édition")),
    responses((status = 200, description = "EventDay[]", body = Object))
)]
pub(crate) async fn journees(
    state: web::Data<EventState>,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let journees = repo_public::journees(state.pool(), EventId::from(chemin.into_inner())).await?;

    Ok(HttpResponse::Ok().json(journees))
}

/// Les fils de programmation **publiés**.
#[utoipa::path(
    get,
    description = "`ProgrammeTrack[]` — les fils dont la page publique est **ouverte**, et eux seuls. Un fil sans page ouverte n'existe pas pour le public : le filtre est `published_at IS NOT NULL`, la colonne même que le modèle indexe pour cet usage.",
    path = "/events/{id}/tracks",
    tag = "Événements",
    operation_id = "fils_publics",
    params(("id" = uuid::Uuid, Path, description = "Identifiant de l'édition")),
    responses((status = 200, description = "ProgrammeTrack[]", body = Object))
)]
pub(crate) async fn fils(
    state: web::Data<EventState>,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let fils = repo_public::fils_publies(state.pool(), EventId::from(chemin.into_inner())).await?;

    Ok(HttpResponse::Ok().json(fils))
}

/// Les lieux d'une édition.
#[utoipa::path(
    get,
    description = "`Venue[]` — les lieux d'une édition. Ce sont eux qui donnent un **sujet nommable** à un conflit de créneaux : sans salle en base, la détection ne peut dire que « deux activités à 14 h ».",
    path = "/events/{id}/venues",
    tag = "Événements",
    operation_id = "lieux_publics",
    params(("id" = uuid::Uuid, Path, description = "Identifiant de l'édition")),
    responses((status = 200, description = "Venue[]", body = Object))
)]
pub(crate) async fn lieux(
    state: web::Data<EventState>,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let lieux = repo_public::lieux(state.pool(), EventId::from(chemin.into_inner())).await?;

    Ok(HttpResponse::Ok().json(lieux))
}

/// Les salles de tous les lieux d'une édition.
#[utoipa::path(
    get,
    description = "`Room[]` — les salles de **tous les lieux** de l'édition : une salle ne porte pas l'édition, elle la tient de son lieu. `is_virtual` n'est pas un détail d'inventaire — une salle virtuelle accepte les créneaux simultanés.",
    path = "/events/{id}/rooms",
    tag = "Événements",
    operation_id = "salles_publiques",
    params(("id" = uuid::Uuid, Path, description = "Identifiant de l'édition")),
    responses((status = 200, description = "Room[]", body = Object))
)]
pub(crate) async fn salles(
    state: web::Data<EventState>,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let salles = repo_public::salles(state.pool(), EventId::from(chemin.into_inner())).await?;

    Ok(HttpResponse::Ok().json(salles))
}

/// Les canaux de diffusion d'une édition, **et ceux de la plateforme**.
#[utoipa::path(
    get,
    description = "`BroadcastChannel[]` — les canaux de l'édition **et** les canaux généraux de la plateforme, comme le front les compose déjà. Un canal général sert les diffusions dont l'événement n'a pas le sien ; le taire ferait croire qu'aucun canal n'existe.",
    path = "/events/{id}/channels",
    tag = "Événements",
    operation_id = "canaux_publics",
    params(("id" = uuid::Uuid, Path, description = "Identifiant de l'édition")),
    responses((status = 200, description = "BroadcastChannel[]", body = Object))
)]
pub(crate) async fn canaux(
    state: web::Data<EventState>,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let canaux = repo_public::canaux(state.pool(), EventId::from(chemin.into_inner())).await?;

    Ok(HttpResponse::Ok().json(canaux))
}

/// L'appel à propositions d'une édition — **zéro ou un**.
#[utoipa::path(
    get,
    description = "`PublicCall | null` — **zéro ou un, jamais un tableau** : `ux_calls_one_per_event` tient la cardinalité, et l'annulé est exclu. Zéro pour une COP sans pavillon, où l'IFDD n'envoie qu'un représentant. Porte sa GRILLE D'ÉVALUATION (`criteria`) : elle est publique par nature — une organisation qui prépare un dossier doit savoir sur quoi il sera jugé —, et la servir à part coûtait une seconde vague d'appels à la page qui l'affiche.",
    path = "/events/{id}/call",
    tag = "Événements",
    operation_id = "appel_public",
    params(("id" = uuid::Uuid, Path, description = "Identifiant de l'édition")),
    responses((status = 200, description = "PublicCall | null", body = Object))
)]
pub(crate) async fn appel(
    state: web::Data<EventState>,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let appel = repo_public::appel(state.pool(), EventId::from(chemin.into_inner())).await?;

    Ok(HttpResponse::Ok().json(appel))
}

/// Les trois déclinaisons d'image — **vouée à disparaître**.
#[utoipa::path(
    get,
    description = "`Record<EditionImageRole, AttachedImage | null>` — les trois déclinaisons résolues par `media.attached_image()`. **Livrée pour ne pas casser un écran déjà en place, et vouée à disparaître** : `GET /events/{slug}` porte désormais ces mêmes images, et cet aller-retour n'a plus de raison d'être. Son retrait est inscrit aux obligations de B7 (écart n° 25). Les trois clés sont toujours présentes, à `null` tant que rien n'a été téléversé.",
    path = "/events/{id}/images",
    tag = "Événements",
    operation_id = "images_de_ledition",
    params(("id" = uuid::Uuid, Path, description = "Identifiant de l'édition")),
    responses((status = 200, description = "Record<EditionImageRole, AttachedImage | null>", body = Object))
)]
pub(crate) async fn images(
    state: web::Data<EventState>,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let images = public_read::images(state.pool(), EventId::from(chemin.into_inner())).await?;

    Ok(HttpResponse::Ok().json(images))
}
