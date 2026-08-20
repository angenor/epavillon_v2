//! Le back-office des éditions.
//!
//! **Les chemins littéraux précèdent les chemins paramétrés.**
//! `/admin/events/form-options` est déclarée avant `/admin/events/{id}`, sans
//! quoi elle serait lue comme un identifiant d'édition et rendrait un refus
//! incompréhensible (research.md § R11).
//!
//! **La création exige la portée GLOBALE, et pas une autre.** Une édition qui
//! n'existe pas encore n'offre aucune portée où vérifier un droit : exiger la
//! permission « sur cette édition » serait la vérifier sur un identifiant que
//! personne ne détient (FR-011). D'où un code distinct de `FORBIDDEN` —
//! l'écran sait dire *pourquoi*.

use actix_web::{web, HttpRequest, HttpResponse};
use kernel::auth::{Perimeter, Scope};
use kernel::error::{ApiError, Result};
use uuid::Uuid;

use crate::domain::edition::{EditionFormOptions, EditionFormPayload};
use crate::domain::ids::EventId;
use crate::domain::permissions::EVENT_MANAGE;
use crate::repo::editions;
use crate::routes::contexte_de;
use crate::service::{
    detail as composition, edition_dans_le_perimetre, edition_read, edition_write,
    portee_globale_exigee, Cible,
};
use crate::state::EventState;

/// Le scope `/admin/events` — **rempli, jamais créé ici** : deux `web::scope`
/// du même préfixe ne se complètent pas.
pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/form-options", web::get().to(options_de_formulaire))
        .route("", web::get().to(lister))
        .route("", web::post().to(creer))
        .route("/{id}", web::get().to(detail))
        .route("/{id}", web::put().to(modifier))
        // **Les journées vivent sous le scope de l'édition**, et non dans le
        // leur : deux `web::scope("/admin/events")` ne se compléteraient pas.
        .route(
            "/{id}/days/plan",
            web::get().to(super::admin_tabs::plan_des_journees),
        )
        .route(
            "/{id}/days",
            web::post().to(super::admin_tabs::generer_les_journees),
        )
        .route(
            "/{id}/days/{dayId}",
            web::put().to(super::admin_tabs::habiller_une_journee),
        );
}

