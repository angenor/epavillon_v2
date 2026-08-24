//! Back-office de l'identité — lectures.
//!
//! Deux gardes, et ils ne disent pas la même chose : la **permission** ouvre
//! l'écran, le **périmètre** borne ce qu'on y voit. Un périmètre vide se refuse
//! à l'entrée par l'extracteur `Perimeter` — jamais par une liste vide, qui
//! ferait croire qu'il n'y a personne plutôt qu'aucun droit.

use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use kernel::auth::{Perimeter, Requires, RequiresAnyScope, ScopeType};
use kernel::context::RequestContext;
use kernel::error::Result;
use serde::Deserialize;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::ids::{PersonId, RoleAssignmentId};
use crate::domain::permissions::{PersonManage, PersonRead, RoleAssign};
use crate::domain::scope::AssignableStatus;
use crate::routes::locale_de;
use crate::service::{admin_users, rbac};
use crate::state::IdentityState;

fn contexte(requete: &HttpRequest) -> RequestContext {
    requete
        .extensions()
        .get::<RequestContext>()
        .cloned()
        .unwrap_or_else(|| RequestContext::new(RequestContext::generated_request_id(), "fr"))
}

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/admin/users")
            .route("", web::get().to(liste))
            // Déclarée AVANT `/{id}` : `role-options` s'apparierait sinon au
            // motif d'identifiant, et le refus serait un UUID mal formé.
            .route("/role-options", web::get().to(options_dattribution))
            .route("/roles/{assignment_id}", web::delete().to(retirer_role))
            .route("/{id}", web::get().to(fiche))
            .route(
                "/{id}/effective-permissions",
                web::get().to(permissions_effectives),
            )
            .route("/{id}/roles", web::post().to(attribuer_role))
            .route("/{id}/status", web::put().to(changer_le_statut)),
    );
}

