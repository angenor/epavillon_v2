//! Le back-office de la vitrine.
//!
//! **Neuf routes plates, jamais un `web::scope("/admin")`** : le préfixe
//! d'administration est partagé — le planificateur vient de B5, les règles de
//! rappel de B6 —, et deux scopes du même préfixe ne se complètent pas. Actix
//! retient le premier dont le préfixe correspond et rend 404 si la route n'y
//! figure pas : un scope ici rendrait muettes les routes des autres modules,
//! exactement le défaut qui a coûté trois routes en B2.
//!
//! **L'ordre d'enregistrement compte** : `/admin/showcase/new` et
//! `/admin/showcase/sessions` sont déclarées AVANT `/admin/showcase/{id}`, sans
//! quoi elles seraient lues comme des identifiants.
//!
//! # Ce que chaque écriture rend
//!
//! `ShowcaseWriteResult`, toujours, et en **200** : un refus de validation est
//! une réponse prévue par le contrat, que le formulaire pose sur ses champs. Les
//! statuts d'erreur restent pour ce que le contrat n'exprime pas — pas de
//! session, périmètre insuffisant, diapositive introuvable.

use actix_web::{web, HttpRequest, HttpResponse};
use kernel::auth::Perimeter;
use kernel::error::{ApiError, Result};
use serde::Deserialize;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::admin::{
    ShowcaseFormScreen, ShowcaseFormValues, ShowcaseListScreen, ShowcaseReorderPayload,
    ShowcaseStatusPayload, ShowcaseWriteResult,
};
use crate::repo::admin as repo;
use crate::service::admin as service;
use crate::state::ContentState;

/// La permission qui ouvre cet écran. Testée par PERMISSION et jamais par nom
/// de rôle, comme partout dans le projet.
const HIGHLIGHT_MANAGE: &str = "content.highlight.manage";

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/admin/showcase", web::get().to(lister))
        .route("/admin/showcase", web::post().to(creer))
        .route("/admin/showcase/new", web::get().to(formulaire_vierge))
        .route("/admin/showcase/sessions", web::get().to(seances))
        .route("/admin/showcase/{id}/form", web::get().to(formulaire))
        .route(
            "/admin/showcase/{id}/status",
            web::post().to(changer_le_statut),
        )
        .route("/admin/showcase/{id}/order", web::post().to(deplacer))
        .route("/admin/showcase/{id}/duplicate", web::post().to(dupliquer))
        .route("/admin/showcase/{id}", web::get().to(valeurs))
        .route("/admin/showcase/{id}", web::patch().to(modifier));
}

