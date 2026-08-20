//! Le back-office de l'appel à propositions et de sa grille.
//!
//! **Permission distincte de celle des événements.** `event.call.manage` garde
//! l'appel, sa grille et son comité ; `event.event.manage` garde le décor de
//! l'édition. Détenir l'une n'accorde pas l'autre, et un test le vérifie dans
//! les deux sens : un compte peut tenir les salles d'une COP sans ouvrir sa
//! campagne.
//!
//! **Le chemin littéral précède le chemin paramétré.**
//! `/admin/calls/default-criteria` est déclarée avant `/admin/calls/{id}` : sans
//! cela, la grille par défaut serait lue comme un identifiant d'appel
//! (research.md § R11).

use actix_web::{web, HttpRequest, HttpResponse};
use kernel::auth::{Perimeter, Scope};
use kernel::error::Result;
use uuid::Uuid;

use crate::domain::call::EditionCallPayload;
use crate::domain::ids::{CallId, EventId};
use crate::domain::permissions::CALL_MANAGE;
use crate::repo::criteria;
use crate::routes::contexte_de;
use crate::service::{call as service_appel, edition_dans_le_perimetre, Cible};
use crate::state::EventState;

/// Le scope `/admin/calls` — **rempli, jamais créé ici**.
pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/default-criteria", web::get().to(grille_par_defaut))
        .route("", web::post().to(creer))
        .route("/{id}", web::put().to(modifier))
        // Le comité vit sous le scope de l'appel : deux `web::scope`
        // « /admin/calls » ne se compléteraient pas.
        .route(
            "/{id}/reviewers",
            web::put().to(super::admin_committee::enregistrer),
        );
}

/// La grille par défaut, **lue en base**.
#[utoipa::path(
    get,
    description = "`EditionCriterion[]` — les six critères que `event.seed_default_criteria()` sème, avec leurs libellés bilingues, leurs poids et l'éliminatoire. **Lue en base, jamais recopiée** : la fonction du modèle est exécutée sur un appel jetable, dans une transaction annulée dont rien ne subsiste. Recopier les six lignes dans un tableau Rust en ferait une seconde vérité, désynchronisée au premier ajustement de la grille. Les identifiants rendus sont nuls : ce sont des lignes **nouvelles**, que l'écran proposera d'enregistrer.",
    path = "/admin/calls/default-criteria",
    tag = "Back-office — appel à propositions",
    operation_id = "admin_appel_grille_par_defaut",
    responses(
        (status = 200, description = "EditionCriterion[]", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente, ou périmètre d'administration vide", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn grille_par_defaut(
    state: web::Data<EventState>,
    perimetre: Perimeter,
) -> Result<HttpResponse> {
    kernel::auth::require_permission_anywhere(state.pool(), perimetre.person_id, CALL_MANAGE)
        .await?;

    let grille = criteria::grille_par_defaut(state.pool()).await?;

    Ok(HttpResponse::Ok().json(grille))
}

/// Ouvrir un appel — **l'appel et sa grille en une transaction**.
#[utoipa::path(
    post,
    description = "`EditionCallPayload` → `CallSaveResult`. **L'appel et sa grille en une seule transaction** : un échec sur la grille ne laisse aucun appel derrière lui. L'édition vient du corps, **et elle est vérifiée** — le périmètre d'administration est appliqué avant toute écriture. Une édition qui porte déjà un appel non annulé rend `already_exists` en 200 ; un appel **annulé** n'empêche rien, l'index l'exclut. `scores_affected` prévient qu'un barème modifié va déplacer des moyennes déjà calculées.",
    path = "/admin/calls",
    tag = "Back-office — appel à propositions",
    operation_id = "admin_appel_creer",
    request_body = Object,
    responses(
        (status = 200, description = "CallSaveResult", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente, ou périmètre d'administration vide", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Édition inexistante **ou hors périmètre** — indiscernables", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Retrait d'un critère porteur de notes (EVENT_CRITERION_HAS_SCORES)", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn creer(
    requete: HttpRequest,
    state: web::Data<EventState>,
    perimetre: Perimeter,
    corps: web::Json<EditionCallPayload>,
) -> Result<HttpResponse> {
    let payload = corps.into_inner();
    // À la création, l'édition ne peut venir que du corps — mais elle est
    // **vérifiée** en base, comme n'importe quelle ascendance : c'est le
    // périmètre qui tranche, jamais ce que le client déclare.
    let event_id = edition_dans_le_perimetre(
        state.pool(),
        &perimetre,
        Cible::Edition(EventId::from(payload.event_id)),
    )
    .await?;

    autoriser(&state, &perimetre, event_id).await?;

    let ctx = contexte_de(&requete, perimetre.person_id);
    let resultat =
        service_appel::creer(&state, &ctx, perimetre.person_id, event_id, payload).await?;

    Ok(HttpResponse::Ok().json(resultat))
}

/// Modifier un appel — écriture **totale**, grille comprise.
#[utoipa::path(
    put,
    description = "`EditionCallPayload` → `CallSaveResult`. L'édition vient de **l'ascendance de l'appel**, jamais du corps. Écriture totale : tous les champs modifiables sont réécrits, y compris à nul, ce qui permet de retirer une prolongation. La grille est enregistrée par **diff sur le code** — insertion, mise à jour, suppression. **Retirer un critère porteur de notes est refusé** en 422 : la clé est `ON DELETE CASCADE`, et la base effacerait sans un mot l'argumentaire des évaluations rendues.",
    path = "/admin/calls/{id}",
    tag = "Back-office — appel à propositions",
    operation_id = "admin_appel_modifier",
    params(("id" = uuid::Uuid, Path, description = "Identifiant de l'appel")),
    request_body = Object,
    responses(
        (status = 200, description = "CallSaveResult", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente, ou périmètre d'administration vide", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Appel inexistant **ou hors périmètre** — indiscernables", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Retrait d'un critère porteur de notes (EVENT_CRITERION_HAS_SCORES)", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn modifier(
    requete: HttpRequest,
    state: web::Data<EventState>,
    perimetre: Perimeter,
    chemin: web::Path<Uuid>,
    corps: web::Json<EditionCallPayload>,
) -> Result<HttpResponse> {
    let call_id = CallId::from(chemin.into_inner());
    let event_id =
        edition_dans_le_perimetre(state.pool(), &perimetre, Cible::Appel(call_id)).await?;

    autoriser(&state, &perimetre, event_id).await?;

    let ctx = contexte_de(&requete, perimetre.person_id);
    let resultat = service_appel::modifier(
        &state,
        &ctx,
        perimetre.person_id,
        event_id,
        call_id,
        corps.into_inner(),
    )
    .await?;

    Ok(HttpResponse::Ok().json(resultat))
}

/// La permission de gérer les **appels**, sur l'édition visée. Écrite une fois :
/// trois routes la partagent, et trois copies finiraient par diverger.
pub(crate) async fn autoriser(
    state: &EventState,
    perimetre: &Perimeter,
    event_id: EventId,
) -> Result<()> {
    kernel::auth::require_permission(
        state.pool(),
        perimetre.person_id,
        CALL_MANAGE,
        Scope::Event(event_id.as_uuid()),
    )
    .await
}
