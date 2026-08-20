//! Le back-office des organisations.
//!
//! **La garde est double, et les trois cas du périmètre restent distincts** :
//! permission de consultation sur une portée quelconque **et** périmètre
//! d'administration non vide. La permission seule ne suffit pas — le rôle
//! d'utilisateur ordinaire la détient (écart n° 73) — et un périmètre vide se
//! refuse explicitement, jamais par une liste vide (FR-043).
//!
//! L'extracteur `Perimeter` du noyau porte la seconde moitié : il refuse
//! lui-même un périmètre vide.

use actix_web::{web, HttpResponse};
use kernel::auth::{Actor, Perimeter, Requires};
use kernel::error::Result;

use crate::domain::admin::{DomainVerification, NameConfirmation, OrganizationVerification};
use crate::domain::duplicates::DuplicateDecision;
use crate::domain::ids::{OrganizationDomainId, OrganizationId, OrganizationNameId, PersonId};
use crate::domain::merge::MergePayload;
use crate::domain::permissions::{OrganizationMerge, ORGANIZATION_MANAGE, ORGANIZATION_READ};
use crate::routes::public::RechercheQuery;
use crate::service::{admin_detail, admin_list, admin_write, duplicates, merge, search};
use crate::state::OrgState;

/// Le scope `/admin/organizations` — **rempli, jamais créé ici**.
///
/// Deux `web::scope` du même préfixe ne se complètent pas : Actix retient le
/// premier dont le préfixe correspond et rend 404 si la route n'y figure pas.
/// Le scope est donc monté une seule fois, dans `lib.rs`.
///
/// **Les chemins littéraux précèdent les chemins paramétrés** : `/similar`,
/// `/duplicates` et `/merge` avant `/{id}`, sans quoi ils seraient lus comme des
/// identifiants d'organisation et rendraient un refus incompréhensible.
pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/similar", web::get().to(similaires_non_filtrees))
        .route("", web::get().to(liste))
        .route("/duplicates", web::get().to(file_des_doublons))
        .route("/duplicates/{pairId}", web::put().to(decision_de_doublon))
        .route("/merge", web::post().to(fusionner))
        .route("/{id}", web::get().to(fiche))
        .route("/{id}/merge-preview", web::get().to(apercu_de_fusion))
        .route("/{id}/verification", web::put().to(verification))
        .route("/{id}/domains/{domainId}", web::put().to(domaine))
        .route("/{id}/names/{nameId}", web::put().to(denomination));
}