/// **Aucun champ de droits.** Le site passe encore ses permissions et son
/// identifiant d'acteur aux fonctions de son client ; ils ne franchissent pas le
/// réseau, et s'ils le franchissaient, rien ici ne les lirait (FR-055). C'est
/// aussi pourquoi ces charges utiles ne déclarent pas `deny_unknown_fields` :
/// un champ inconnu s'ignore, il ne fait pas échouer l'écriture.
#[derive(Debug, Deserialize)]
pub struct GrantRolePayload {
    pub role_code: String,
    pub scope_type: ScopeType,
    #[serde(default)]
    pub scope_id: Option<Uuid>,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub valid_from: Option<OffsetDateTime>,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub valid_until: Option<OffsetDateTime>,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RevokeRolePayload {
    pub reason: String,
}

#[derive(Debug, Deserialize)]
pub struct SetPersonStatusPayload {
    pub status: AssignableStatus,
    pub reason: String,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub suspended_until: Option<OffsetDateTime>,
    #[serde(default)]
    pub revoke_sessions: bool,
}

#[utoipa::path(
    get,
    description = "`UserListScreen`, borné par le périmètre d'administration.",
    path = "/admin/users",
    tag = "Back-office — utilisateurs",
    operation_id = "admin_utilisateur_liste",
    responses(
        (status = 200, description = "UserListScreen", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Sans la permission, ou sur périmètre vide", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn liste(
    state: web::Data<IdentityState>,
    _permission: RequiresAnyScope<PersonRead>,
    perimetre: Perimeter,
    requete: HttpRequest,
) -> Result<HttpResponse> {
    let ecran =
        admin_users::list_screen(state.pool(), &locale_de(&requete), &perimetre.scope).await?;
    Ok(HttpResponse::Ok().json(ecran))
}

/// Hors périmètre, la fiche sort quand même — avec `in_scope` à faux, qui la
/// met en lecture seule côté écran. `null` ne dit qu'une chose : la personne
/// n'existe pas.
#[utoipa::path(
    get,
    description = "`UserDetail | null`. **Hors périmètre → 200 avec `in_scope: false`.**",
    path = "/admin/users/{id}",
    tag = "Back-office — utilisateurs",
    operation_id = "admin_utilisateur_fiche",
    params(("id" = Uuid, Path, description = "Identifiant de la personne")),
    responses(
        (status = 200, description = "UserDetail | null", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission ou portée insuffisante", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn fiche(
    state: web::Data<IdentityState>,
    _permission: RequiresAnyScope<PersonRead>,
    perimetre: Perimeter,
    chemin: web::Path<Uuid>,
    requete: HttpRequest,
) -> Result<HttpResponse> {
    let fiche = admin_users::detail(
        state.pool(),
        &locale_de(&requete),
        &perimetre.scope,
        PersonId(chemin.into_inner()),
    )
    .await?;

    Ok(HttpResponse::Ok().json(fiche))
}

/// **Deux gardes, et ils ne disent pas la même chose.** `identity.role.assign`
/// sur *au moins une* portée ouvre la route : sans elle, on n'est pas
/// administrateur du tout, et le refus est un **403** — pas un discriminant, qui
/// laisserait un compte ordinaire lire les rôles de n'importe qui en sondant
/// cette route. `forbidden_scope`, lui, répond à un administrateur qui vise une
/// portée hors de la sienne : c'est un refus qu'il peut comprendre et corriger.
///
/// **La personne visée vient de l'URL, jamais du corps.** Le site envoie encore
/// `person_id` dans sa charge utile ; le lire laisserait attribuer un rôle à
/// quelqu'un d'autre que la fiche ouverte.
///
/// Les six issues sortent en **200** : ce sont des refus prévus par le contrat
/// du site, et chacune dit quoi corriger.
#[utoipa::path(
    post,
    description = "`GrantRolePayload` → `RoleWriteResult`. **Les six issues sortent en 200.**",
    path = "/admin/users/{id}/roles",
    tag = "Back-office — utilisateurs",
    operation_id = "admin_utilisateur_attribuer_role",
    params(("id" = Uuid, Path, description = "Identifiant de la personne")),
    request_body = Object,
    responses(
        (status = 200, description = "RoleWriteResult — granted, duplicate, scope_not_allowed, forbidden_scope, not_found", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission ou portée insuffisante", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Requête invalide", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn attribuer_role(
    state: web::Data<IdentityState>,
    acteur: RequiresAnyScope<RoleAssign>,
    chemin: web::Path<Uuid>,
    corps: web::Json<GrantRolePayload>,
    requete: HttpRequest,
) -> Result<HttpResponse> {
    let ctx = contexte(&requete);
    let issue = admin_users::grant_role(
        &state,
        &ctx,
        acteur.person_id,
        admin_users::GrantRequest {
            person_id: PersonId(chemin.into_inner()),
            role_code: corps.role_code.trim(),
            scope_type: corps.scope_type,
            scope_id: corps.scope_id,
            valid_from: corps.valid_from,
            valid_until: corps.valid_until,
            note: corps
                .note
                .as_deref()
                .map(str::trim)
                .filter(|n| !n.is_empty()),
        },
    )
    .await?;

    Ok(HttpResponse::Ok().json(issue))
}

/// `DELETE` par la route, mais **pas une suppression** : la ligne reste, avec
/// son auteur et son motif de retrait.
#[utoipa::path(
    delete,
    description = "`RevokeRolePayload` → `RoleWriteResult`. **Pas une suppression** : la ligne reste.",
    path = "/admin/users/roles/{assignment_id}",
    tag = "Back-office — utilisateurs",
    operation_id = "admin_utilisateur_retirer_role",
    params(("assignment_id" = Uuid, Path, description = "Identifiant de l'attribution")),
    request_body = Object,
    responses(
        (status = 200, description = "RoleWriteResult", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission ou portée insuffisante", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Requête invalide", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn retirer_role(
    state: web::Data<IdentityState>,
    acteur: RequiresAnyScope<RoleAssign>,
    chemin: web::Path<Uuid>,
    corps: web::Json<RevokeRolePayload>,
    requete: HttpRequest,
) -> Result<HttpResponse> {
    let ctx = contexte(&requete);
    let issue = admin_users::revoke_role(
        &state,
        &ctx,
        acteur.person_id,
        RoleAssignmentId(chemin.into_inner()),
        corps.reason.trim(),
    )
    .await?;

    Ok(HttpResponse::Ok().json(issue))
}

/// **Portée globale.** Une suspension vaut sur toute la plateforme : il n'existe
/// aucune édition à laquelle la rapporter, et un administrateur détaché sur une
/// COP ne peut pas fermer un compte qui sert ailleurs.
#[utoipa::path(
    put,
    description = "`SetPersonStatusPayload` → `PersonWriteResult`. Portée **globale**.",
    path = "/admin/users/{id}/status",
    tag = "Back-office — utilisateurs",
    operation_id = "admin_utilisateur_changer_le_statut",
    params(("id" = Uuid, Path, description = "Identifiant de la personne")),
    request_body = Object,
    responses(
        (status = 200, description = "PersonWriteResult — saved, missing_deadline, not_found", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission ou portée insuffisante", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Requête invalide", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn changer_le_statut(
    state: web::Data<IdentityState>,
    permission: Requires<PersonManage>,
    perimetre: Perimeter,
    chemin: web::Path<Uuid>,
    corps: web::Json<SetPersonStatusPayload>,
    requete: HttpRequest,
) -> Result<HttpResponse> {
    let ctx = contexte(&requete);
    let issue = admin_users::set_status(
        &state,
        &ctx,
        permission.person_id,
        &perimetre.scope,
        admin_users::StatusRequest {
            person_id: PersonId(chemin.into_inner()),
            status: corps.status.en_statut(),
            reason: corps.reason.trim(),
            suspended_until: corps.suspended_until,
            revoke_sessions: corps.revoke_sessions,
        },
    )
    .await?;

    Ok(HttpResponse::Ok().json(issue))
}

/// Ce que l'appelant peut réellement accorder — rien de plus.
#[utoipa::path(
    get,
    description = "`RoleAssignmentOptions` — restreint à ce que l'appelant peut accorder.",
    path = "/admin/users/role-options",
    tag = "Back-office — utilisateurs",
    operation_id = "admin_utilisateur_options_dattribution",
    responses(
        (status = 200, description = "RoleAssignmentOptions", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission ou portée insuffisante", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn options_dattribution(
    state: web::Data<IdentityState>,
    permission: RequiresAnyScope<RoleAssign>,
    requete: HttpRequest,
) -> Result<HttpResponse> {
    let options =
        admin_users::role_options(state.pool(), &locale_de(&requete), permission.person_id).await?;

    Ok(HttpResponse::Ok().json(options))
}

#[utoipa::path(
    get,
    description = "`EffectivePermissionsView`.",
    path = "/admin/users/{id}/effective-permissions",
    tag = "Back-office — utilisateurs",
    operation_id = "admin_utilisateur_permissions_effectives",
    params(("id" = Uuid, Path, description = "Identifiant de la personne")),
    responses(
        (status = 200, description = "EffectivePermissionsView", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission ou portée insuffisante", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn permissions_effectives(
    state: web::Data<IdentityState>,
    _permission: RequiresAnyScope<PersonRead>,
    _perimetre: Perimeter,
    chemin: web::Path<Uuid>,
    requete: HttpRequest,
) -> Result<HttpResponse> {
    let vue = rbac::effective_permissions_view(
        state.pool(),
        &locale_de(&requete),
        PersonId(chemin.into_inner()),
    )
    .await?;

    Ok(HttpResponse::Ok().json(vue))
}