/// Ce qu'il faut pour ouvrir le formulaire.
#[utoipa::path(
    get,
    description = "`EditionFormOptions` — séries avec leur décompte d'éditions, pays, fuseaux et statuts. Servie **à part de la liste** : le référentiel des pays ne repart pas à chaque affichage du tableau. Les fuseaux viennent de `pg_timezone_names`, le dictionnaire même contre lequel le domaine du modèle vérifie ce qu'on écrit ; les statuts sont lus dans l'énuméré du modèle, dans l'ordre où il les déclare.",
    path = "/admin/events/form-options",
    tag = "Back-office — événements",
    operation_id = "admin_editions_options_de_formulaire",
    responses(
        (status = 200, description = "EditionFormOptions", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente, ou périmètre d'administration vide", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn options_de_formulaire(
    state: web::Data<EventState>,
    perimetre: Perimeter,
) -> Result<HttpResponse> {
    kernel::auth::require_permission_anywhere(state.pool(), perimetre.person_id, EVENT_MANAGE)
        .await?;

    let options = EditionFormOptions {
        series: editions::series(state.pool()).await?,
        countries: editions::pays(state.pool()).await?,
        timezones: editions::fuseaux(state.pool()).await?,
        statuses: editions::statuts(state.pool()).await?,
    };

    Ok(HttpResponse::Ok().json(options))
}

/// La liste, bornée par le périmètre.
#[utoipa::path(
    get,
    description = "`EditionListScreen` — les lignes, les séries proposables au filtre et les millésimes présents, **en une réponse**, les facettes comptées sur le **même jeu de lignes** que la liste. `is_global_scope` dit si l'appelant administre la plateforme entière, pour que l'écran distingue un filtrage d'une absence. **Un périmètre vide reçoit 403, jamais une liste vide** : personne ne doit avoir à deviner s'il n'y a rien à voir ou s'il n'a pas le droit de voir.",
    path = "/admin/events",
    tag = "Back-office — événements",
    operation_id = "admin_editions_lister",
    responses(
        (status = 200, description = "EditionListScreen", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente, ou périmètre d'administration vide", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn lister(
    state: web::Data<EventState>,
    perimetre: Perimeter,
) -> Result<HttpResponse> {
    kernel::auth::require_permission_anywhere(state.pool(), perimetre.person_id, EVENT_MANAGE)
        .await?;

    let ecran = edition_read::ecran(state.pool(), &perimetre.scope).await?;

    Ok(HttpResponse::Ok().json(ecran))
}

/// Le détail d'une édition — **les six onglets en une réponse**.
#[utoipa::path(
    get,
    description = "`EditionDetail` — l'édition, ses deux textes longs, sa période en dates civiles **dans son fuseau**, ses trois déclinaisons d'image résolues, ses journées, ses fils, ses lieux et salles, ses canaux, son appel et sa grille, son comité, le personnel assignable et les thématiques disponibles. **Ouvrir un onglet ne demande aucun appel supplémentaire** : les douze lectures se font sur une seule connexion, dans une transaction en lecture seule, pour que les décomptes des six onglets soient cohérents entre eux. Une édition **inexistante ou hors périmètre** rend 404 — les deux sont indiscernables, sans quoi une URL forgée dirait à qui la forge si l'objet existe.",
    path = "/admin/events/{id}",
    tag = "Back-office — événements",
    operation_id = "admin_edition_detail",
    params(("id" = uuid::Uuid, Path, description = "Identifiant de l'édition")),
    responses(
        (status = 200, description = "EditionDetail", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente, ou périmètre d'administration vide", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Édition inexistante **ou hors périmètre** — indiscernables", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn detail(
    state: web::Data<EventState>,
    perimetre: Perimeter,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let id = EventId::from(chemin.into_inner());
    // **Résoudre, puis vérifier le périmètre, puis lire** : l'ordre est imposé,
    // et l'absence de l'édition produit le même refus que l'échec du périmètre.
    let id = edition_dans_le_perimetre(state.pool(), &perimetre, Cible::Edition(id)).await?;

    kernel::auth::require_permission(
        state.pool(),
        perimetre.person_id,
        EVENT_MANAGE,
        Scope::Event(id.as_uuid()),
    )
    .await?;

    // L'édition vient d'être vue dans le périmètre : son absence ici ne peut
    // venir que d'une suppression concurrente, et se rend comme un introuvable.
    let detail = composition::composer(state.pool(), id)
        .await?
        .ok_or_else(ApiError::not_found)?;

    Ok(HttpResponse::Ok().json(detail))
}

/// Créer une édition.
#[utoipa::path(
    post,
    description = "`EditionFormPayload` → `EditionSaveResult`. **Portée GLOBALE exigée** : une édition qui n'existe pas encore n'offre aucune portée où vérifier un droit — `EVENT_GLOBAL_SCOPE_REQUIRED` sinon. Les refus de saisie sortent en **200**, dans `errors`, chacun sur son champ. Une édition **dont le pavillon est tenu** doit porter un sigle : le refus emprunte `{ code: 'required', field: 'acronym' }` et la réponse porte en plus `suggested_acronym`, une valeur dérivée du libellé, utilisable telle quelle. `days_created` compte les journées que la période a ajoutées ; `days_removed` et `sessions_detached` valent **toujours zéro** — un enregistrement d'édition ne supprime aucune journée.",
    path = "/admin/events",
    tag = "Back-office — événements",
    operation_id = "admin_edition_creer",
    request_body = Object,
    responses(
        (status = 200, description = "EditionSaveResult", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Portée globale exigée (EVENT_GLOBAL_SCOPE_REQUIRED), ou périmètre vide", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Série, pays ou fuseau inconnus (EVENT_UNKNOWN_REFERENCE)", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn creer(
    requete: HttpRequest,
    state: web::Data<EventState>,
    perimetre: Perimeter,
    corps: web::Json<EditionFormPayload>,
) -> Result<HttpResponse> {
    portee_globale_exigee(state.pool(), perimetre.person_id).await?;

    let ctx = contexte_de(&requete, perimetre.person_id);
    let resultat =
        edition_write::creer(&state, &ctx, perimetre.person_id, corps.into_inner()).await?;

    Ok(HttpResponse::Ok().json(resultat))
}

/// Modifier une édition — écriture **totale**.
#[utoipa::path(
    put,
    description = "`EditionFormPayload` → `EditionSaveResult`. Écriture **totale** : tous les champs modifiables sont réécrits, y compris à nul — c'est ce qui permet d'effacer un sigle, une ville ou des coordonnées. `programme_published_at` n'est **jamais** touchée ici : elle est posée par la publication seule. L'identifiant vient de l'adresse, jamais du corps. Une édition hors périmètre rend **404**, indiscernable d'une édition inexistante.",
    path = "/admin/events/{id}",
    tag = "Back-office — événements",
    operation_id = "admin_edition_modifier",
    params(("id" = uuid::Uuid, Path, description = "Identifiant de l'édition")),
    request_body = Object,
    responses(
        (status = 200, description = "EditionSaveResult", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente, ou périmètre d'administration vide", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Édition inexistante **ou hors périmètre** — indiscernables", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Série, pays ou fuseau inconnus (EVENT_UNKNOWN_REFERENCE)", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn modifier(
    requete: HttpRequest,
    state: web::Data<EventState>,
    perimetre: Perimeter,
    chemin: web::Path<Uuid>,
    corps: web::Json<EditionFormPayload>,
) -> Result<HttpResponse> {
    let id = EventId::from(chemin.into_inner());
    let id = edition_dans_le_perimetre(state.pool(), &perimetre, Cible::Edition(id)).await?;

    kernel::auth::require_permission(
        state.pool(),
        perimetre.person_id,
        EVENT_MANAGE,
        Scope::Event(id.as_uuid()),
    )
    .await?;

    let ctx = contexte_de(&requete, perimetre.person_id);
    let resultat =
        edition_write::modifier(&state, &ctx, perimetre.person_id, id, corps.into_inner()).await?;

    Ok(HttpResponse::Ok().json(resultat))
}