/// La liste du back-office, **bornée par le périmètre**.
#[utoipa::path(
    get,
    description = "`OrganizationListScreen`. Filtrée par périmètre — **organisations ayant déposé ou tenu une activité** dans les éditions administrées : une organisation n'appartient à aucune édition, c'est l'activité qui la rattache. `scoped_to_events` dit que la liste est restreinte. Les facettes sont comptées sur le même jeu de lignes. Un périmètre vide se refuse explicitement, **jamais par une liste vide**.",
    path = "/admin/organizations",
    tag = "Back-office — organisations",
    operation_id = "admin_organisations_liste",
    responses(
        (status = 200, description = "OrganizationListScreen", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente, ou périmètre d'administration vide", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn liste(
    state: web::Data<OrgState>,
    perimetre: Perimeter,
) -> Result<HttpResponse> {
    kernel::auth::require_permission_anywhere(state.pool(), perimetre.person_id, ORGANIZATION_READ)
        .await?;

    let ecran = admin_list::screen(state.pool(), &perimetre.scope).await?;
    Ok(HttpResponse::Ok().json(ecran))
}

/// La fiche entière — huit lectures assemblées.
///
/// **Une fiche hors périmètre rend un refus indiscernable d'une fiche
/// inexistante**, URL forgée comprise.
#[utoipa::path(
    get,
    description = "`OrganizationDetail | null` — huit lectures assemblées. Une fiche **absorbée** s'ouvre normalement, coiffée de son renvoi vers la fiche vivante. Une fiche **hors périmètre** rend `null`, indiscernable d'une fiche inexistante — URL forgée comprise.",
    path = "/admin/organizations/{id}",
    tag = "Back-office — organisations",
    operation_id = "admin_organisation_fiche",
    params(("id" = uuid::Uuid, Path, description = "Identifiant de l'organisation")),
    responses(
        (status = 200, description = "OrganizationDetail | null", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente, ou périmètre d'administration vide", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn fiche(
    state: web::Data<OrgState>,
    perimetre: Perimeter,
    chemin: web::Path<uuid::Uuid>,
) -> Result<HttpResponse> {
    kernel::auth::require_permission_anywhere(state.pool(), perimetre.person_id, ORGANIZATION_READ)
        .await?;

    let fiche = admin_detail::detail(
        state.pool(),
        &perimetre.scope,
        OrganizationId(chemin.into_inner()),
    )
    .await?;

    Ok(HttpResponse::Ok().json(fiche))
}

/// **La seconde des deux lectures de recherche** (écart n° 23).
///
/// Celle-ci répond à « qu'est-ce qui pourrait être la même entité ? », et ne
/// filtre **rien**. Le domaine partagé fait entrer la fiche, et c'est voulu :
/// deux fiches déclarant `osed-sahel.org` sont la même maison, quels que soient
/// les libellés saisis. C'est aussi la lecture qu'emploie le balayage de fond.
#[utoipa::path(
    get,
    description = "`SimilarOrganization[]` — **lecture non filtrée** : le domaine partagé fait entrer la fiche, c'est le signal le plus fiable du modèle. À l'inverse de `/organizations/similar`, qui n'admet que les ressemblances de dénomination.",
    path = "/admin/organizations/similar",
    tag = "Back-office — organisations",
    operation_id = "admin_organisations_similaires",
    params(
        ("name" = String, Query, description = "Terme cherché"),
        ("country_id" = Option<uuid::Uuid>, Query, description = "Pays"),
        ("email" = Option<String>, Query, description = "Adresse dont le domaine fait entrer la fiche"),
        ("website" = Option<String>, Query, description = "Site dont le domaine fait entrer la fiche"),
        ("limit" = Option<i32>, Query, description = "Défaut 10, maximum 50"),
    ),
    responses(
        (status = 200, description = "SimilarOrganization[]", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente, ou périmètre d'administration vide", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn similaires_non_filtrees(
    state: web::Data<OrgState>,
    perimetre: Perimeter,
    query: web::Query<RechercheQuery>,
) -> Result<HttpResponse> {
    kernel::auth::require_permission_anywhere(state.pool(), perimetre.person_id, ORGANIZATION_READ)
        .await?;

    let resultats = search::similar_for_review(state.pool(), query.into_inner().into()).await?;
    Ok(HttpResponse::Ok().json(resultats))
}

/// Pose ou retire le sceau. **Permission de gestion.**
#[utoipa::path(
    put,
    description = "`OrganizationWriteResult` — la fiche entière recomposée. Poser le sceau sur une fiche `candidate` l'**admet** du même geste ; le retirer ne change pas le statut.",
    path = "/admin/organizations/{id}/verification",
    tag = "Back-office — organisations",
    operation_id = "admin_organisation_verification",
    params(("id" = uuid::Uuid, Path, description = "Organisation")),
    request_body = Object,
    responses(
        (status = 200, description = "OrganizationWriteResult", body = Object),
        (status = 403, description = "Permission de gestion absente, ou périmètre vide", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn verification(
    state: web::Data<OrgState>,
    perimetre: Perimeter,
    chemin: web::Path<uuid::Uuid>,
    corps: web::Json<OrganizationVerification>,
    requete: actix_web::HttpRequest,
) -> Result<HttpResponse> {
    kernel::auth::require_permission_anywhere(
        state.pool(),
        perimetre.person_id,
        ORGANIZATION_MANAGE,
    )
    .await?;

    let issue = admin_write::set_verification(
        &state,
        &crate::routes::contexte_de(&requete, perimetre.person_id),
        &perimetre.scope,
        PersonId(perimetre.person_id),
        OrganizationId(chemin.into_inner()),
        corps.into_inner(),
    )
    .await?;

    Ok(HttpResponse::Ok().json(issue))
}

/// Vérifie un domaine, et règle son rattachement automatique. **Permission de
/// gestion.**
#[utoipa::path(
    put,
    description = "`OrganizationWriteResult`. `domain_taken` **nomme la fiche** qui détient déjà le domaine vérifié — sans ce nom, le refus est incompréhensible. Le rattachement automatique sur un domaine non vérifié rend `ORG_DOMAIN_VERIFICATION_REQUIRED`. Seule la méthode `manual` est livrée.",
    path = "/admin/organizations/{id}/domains/{domainId}",
    tag = "Back-office — organisations",
    operation_id = "admin_organisation_domaine",
    params(
        ("id" = uuid::Uuid, Path, description = "Organisation"),
        ("domainId" = uuid::Uuid, Path, description = "Domaine"),
    ),
    request_body = Object,
    responses(
        (status = 200, description = "OrganizationWriteResult", body = Object),
        (status = 403, description = "Permission de gestion absente, ou périmètre vide", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "ORG_DOMAIN_VERIFICATION_REQUIRED", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn domaine(
    state: web::Data<OrgState>,
    perimetre: Perimeter,
    chemin: web::Path<(uuid::Uuid, uuid::Uuid)>,
    corps: web::Json<DomainVerification>,
    requete: actix_web::HttpRequest,
) -> Result<HttpResponse> {
    kernel::auth::require_permission_anywhere(
        state.pool(),
        perimetre.person_id,
        ORGANIZATION_MANAGE,
    )
    .await?;

    let (organisation, domain_id) = chemin.into_inner();
    let issue = admin_write::set_domain(
        &state,
        &crate::routes::contexte_de(&requete, perimetre.person_id),
        &perimetre.scope,
        OrganizationId(organisation),
        OrganizationDomainId(domain_id),
        corps.into_inner(),
    )
    .await?;

    Ok(HttpResponse::Ok().json(issue))
}

/// Confirme une dénomination. **Permission de gestion.**
#[utoipa::path(
    put,
    description = "`OrganizationWriteResult`. Une dénomination **posée par la base** — le nom légal, le sigle — ne se retire pas à la main : `ORG_NAME_IS_DERIVED`.",
    path = "/admin/organizations/{id}/names/{nameId}",
    tag = "Back-office — organisations",
    operation_id = "admin_organisation_denomination",
    params(
        ("id" = uuid::Uuid, Path, description = "Organisation"),
        ("nameId" = uuid::Uuid, Path, description = "Dénomination"),
    ),
    request_body = Object,
    responses(
        (status = 200, description = "OrganizationWriteResult", body = Object),
        (status = 403, description = "Permission de gestion absente, ou périmètre vide", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "ORG_NAME_IS_DERIVED", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn denomination(
    state: web::Data<OrgState>,
    perimetre: Perimeter,
    chemin: web::Path<(uuid::Uuid, uuid::Uuid)>,
    corps: web::Json<NameConfirmation>,
    requete: actix_web::HttpRequest,
) -> Result<HttpResponse> {
    kernel::auth::require_permission_anywhere(
        state.pool(),
        perimetre.person_id,
        ORGANIZATION_MANAGE,
    )
    .await?;

    let (organisation, name_id) = chemin.into_inner();
    let issue = admin_write::set_name_confirmation(
        &state,
        &crate::routes::contexte_de(&requete, perimetre.person_id),
        &perimetre.scope,
        OrganizationId(organisation),
        OrganizationNameId(name_id),
        corps.into_inner(),
    )
    .await?;

    Ok(HttpResponse::Ok().json(issue))
}

/// La file des doublons. **Permission de fusion, portée GLOBALE.**
///
/// Elle n'est pas filtrée par périmètre, et ce n'est pas un oubli : une paire ne
/// relève d'aucune édition, et sa résolution exige de toute façon la portée
/// globale. Un administrateur détaché n'y accède **pas du tout**, plutôt que
/// d'en voir une part qui ne voudrait rien dire.
#[utoipa::path(
    get,
    description = "`DuplicateQueueScreen` — deux sections : en attente (triées par similarité décroissante) et déjà tranchées. **Permission de fusion en portée globale.**",
    path = "/admin/organizations/duplicates",
    tag = "Back-office — organisations",
    operation_id = "admin_file_des_doublons",
    responses(
        (status = 200, description = "DuplicateQueueScreen", body = Object),
        (status = 403, description = "Permission de fusion absente, ou détenue sur une portée qui n'est pas globale", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn file_des_doublons(
    state: web::Data<OrgState>,
    acteur: Requires<OrganizationMerge>,
) -> Result<HttpResponse> {
    let _ = acteur;
    let ecran = duplicates::queue(&state).await?;
    Ok(HttpResponse::Ok().json(ecran))
}

/// Tranche une paire. **Permission de fusion, portée globale.**
#[utoipa::path(
    put,
    description = "`DuplicateDecisionResult`. `distinct` retire la paire de la file — le balayage ne la ressuscite pas ; `deferred` la met de côté. Rien n'est définitif : `deferred` posé sur une paire **déjà sortie** de la file l'y **ramène**, écartée comme reportée. Une paire **fusionnée** ne se rejuge pas — la réécrire effacerait la trace de la fusion sans la défaire. `merged` ne se pose jamais ici : c'est la fusion qui l'écrit.",
    path = "/admin/organizations/duplicates/{pairId}",
    tag = "Back-office — organisations",
    operation_id = "admin_decision_de_doublon",
    params(("pairId" = uuid::Uuid, Path, description = "Paire")),
    request_body = Object,
    responses(
        (status = 200, description = "DuplicateDecisionResult", body = Object),
        (status = 403, description = "Permission de fusion absente, ou portée non globale", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Décision hors des deux valeurs recevables", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn decision_de_doublon(
    state: web::Data<OrgState>,
    acteur: Requires<OrganizationMerge>,
    chemin: web::Path<uuid::Uuid>,
    corps: web::Json<DuplicateDecision>,
    requete: actix_web::HttpRequest,
) -> Result<HttpResponse> {
    let demande = DuplicateDecision {
        pair_id: Some(chemin.into_inner()),
        ..corps.into_inner()
    };

    let issue = duplicates::decide(
        &state,
        &crate::routes::contexte_de(&requete, acteur.person_id),
        PersonId(acteur.person_id),
        demande,
    )
    .await?;

    Ok(HttpResponse::Ok().json(issue))
}

/// L'aperçu de fusion, **pour un sens donné**. Permission de fusion, portée
/// globale.
#[utoipa::path(
    get,
    description = "`MergePreview | null` — calculé pour un sens, **recalculé à l'inversion** : le décompte n'est pas symétrique. `null` si l'une des fiches est introuvable ou déjà absorbée. Les avertissements sont **non bloquants** : l'écran ne décide pas à la place de l'équipe.",
    path = "/admin/organizations/{id}/merge-preview",
    tag = "Back-office — organisations",
    operation_id = "admin_apercu_de_fusion",
    params(
        ("id" = uuid::Uuid, Path, description = "Fiche **absorbée**"),
        ("target_id" = uuid::Uuid, Query, description = "Fiche **absorbante**"),
        ("pair_id" = Option<uuid::Uuid>, Query, description = "Paire de la file d'où vient la fusion"),
    ),
    responses(
        (status = 200, description = "MergePreview | null", body = Object),
        (status = 403, description = "ORG_MERGE_GLOBAL_SCOPE_REQUIRED", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn apercu_de_fusion(
    state: web::Data<OrgState>,
    acteur: Actor,
    chemin: web::Path<uuid::Uuid>,
    query: web::Query<ApercuQuery>,
) -> Result<HttpResponse> {
    let q = query.into_inner();
    let apercu = merge::preview(
        &state,
        PersonId(acteur.0),
        OrganizationId(chemin.into_inner()),
        OrganizationId(q.target_id),
        q.pair_id,
    )
    .await?;

    Ok(HttpResponse::Ok().json(apercu))
}

#[derive(Debug, serde::Deserialize)]
pub struct ApercuQuery {
    pub target_id: uuid::Uuid,
    #[serde(default)]
    pub pair_id: Option<uuid::Uuid>,
}

/// La fusion. Permission de fusion, **portée globale**.
#[utoipa::path(
    post,
    description = "`MergeResult` — quatre issues. Un choix portant sur l'**adresse d'URL** est refusé en 422, champ nommé : elle reste celle de la fiche absorbée, et c'est ce qui fait que ses anciens liens continuent de fonctionner. Les arbitrages de champ s'appliquent **après** l'appel de la fonction de base, dans la même transaction.",
    path = "/admin/organizations/merge",
    tag = "Back-office — organisations",
    operation_id = "admin_fusionner",
    request_body = Object,
    responses(
        (status = 200, description = "MergeResult", body = Object),
        (status = 403, description = "ORG_MERGE_GLOBAL_SCOPE_REQUIRED", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "ORG_MERGE_FIELD_NOT_ARBITRABLE ou ORG_MERGE_SAME_ORGANIZATION", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn fusionner(
    state: web::Data<OrgState>,
    acteur: Actor,
    corps: web::Json<MergePayload>,
    requete: actix_web::HttpRequest,
) -> Result<HttpResponse> {
    let issue = merge::merge(
        &state,
        &crate::routes::contexte(&requete, acteur),
        PersonId(acteur.0),
        corps.into_inner(),
    )
    .await?;

    Ok(HttpResponse::Ok().json(issue))
}
