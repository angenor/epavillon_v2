//! Lectures d'identité.
//!
//! **« Soi-même » est décidé par la session, jamais par un paramètre.** C'est
//! ce qui empêche de lire le périmètre d'administration d'un autre en forgeant
//! son identifiant : l'égalité se teste contre l'acteur résolu par
//! l'intergiciel, et un identifiant qui ne lui correspond pas retombe sur la
//! permission — qu'il faut alors détenir.

use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use kernel::auth::Actor;
use kernel::context::RequestContext;
use kernel::error::Result;
use uuid::Uuid;

use crate::domain::ids::PersonId;
use crate::domain::permissions::PERSON_READ;
use crate::domain::person::ProfileChanges;
use crate::repo::people;
use crate::routes::locale_de;
use crate::service::{profile, rbac};
use crate::state::IdentityState;

/// Les routes de lecture d'identité, **sans leur scope**.
///
/// Le préfixe `/people` est partagé avec le module Organisations, qui y ajoute
/// les adhésions d'une personne. Or deux `web::scope("/people")` enregistrés
/// séparément **ne se complètent pas** : Actix retient le premier dont le
/// préfixe correspond et rend 404 si la route n'y figure pas. Le scope est donc
/// composé une seule fois, par l'API.
pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("", web::get().to(lister))
        .route("/{id}", web::get().to(fiche))
        .route("/{id}", web::put().to(modifier))
        .route("/{id}/roles", web::get().to(roles))
        .route("/{id}/permissions", web::get().to(permissions))
        .route("/{id}/administered-events", web::get().to(perimetre));
}

/// La liste des personnes. Elle n'est pas bornée par le périmètre
/// d'administration : c'est `/admin/users` qui porte la liste du back-office et
/// son filtrage. Ici, la permission — quelle que soit sa portée — ouvre la
/// lecture, et rien de plus qu'une fiche publique n'en sort.
#[utoipa::path(
    get,
    description = "`Person[]`.",
    path = "/people",
    tag = "Identité",
    operation_id = "lister",
    responses(
        (status = 200, description = "Person[]", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission ou portée insuffisante", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn lister(state: web::Data<IdentityState>, acteur: Actor) -> Result<HttpResponse> {
    kernel::auth::require_permission_anywhere(state.pool(), acteur.0, PERSON_READ).await?;
    let personnes = people::list(state.pool()).await?;
    Ok(HttpResponse::Ok().json(personnes))
}

#[utoipa::path(
    get,
    description = "`Person | null` — soi-même, ou `identity.person.read`.",
    path = "/people/{id}",
    tag = "Identité",
    operation_id = "fiche",
    params(("id" = Uuid, Path, description = "Identifiant de la personne")),
    responses(
        (status = 200, description = "Person | null", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission ou portée insuffisante", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn fiche(
    state: web::Data<IdentityState>,
    acteur: Actor,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let cible = soi_meme_ou_permission(&state, acteur, chemin.into_inner()).await?;
    let personne = people::view(state.pool(), cible).await?;
    Ok(HttpResponse::Ok().json(personne))
}

#[utoipa::path(
    get,
    description = "`RoleAssignmentView[]` — attributions en cours, avec le libellé du rôle, la cible de la portée résolue et qui l'a confiée. **Ce n'est pas `RoleAssignment`**, la ligne nue de la table : une pastille ne porte jamais un rôle sans sa PORTÉE, et la résoudre côté site demanderait une lecture par attribution.",
    path = "/people/{id}/roles",
    tag = "Identité",
    operation_id = "roles",
    params(("id" = Uuid, Path, description = "Identifiant de la personne")),
    responses(
        (status = 200, description = "RoleAssignmentView[]", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission ou portée insuffisante", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn roles(
    state: web::Data<IdentityState>,
    acteur: Actor,
    chemin: web::Path<Uuid>,
    requete: actix_web::HttpRequest,
) -> Result<HttpResponse> {
    let cible = soi_meme_ou_permission(&state, acteur, chemin.into_inner()).await?;
    let attributions = rbac::assignments(state.pool(), &locale_de(&requete), cible, true).await?;
    Ok(HttpResponse::Ok().json(attributions))
}

#[utoipa::path(
    get,
    description = "`EffectivePermission[]`.",
    path = "/people/{id}/permissions",
    tag = "Identité",
    operation_id = "permissions",
    params(("id" = Uuid, Path, description = "Identifiant de la personne")),
    responses(
        (status = 200, description = "EffectivePermission[]", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission ou portée insuffisante", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn permissions(
    state: web::Data<IdentityState>,
    acteur: Actor,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let cible = soi_meme_ou_permission(&state, acteur, chemin.into_inner()).await?;
    let effectives = rbac::effective_permissions(state.pool(), cible).await?;
    Ok(HttpResponse::Ok().json(effectives))
}

/// **Jamais nul, toujours une valeur pleine** : les trois cas du périmètre se
/// lisent sans ambiguïté côté site, et « aucun droit » ne se confond pas avec
/// « réponse absente ».
#[utoipa::path(
    get,
    description = "`AdministeredEvents` — **jamais nul, toujours une valeur pleine**.",
    path = "/people/{id}/administered-events",
    tag = "Identité",
    operation_id = "perimetre",
    params(("id" = Uuid, Path, description = "Identifiant de la personne")),
    responses(
        (status = 200, description = "AdministeredEvents", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission ou portée insuffisante", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn perimetre(
    state: web::Data<IdentityState>,
    acteur: Actor,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let cible = soi_meme_ou_permission(&state, acteur, chemin.into_inner()).await?;
    let administre = rbac::administered_events(state.pool(), cible).await?;
    Ok(HttpResponse::Ok().json(administre))
}

/// Le profil, modifié par la personne elle-même.
#[utoipa::path(
    put,
    description = "`UpdateProfilePayload` → `Person`. Soi-même seulement ; l'adresse principale ne se change pas ici.",
    path = "/people/{id}",
    tag = "Identité",
    operation_id = "modifier_son_profil",
    params(("id" = Uuid, Path, description = "Identifiant de la personne connectée")),
    request_body = Object,
    responses(
        (status = 200, description = "Person", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Ce n'est pas votre profil", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Champ refusé, nommé dans la réponse", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn modifier(
    requete: HttpRequest,
    state: web::Data<IdentityState>,
    acteur: Actor,
    chemin: web::Path<Uuid>,
    corps: web::Json<ProfileChanges>,
) -> Result<HttpResponse> {
    let ctx = requete
        .extensions()
        .get::<RequestContext>()
        .cloned()
        .unwrap_or_else(|| RequestContext::new(RequestContext::generated_request_id(), "fr"));
    let personne = profile::update(&state, &ctx, acteur.0, chemin.into_inner(), &corps).await?;
    Ok(HttpResponse::Ok().json(personne))
}

/// Soi-même, ou la permission de lire les personnes. Le refus est un 403 : la
/// personne visée existe ou non, la réponse ne le dit pas.
async fn soi_meme_ou_permission(
    state: &IdentityState,
    acteur: Actor,
    cible: Uuid,
) -> Result<PersonId> {
    if cible != acteur.0 {
        kernel::auth::require_permission_anywhere(state.pool(), acteur.0, PERSON_READ).await?;
    }
    Ok(PersonId(cible))
}
