//! Les adhésions : rejoindre, inviter, décider, accepter, quitter.
//!
//! **Deux files, deux autorisations.** Un référent tranche ce qu'il a reçu ; une
//! personne accepte ce qu'on lui a envoyé. Aucune route ne permet de faire
//! l'autre geste — c'est ce que `invited_at` protège.

use actix_web::{web, HttpResponse};
use kernel::auth::Actor;
use kernel::error::Result;
use uuid::Uuid;

use crate::domain::ids::{MembershipId, OrganizationId, PersonId};
use crate::domain::membership::{
    AcceptInvitation, DecideMembership, InviteMember, JoinOrganization,
};
use crate::repo::memberships;
use crate::service::{join, membership};
use crate::state::OrgState;

/// Les routes d'adhésion qui vivent sous `/organizations`.
///
/// **Le chemin littéral précède le chemin paramétré** : `/invitations/accept`
/// avant `/{id}/…`, sans quoi « invitations » serait lu comme un identifiant.
pub fn organisations(cfg: &mut web::ServiceConfig) {
    cfg.route("/invitations/accept", web::post().to(accepter_invitation))
        .route("/{id}/members", web::post().to(rejoindre))
        .route("/{id}/invitations", web::post().to(inviter));
}

/// Le scope `/memberships`.
pub fn adhesions(cfg: &mut web::ServiceConfig) {
    cfg.route("/{id}/decision", web::put().to(decider))
        .route("/{id}", web::delete().to(revoquer));
}

/// Le scope `/people` — que le module `identity` monte aussi. Même piège que
/// pour `/organizations` : les deux ne se complètent pas, et c'est `lib.rs` qui
/// décide où celle-ci atterrit.
pub fn personnes(cfg: &mut web::ServiceConfig) {
    cfg.route("/{id}/memberships", web::get().to(mes_adhesions));
}