#[derive(Debug, Deserialize)]
pub struct SeancesQuery {
    pub event_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct FormulaireQuery {
    /// Emplacement d'arrivée. Un seul existe depuis le 24/08 ; le paramètre
    /// reste accepté pour que le site n'ait pas à le retirer de son appel.
    pub placement: Option<String>,
}

/// La liste et ses facettes.
#[utoipa::path(
    get,
    description = "`ShowcaseListScreen` — les lignes du périmètre, leurs facettes et les référentiels du filtre, **en une réponse**. Les lignes arrivent triées par emplacement puis rang, avec `is_first` / `is_last` déjà posés : c'est ce qui désactive les boutons d'ordre aux extrémités sans que l'écran recompte.\n\n**`broadcast_state` n'est pas `status`** : une diapositive publiée dont la fenêtre s'ouvre la semaine prochaine est `scheduled`, une autre dont la fenêtre est close est `expired`. La liste dit ce que le public voit, pas seulement ce que l'éditeur a décidé.\n\n**Un périmètre vide reçoit 403, jamais une liste vide.** Un contenu de plateforme (`event_id` nul) n'est visible qu'en portée globale.",
    path = "/admin/showcase",
    tag = "Back-office — vitrine",
    operation_id = "admin_vitrine_lister",
    responses(
        (status = 200, description = "ShowcaseListScreen", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente, ou périmètre d'administration vide", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn lister(
    state: web::Data<ContentState>,
    perimetre: Perimeter,
) -> Result<HttpResponse> {
    kernel::auth::require_permission_anywhere(state.pool(), perimetre.person_id, HIGHLIGHT_MANAGE)
        .await?;

    let ecran = composer_la_liste(&state, &perimetre).await?;
    Ok(HttpResponse::Ok().json(ecran))
}

/// L'écran de formulaire, en création.
#[utoipa::path(
    get,
    description = "`ShowcaseFormScreen` — le formulaire vierge et ses référentiels : natures, éditions du périmètre, organisations, personnes, pays, thématiques et les trois emplacements de média avec leurs contraintes, lues de `media.attachable_roles`.\n\n`preview` porte le contrat **exact** du bandeau public : l'aperçu est rendu par le composant qui sert la vitrine, jamais par une seconde mise en page qui divergerait au premier ajustement de charte.\n\n**Une administratrice détachée n'ouvre pas un contenu de plateforme** : `is_global_scope` est faux, et le formulaire s'ouvre alors sur son édition.",
    path = "/admin/showcase/new",
    tag = "Back-office — vitrine",
    operation_id = "admin_vitrine_formulaire_vierge",
    params(("placement" = Option<String>, Query, description = "Emplacement d'arrivée — un seul existe")),
    responses(
        (status = 200, description = "ShowcaseFormScreen", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente, ou périmètre d'administration vide", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn formulaire_vierge(
    state: web::Data<ContentState>,
    perimetre: Perimeter,
    requete: web::Query<FormulaireQuery>,
) -> Result<HttpResponse> {
    kernel::auth::require_permission_anywhere(state.pool(), perimetre.person_id, HIGHLIGHT_MANAGE)
        .await?;

    // Une personne détachée ne crée pas de contenu de plateforme : le
    // formulaire s'ouvre sur SA première édition plutôt que de proposer une
    // option qu'on lui refuserait ensuite.
    let event_id = if perimetre.scope.is_global {
        None
    } else {
        perimetre.scope.event_ids.first().copied()
    };

    let valeurs = ShowcaseFormValues {
        id: None,
        placement: requete
            .placement
            .clone()
            .unwrap_or_else(|| "home_hero".to_owned()),
        status: "draft".to_owned(),
        nature_code: String::new(),
        sort_order: 0,
        title: serde_json::json!({}),
        quote: None,
        body: None,
        person_id: None,
        author_name: None,
        author_title: None,
        organization_id: None,
        organization_label: None,
        country_id: None,
        event_id,
        session_id: None,
        link_url: None,
        link_label: None,
        background_color_hex: None,
        starts_at: None,
        ends_at: None,
        theme_codes: Vec::new(),
    };

    let ecran = composer_le_formulaire(&state, &perimetre, valeurs, None).await?;
    Ok(HttpResponse::Ok().json(ecran))
}

/// L'écran de formulaire, en modification.
#[utoipa::path(
    get,
    description = "`ShowcaseFormScreen` — la diapositive et tous ses référentiels. **Deux issues pour deux choses différentes** : une diapositive inexistante ou hors périmètre rend 404 — les deux sont indiscernables, sans quoi une URL forgée dirait à qui la forge si l'objet existe —, tandis qu'un contenu de plateforme demandé sans portée globale rend 403 **en le disant** : l'écran doit pouvoir expliquer pourquoi une ligne visible n'est pas modifiable.",
    path = "/admin/showcase/{id}/form",
    tag = "Back-office — vitrine",
    operation_id = "admin_vitrine_formulaire",
    params(("id" = Uuid, Path, description = "Identifiant de la diapositive")),
    responses(
        (status = 200, description = "ShowcaseFormScreen", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente, périmètre vide, ou contenu de plateforme hors portée globale", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Diapositive inexistante **ou hors périmètre** — indiscernables", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn formulaire(
    state: web::Data<ContentState>,
    perimetre: Perimeter,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let id = chemin.into_inner();
    let valeurs = charger_dans_le_perimetre(&state, &perimetre, id).await?;
    let ecran = composer_le_formulaire(&state, &perimetre, valeurs, Some(id)).await?;
    Ok(HttpResponse::Ok().json(ecran))
}

/// Les valeurs seules — pour recharger le fond du formulaire après une écriture.
#[utoipa::path(
    get,
    description = "`ShowcaseFormValues` — la diapositive **sans les référentiels**. Sert à relire le fond du formulaire après une écriture sans repayer les natures, les éditions, les organisations et les pays que l'écran de formulaire embarque. Mêmes refus que lui.",
    path = "/admin/showcase/{id}",
    tag = "Back-office — vitrine",
    operation_id = "admin_vitrine_valeurs",
    params(("id" = Uuid, Path, description = "Identifiant de la diapositive")),
    responses(
        (status = 200, description = "ShowcaseFormValues", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente, périmètre vide, ou contenu de plateforme hors portée globale", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Diapositive inexistante **ou hors périmètre**", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn valeurs(
    state: web::Data<ContentState>,
    perimetre: Perimeter,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let valeurs = charger_dans_le_perimetre(&state, &perimetre, chemin.into_inner()).await?;
    Ok(HttpResponse::Ok().json(valeurs))
}

/// Les séances d'une édition, pour la cascade « édition → séance ».
#[utoipa::path(
    get,
    description = "`ShowcaseSessionOption[]` — les séances **publiées** de l'édition demandée, dans l'ordre du temps, chacune avec son fuseau. Changer d'édition dans le formulaire change la liste sans recharger l'écran : sans cette route, la saisie en cours serait perdue à chaque changement. L'édition est vérifiée contre le périmètre **avant** la lecture.",
    path = "/admin/showcase/sessions",
    tag = "Back-office — vitrine",
    operation_id = "admin_vitrine_seances",
    params(("event_id" = Uuid, Query, description = "Édition dont on veut les séances")),
    responses(
        (status = 200, description = "ShowcaseSessionOption[]", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente, ou périmètre d'administration vide", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Édition hors périmètre", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn seances(
    state: web::Data<ContentState>,
    perimetre: Perimeter,
    requete: web::Query<SeancesQuery>,
) -> Result<HttpResponse> {
    kernel::auth::require_permission_anywhere(state.pool(), perimetre.person_id, HIGHLIGHT_MANAGE)
        .await?;
    perimetre.ensure(requete.event_id)?;

    let seances = repo::seances(state.pool(), requete.event_id).await?;
    Ok(HttpResponse::Ok().json(seances))
}

/// Création.
#[utoipa::path(
    post,
    description = "`ShowcaseFormValues` → `ShowcaseWriteResult`. La diapositive se place **en fin d'emplacement** : la placer en tête déplacerait silencieusement tout le reste du bandeau. `placement_rows` rend l'emplacement entier renuméroté.\n\n**Les refus de validation sortent en 200**, avec leur champ et leur code : fenêtre inversée, organisation désignée ET nommée, libellé de lien sans lien, français manquant, couleur mal formée, contenu de plateforme sans portée globale.",
    path = "/admin/showcase",
    tag = "Back-office — vitrine",
    operation_id = "admin_vitrine_creer",
    responses(
        (status = 200, description = "ShowcaseWriteResult", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente, ou périmètre d'administration vide", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn creer(
    state: web::Data<ContentState>,
    requete: HttpRequest,
    perimetre: Perimeter,
    corps: web::Json<ShowcaseFormValues>,
) -> Result<HttpResponse> {
    kernel::auth::require_permission_anywhere(state.pool(), perimetre.person_id, HIGHLIGHT_MANAGE)
        .await?;

    let valeurs = corps.into_inner();
    let erreurs = service::valider(&valeurs, &perimetre.scope);
    if !erreurs.is_empty() {
        return Ok(HttpResponse::Ok().json(ShowcaseWriteResult::refuse(erreurs)));
    }

    let mut tx = state
        .db()
        .write(&crate::routes::contexte_de(&requete, perimetre.person_id))
        .await?;
    let id = repo::creer(&mut tx, &valeurs, perimetre.person_id).await?;
    repo::poser_les_themes(&mut tx, id, &valeurs.theme_codes).await?;
    repo::renumeroter(&mut tx, &valeurs.placement).await?;
    tx.commit().await?;

    let resultat = resultat_avec_ordre(&state, &perimetre, id, &valeurs.placement).await?;
    Ok(HttpResponse::Ok().json(resultat))
}

/// Modification.
#[utoipa::path(
    patch,
    description = "`ShowcaseFormValues` → `ShowcaseWriteResult`. Mêmes refus que la création. **Le périmètre se vérifie sur la source ET sur la cible** : on ne déplace pas une diapositive vers une édition qu'on n'administre pas, et on n'en fait pas un contenu de plateforme sans la portée globale.\n\n`published_at` ne se rejoue jamais : c'est le déclencheur du modèle qui le pose au premier passage en `published`.",
    path = "/admin/showcase/{id}",
    tag = "Back-office — vitrine",
    operation_id = "admin_vitrine_modifier",
    params(("id" = Uuid, Path, description = "Identifiant de la diapositive")),
    responses(
        (status = 200, description = "ShowcaseWriteResult", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente, périmètre vide, ou contenu de plateforme hors portée globale", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Diapositive inexistante **ou hors périmètre**", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn modifier(
    state: web::Data<ContentState>,
    requete: HttpRequest,
    perimetre: Perimeter,
    chemin: web::Path<Uuid>,
    corps: web::Json<ShowcaseFormValues>,
) -> Result<HttpResponse> {
    let id = chemin.into_inner();
    // La SOURCE d'abord : une diapositive qu'on n'a pas le droit de toucher ne
    // se déplace pas, même vers une édition qu'on administre.
    let _ = charger_dans_le_perimetre(&state, &perimetre, id).await?;

    let valeurs = corps.into_inner();
    let erreurs = service::valider(&valeurs, &perimetre.scope);
    if !erreurs.is_empty() {
        return Ok(HttpResponse::Ok().json(ShowcaseWriteResult::refuse(erreurs)));
    }

    let mut tx = state
        .db()
        .write(&crate::routes::contexte_de(&requete, perimetre.person_id))
        .await?;
    repo::modifier(&mut tx, id, &valeurs).await?;
    repo::poser_les_themes(&mut tx, id, &valeurs.theme_codes).await?;
    repo::renumeroter(&mut tx, &valeurs.placement).await?;
    tx.commit().await?;

    let resultat = resultat_avec_ordre(&state, &perimetre, id, &valeurs.placement).await?;
    Ok(HttpResponse::Ok().json(resultat))
}

/// Publier, retirer, archiver — depuis la liste, sans ouvrir le formulaire.
#[utoipa::path(
    post,
    description = "`ShowcaseStatusPayload` → `ShowcaseWriteResult`. Trois actes de diffusion, pas une modification de contenu : ils ne touchent ni les textes ni les médias, et restent possibles à une main depuis le tableau. `placement_rows` est nul — aucun ordre ne change.",
    path = "/admin/showcase/{id}/status",
    tag = "Back-office — vitrine",
    operation_id = "admin_vitrine_statut",
    params(("id" = Uuid, Path, description = "Identifiant de la diapositive")),
    responses(
        (status = 200, description = "ShowcaseWriteResult", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente, périmètre vide, ou contenu de plateforme hors portée globale", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Diapositive inexistante **ou hors périmètre**", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn changer_le_statut(
    state: web::Data<ContentState>,
    requete: HttpRequest,
    perimetre: Perimeter,
    chemin: web::Path<Uuid>,
    corps: web::Json<ShowcaseStatusPayload>,
) -> Result<HttpResponse> {
    let id = chemin.into_inner();
    let _ = charger_dans_le_perimetre(&state, &perimetre, id).await?;

    let statut = corps.status.clone();
    if !matches!(statut.as_str(), "draft" | "published" | "archived") {
        return Err(ApiError::validation("Statut inconnu.", "status"));
    }

    let mut tx = state
        .db()
        .write(&crate::routes::contexte_de(&requete, perimetre.person_id))
        .await?;
    repo::changer_le_statut(&mut tx, id, &statut).await?;
    tx.commit().await?;

    let ligne = ligne_courante(&state, &perimetre, id).await?;
    Ok(HttpResponse::Ok().json(ShowcaseWriteResult {
        ok: true,
        errors: Vec::new(),
        row: ligne,
        placement_rows: None,
    }))
}

/// Monter ou descendre d'un cran, dans son emplacement.
#[utoipa::path(
    post,
    description = "`ShowcaseReorderPayload` → `ShowcaseWriteResult`. L'ordre est la fonction principale de cet écran — son absence était le défaut n° 6 de la v1. **Aux extrémités, la réponse est `ok: true` sans changement** : les boutons y sont déjà désactivés, et une erreur pour une action que l'écran n'offrait pas serait du bruit.\n\n`placement_rows` rend l'emplacement **entier**, renuméroté : deux lignes ont bougé, et rafraîchir la seule ligne cliquée laisserait sa voisine afficher un rang faux.",
    path = "/admin/showcase/{id}/order",
    tag = "Back-office — vitrine",
    operation_id = "admin_vitrine_ordonner",
    params(("id" = Uuid, Path, description = "Identifiant de la diapositive")),
    responses(
        (status = 200, description = "ShowcaseWriteResult", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente, périmètre vide, ou contenu de plateforme hors portée globale", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Diapositive inexistante **ou hors périmètre**", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn deplacer(
    state: web::Data<ContentState>,
    requete: HttpRequest,
    perimetre: Perimeter,
    chemin: web::Path<Uuid>,
    corps: web::Json<ShowcaseReorderPayload>,
) -> Result<HttpResponse> {
    let id = chemin.into_inner();
    let valeurs = charger_dans_le_perimetre(&state, &perimetre, id).await?;

    let vers_le_haut = match corps.direction.as_str() {
        "up" => true,
        "down" => false,
        _ => return Err(ApiError::validation("Sens inconnu.", "direction")),
    };

    let mut tx = state
        .db()
        .write(&crate::routes::contexte_de(&requete, perimetre.person_id))
        .await?;
    repo::deplacer(&mut tx, id, vers_le_haut).await?;
    tx.commit().await?;

    let resultat = resultat_avec_ordre(&state, &perimetre, id, &valeurs.placement).await?;
    Ok(HttpResponse::Ok().json(resultat))
}

/// Dupliquer — le geste qui remet un témoignage de la COP30 à la COP31.
#[utoipa::path(
    post,
    description = "`ShowcaseWriteResult` — la copie part **en brouillon**, en fin d'emplacement : dupliquer un contenu publié et le voir sortir aussitôt sur l'accueil serait une publication que personne n'a demandée. Les thématiques suivent. `row` porte la COPIE, et `placement_rows` l'emplacement renuméroté.",
    path = "/admin/showcase/{id}/duplicate",
    tag = "Back-office — vitrine",
    operation_id = "admin_vitrine_dupliquer",
    params(("id" = Uuid, Path, description = "Identifiant de la diapositive à copier")),
    responses(
        (status = 200, description = "ShowcaseWriteResult", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente, périmètre vide, ou contenu de plateforme hors portée globale", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Diapositive inexistante **ou hors périmètre**", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn dupliquer(
    state: web::Data<ContentState>,
    requete: HttpRequest,
    perimetre: Perimeter,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let id = chemin.into_inner();
    let valeurs = charger_dans_le_perimetre(&state, &perimetre, id).await?;

    let mut tx = state
        .db()
        .write(&crate::routes::contexte_de(&requete, perimetre.person_id))
        .await?;
    let copie = repo::dupliquer(&mut tx, id, perimetre.person_id).await?;
    repo::renumeroter(&mut tx, &valeurs.placement).await?;
    tx.commit().await?;

    let copie = copie.ok_or_else(ApiError::not_found)?;
    let resultat = resultat_avec_ordre(&state, &perimetre, copie, &valeurs.placement).await?;
    Ok(HttpResponse::Ok().json(resultat))
}

// ---------------------------------------------------------------------------
// Composition — ce que plusieurs routes partagent
// ---------------------------------------------------------------------------

/// Charge une diapositive **après** avoir vérifié permission et périmètre.
///
/// L'ordre est imposé : permission, existence, périmètre. Une diapositive
/// inexistante et une diapositive hors périmètre rendent le même refus.
async fn charger_dans_le_perimetre(
    state: &ContentState,
    perimetre: &Perimeter,
    id: Uuid,
) -> Result<ShowcaseFormValues> {
    kernel::auth::require_permission_anywhere(state.pool(), perimetre.person_id, HIGHLIGHT_MANAGE)
        .await?;

    let event_id = repo::edition_de(state.pool(), id)
        .await?
        .ok_or_else(ApiError::not_found)?;
    service::assurer_le_perimetre(event_id, &perimetre.scope)?;

    repo::valeurs(state.pool(), id)
        .await?
        .ok_or_else(ApiError::not_found)
}

async fn composer_la_liste(
    state: &ContentState,
    perimetre: &Perimeter,
) -> Result<ShowcaseListScreen> {
    let maintenant = OffsetDateTime::now_utc();
    let rows = repo::lignes(
        state.pool(),
        perimetre.scope.is_global,
        &perimetre.scope.event_ids,
        maintenant,
    )
    .await?;

    let mut counts = serde_json::Map::new();
    let mut broadcast_counts = serde_json::Map::new();
    // Les cinq états sont posés à zéro d'abord : un état absent de la réponse
    // ferait afficher un tiret là où l'écran attend un décompte.
    for etat in ["live", "scheduled", "expired", "draft", "archived"] {
        broadcast_counts.insert(etat.to_owned(), serde_json::json!(0));
    }
    counts.insert("home_hero".to_owned(), serde_json::json!(0));

    for row in &rows {
        for (cle, table) in [
            (row.placement.clone(), &mut counts),
            (row.broadcast_state.clone(), &mut broadcast_counts),
        ] {
            let compte = table
                .get(&cle)
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(0);
            table.insert(cle, serde_json::json!(compte + 1));
        }
    }

    Ok(ShowcaseListScreen {
        rows,
        counts,
        broadcast_counts,
        natures: repo::natures(state.pool()).await?,
        events: repo::editions(
            state.pool(),
            perimetre.scope.is_global,
            &perimetre.scope.event_ids,
        )
        .await?,
        is_global_scope: perimetre.scope.is_global,
    })
}

async fn composer_le_formulaire(
    state: &ContentState,
    perimetre: &Perimeter,
    valeurs: ShowcaseFormValues,
    id: Option<Uuid>,
) -> Result<ShowcaseFormScreen> {
    let sessions = match valeurs.event_id {
        Some(event_id) => repo::seances(state.pool(), event_id).await?,
        None => Vec::new(),
    };

    Ok(ShowcaseFormScreen {
        preview: crate::repo::showcase::apercu(state.pool(), id, &valeurs).await?,
        natures: repo::natures(state.pool()).await?,
        events: repo::editions(
            state.pool(),
            perimetre.scope.is_global,
            &perimetre.scope.event_ids,
        )
        .await?,
        sessions,
        organizations: repo::organisations(state.pool()).await?,
        people: repo::personnes(state.pool()).await?,
        countries: repo::pays(state.pool()).await?,
        available_themes: repo::themes_disponibles(state.pool()).await?,
        media: repo::media(state.pool(), id).await?,
        is_global_scope: perimetre.scope.is_global,
        values: valeurs,
    })
}

async fn ligne_courante(
    state: &ContentState,
    perimetre: &Perimeter,
    id: Uuid,
) -> Result<Option<crate::domain::admin::ShowcaseListRow>> {
    repo::ligne(
        state.pool(),
        id,
        perimetre.scope.is_global,
        &perimetre.scope.event_ids,
        OffsetDateTime::now_utc(),
    )
    .await
}

/// Le résultat d'une écriture **qui touche à l'ordre** : la ligne, et
/// l'emplacement entier renuméroté.
async fn resultat_avec_ordre(
    state: &ContentState,
    perimetre: &Perimeter,
    id: Uuid,
    placement: &str,
) -> Result<ShowcaseWriteResult> {
    let maintenant = OffsetDateTime::now_utc();
    let toutes = repo::lignes(
        state.pool(),
        perimetre.scope.is_global,
        &perimetre.scope.event_ids,
        maintenant,
    )
    .await?;

    let row = toutes.iter().find(|r| r.id == id).cloned();
    let placement_rows = toutes
        .into_iter()
        .filter(|r| r.placement == placement)
        .collect();

    Ok(ShowcaseWriteResult {
        ok: true,
        errors: Vec::new(),
        row,
        placement_rows: Some(placement_rows),
    })
}