/// Rejoindre une organisation. **Trois issues**, et `pending` n'est pas un
/// échec : c'est le fonctionnement normal quand le domaine ne prouve rien.
#[utoipa::path(
    post,
    description = "`JoinOrganizationResult` — trois issues. L'organisation visée est **résolue** : rejoindre une fiche absorbée mène à la fiche vivante.",
    path = "/organizations/{id}/members",
    tag = "Organisations",
    operation_id = "organisation_rejoindre",
    params(("id" = Uuid, Path, description = "Organisation visée")),
    request_body = Object,
    responses(
        (status = 200, description = "JoinOrganizationResult", body = Object),
        (status = 401, description = "Aucune session", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Organisation inexistante", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn rejoindre(
    state: web::Data<OrgState>,
    acteur: Actor,
    chemin: web::Path<Uuid>,
    corps: web::Json<JoinOrganization>,
    requete: actix_web::HttpRequest,
) -> Result<HttpResponse> {
    // **L'identifiant du chemin fait foi.** Le corps porte le même champ — le
    // contrat du front le tient des données simulées, qui n'ont pas de chemin —
    // et deux sources pour une seule vérité finissent par diverger. Il est donc
    // facultatif dans le corps, et ignoré quand il y est.
    let issue = join::join(
        &state,
        &crate::routes::contexte(&requete, acteur),
        PersonId(acteur.0),
        OrganizationId(chemin.into_inner()),
        corps.into_inner(),
    )
    .await?;

    Ok(HttpResponse::Ok().json(issue))
}

/// Les adhésions d'une personne. **Soi-même**, ou la permission de consultation
/// des utilisateurs — décidée par la session, jamais par le paramètre.
#[utoipa::path(
    get,
    description = "`Membership[]` — adhésions vivantes : actives et en attente. Soi-même, ou `identity.person.read`.",
    path = "/people/{id}/memberships",
    tag = "Organisations",
    operation_id = "personne_adhesions",
    params(("id" = Uuid, Path, description = "Identifiant de la personne")),
    responses(
        (status = 200, description = "Membership[]", body = Object),
        (status = 401, description = "Aucune session", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Ni soi-même, ni la permission", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn mes_adhesions(
    state: web::Data<OrgState>,
    acteur: Actor,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let cible = chemin.into_inner();
    if cible != acteur.0 {
        // La permission appartient au module `identity` — c'est son entité. On
        // la teste par le garde du noyau, jamais en dépendant de son crate.
        kernel::auth::require_permission_anywhere(state.pool(), acteur.0, "identity.person.read")
            .await?;
    }

    let adhesions = memberships::of_person(state.pool(), PersonId(cible)).await?;
    Ok(HttpResponse::Ok().json(adhesions))
}

/// Inviter quelqu'un par son adresse. **Référent actif de cette organisation.**
#[utoipa::path(
    post,
    description = "`InviteMemberResult` — trois issues. Crée la personne si l'adresse est inconnue, **sans compte et sans nom déduit de l'adresse**. `already_invited` propose de relancer, jamais d'émettre une seconde invitation.",
    path = "/organizations/{id}/invitations",
    tag = "Organisations",
    operation_id = "organisation_inviter",
    params(("id" = Uuid, Path, description = "Organisation")),
    request_body = Object,
    responses(
        (status = 200, description = "InviteMemberResult", body = Object),
        (status = 401, description = "Aucune session", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "ORG_NOT_MANAGER — pas référent actif de cette organisation", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn inviter(
    state: web::Data<OrgState>,
    acteur: Actor,
    chemin: web::Path<Uuid>,
    corps: web::Json<InviteMember>,
    requete: actix_web::HttpRequest,
) -> Result<HttpResponse> {
    let issue = membership::invite(
        &state,
        &crate::routes::contexte(&requete, acteur),
        PersonId(acteur.0),
        OrganizationId(chemin.into_inner()),
        corps.into_inner(),
    )
    .await?;

    Ok(HttpResponse::Ok().json(issue))
}

/// La décision d'un référent sur une **demande**.
///
/// Sur une invitation, refus explicite : elle attend la réponse de la personne,
/// pas celle de l'organisation. C'est le refus qui empêche de faire entrer
/// quelqu'un qui n'a rien accepté.
#[utoipa::path(
    put,
    description = "`Membership | null`. **Ne porte que sur une demande** (`invited_at` nul) : sur une invitation, `ORG_MEMBERSHIP_IS_INVITATION`. Un refus **révoque**, il ne supprime pas.",
    path = "/memberships/{id}/decision",
    tag = "Organisations",
    operation_id = "adhesion_decision",
    params(("id" = Uuid, Path, description = "Adhésion")),
    request_body = Object,
    responses(
        (status = 200, description = "Membership | null", body = Object),
        (status = 401, description = "Aucune session", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "ORG_NOT_MANAGER", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "ORG_MEMBERSHIP_IS_INVITATION ou ORG_MEMBERSHIP_NOT_PENDING", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn decider(
    state: web::Data<OrgState>,
    acteur: Actor,
    chemin: web::Path<Uuid>,
    corps: web::Json<DecideMembership>,
    requete: actix_web::HttpRequest,
) -> Result<HttpResponse> {
    let adhesion = membership::decide(
        &state,
        &crate::routes::contexte(&requete, acteur),
        PersonId(acteur.0),
        MembershipId(chemin.into_inner()),
        corps.into_inner(),
    )
    .await?;

    Ok(HttpResponse::Ok().json(adhesion))
}

/// Accepter une invitation par son jeton. **N'exige pas de session** : le jeton
/// est la preuve d'adresse, comme pour la vérification d'adresse de B1.
///
/// Exiger une session rendrait l'invitation inutilisable par la personne qu'elle
/// vise le plus souvent : celle qui n'a pas encore de compte. Si une session
/// existe, elle doit désigner la même personne — c'est le seul cas gênant, celui
/// de quelqu'un de connecté qui suit le lien reçu par un collègue.
#[utoipa::path(
    post,
    description = "`{ status, membership, organization }`. **Aucune session exigée** : le jeton est la preuve d'adresse. Si une session existe, elle doit désigner la même personne (`ORG_INVITATION_NOT_YOURS`). L'adresse est marquée vérifiée : le lien vient de la prouver.",
    path = "/organizations/invitations/accept",
    tag = "Organisations",
    operation_id = "invitation_accepter",
    request_body = Object,
    responses(
        (status = 200, description = "Acceptation, ou l'un des trois refus de jeton", body = Object),
        (status = 403, description = "ORG_INVITATION_NOT_YOURS", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn accepter_invitation(
    state: web::Data<OrgState>,
    corps: web::Json<AcceptInvitation>,
    requete: actix_web::HttpRequest,
) -> Result<HttpResponse> {
    let session = crate::routes::acteur_optionnel(&requete);
    let ctx = crate::routes::contexte_sans_acteur(&requete);

    let issue = membership::accept_invitation(
        &state,
        &ctx,
        session.map(PersonId),
        &corps.into_inner().token,
    )
    .await?;

    Ok(HttpResponse::Ok().json(issue))
}

/// Retirer un membre, ou quitter une organisation.
///
/// **Le seul point d'application de FR-041** : sans cette route, la règle du
/// dernier référent n'aurait aucun endroit où s'exercer.
#[utoipa::path(
    delete,
    description = "`{ status: 'revoked' | 'last_manager' }`. Un référent retire un membre, ou une personne quitte l'organisation. Le retrait du **dernier référent actif** est refusé — contournable par `org.organization.manage`.",
    path = "/memberships/{id}",
    tag = "Organisations",
    operation_id = "adhesion_revoquer",
    params(("id" = Uuid, Path, description = "Adhésion")),
    responses(
        (status = 200, description = "{ status }", body = Object),
        (status = 401, description = "Aucune session", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "ORG_NOT_MANAGER", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Adhésion inexistante", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn revoquer(
    state: web::Data<OrgState>,
    acteur: Actor,
    chemin: web::Path<Uuid>,
    requete: actix_web::HttpRequest,
) -> Result<HttpResponse> {
    let issue = membership::revoke(
        &state,
        &crate::routes::contexte(&requete, acteur),
        PersonId(acteur.0),
        MembershipId(chemin.into_inner()),
    )
    .await?;

    Ok(HttpResponse::Ok().json(issue))
}
