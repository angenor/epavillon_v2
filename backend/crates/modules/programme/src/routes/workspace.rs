//! L'espace organisation, et le fil du déposant.
//!
//! # Le préfixe `/organizations` est PARTAGÉ, et composé par l'API
//!
//! Deux `web::scope` du même préfixe **ne se complètent pas** : Actix retient
//! le premier et rend 404 sur les routes du second, sans essayer. Le défaut a
//! coûté trois routes muettes en B2. Ce module dépose donc ses deux routes de
//! ce préfixe **sans le préfixe**, et `api` compose une seule fois — patron de
//! `/people` (R18).
//!
//! # Ici, aucun périmètre d'administration
//!
//! Toutes ces routes sont gardées par l'**adhésion active**. Une organisation
//! n'administre rien : lui appliquer un périmètre n'aurait aucun sens, et un
//! administrateur de la COP31 n'entre pas dans l'espace d'une organisation dont
//! il n'est pas membre — il a la fiche du comité pour cela.

use actix_web::{web, HttpRequest, HttpResponse};
use kernel::auth::Actor;
use kernel::error::Result;
use uuid::Uuid;

use crate::domain::ids::{CommentId, ProposalId};
use crate::routes::contexte_de;
use crate::service::comments::{self, PostCommentPayload, ResolveCommentPayload};
use crate::service::workspace;
use crate::state::ProgrammeState;

/// Ce que ce module dépose sous `/organizations`, **sans le préfixe**.
pub fn sous_organizations(cfg: &mut web::ServiceConfig) {
    cfg.route("/{id}/workspace", web::get().to(espace))
        .route("/{id}/editions", web::get().to(editions));
}

/// Ce que ce module dépose sous `/proposal-comments`.
pub fn sous_proposal_comments(cfg: &mut web::ServiceConfig) {
    cfg.route("/{id}/resolution", web::post().to(resoudre))
        .route("/{id}/resolution", web::delete().to(rouvrir));
}

/// Les chemins de dossier de l'espace organisation.
pub fn chemins_de_dossier(cfg: &mut web::ServiceConfig) {
    cfg.route("/{id}/file", web::get().to(fichier))
        .route("/{id}/comments", web::post().to(ecrire));
}

/// La page d'accueil de l'espace.
#[utoipa::path(
    get,
    description = "`WorkspaceOverview` — l'organisation, l'adhésion de la personne connectée, ses dossiers avec leur journal et leurs demandes de correction ouvertes, ses membres, ce qui attend une action **de sa part**, et l'appel en cours. **Composition propre au soumissionnaire, jamais la vue de pilotage du comité** : ni note, ni note pondérée, ni rang, ni nom de membre du comité, ni inscrit nommé (FR-076, FR-077). **Gardée par l'adhésion active**, jamais par un périmètre d'administration : une organisation n'administre rien. Sans adhésion active, la réponse est `null` en 200, et non 404 : l'indiscernabilité voulue — inexistante et non-membre donnent la même réponse — ne demandait pas un statut d'erreur, et l'écran affichait « une erreur est survenue » là où il faut lire « vous n'avez pas d'espace ici ». Les séances programmées et leurs rappels partent **vides** jusqu'à B5 et B6 — un champ absent ferait échouer l'écran, un champ vide dit qu'il n'y a rien.",
    path = "/organizations/{id}/workspace",
    tag = "Espace organisation",
    operation_id = "propositions_espace_organisation",
    params(("id" = Uuid, Path, description = "Identifiant de l'organisation")),
    responses(
        (status = 200, description = "WorkspaceOverview | null", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn espace(
    state: web::Data<ProgrammeState>,
    acteur: Actor,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let vue = workspace::espace(&state, acteur.0, chemin.into_inner()).await?;
    Ok(HttpResponse::Ok().json(vue))
}

/// Les éditions sur lesquelles cette organisation a déposé.
#[utoipa::path(
    get,
    description = "`EventEdition[] | null`, de la plus récente à la plus ancienne. Une organisation fidèle en a plusieurs, et sa liste de dossiers les groupe : un dossier de la COP30 ne se lit pas comme un dossier en cours. **Adhésion active exigée** — à défaut, `null` en 200, jamais une liste vide : « aucun dossier » et « ce n'est pas votre espace » ne se confondent pas.",
    path = "/organizations/{id}/editions",
    tag = "Espace organisation",
    operation_id = "propositions_editions_de_lorganisation",
    params(("id" = Uuid, Path, description = "Identifiant de l'organisation")),
    responses(
        (status = 200, description = "EventEdition[] | null", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn editions(
    state: web::Data<ProgrammeState>,
    acteur: Actor,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let fiches = workspace::editions(&state, acteur.0, chemin.into_inner()).await?;
    Ok(HttpResponse::Ok().json(fiches))
}

/// Le dossier vu par son déposant.
#[utoipa::path(
    get,
    description = "`ProposalFile` — le suivi du dossier, le **fil partagé** et l'historique champ par champ. Le fil est filtré **à la source** sur la visibilité partagée : les délibérations du comité n'y sont jamais, et les notes personnelles encore moins. C'est le **même** filtre que celui du comité — l'écrire deux fois serait écrire deux filtres, et le second finirait par diverger. `null` en 200 pour un dossier inexistant **ou porté par une organisation dont on n'est pas membre** : indiscernables, et ce n'est pas une panne.",
    path = "/proposals/{id}/file",
    tag = "Espace organisation",
    operation_id = "propositions_dossier_du_deposant",
    params(("id" = Uuid, Path, description = "Identifiant du dossier")),
    responses(
        (status = 200, description = "ProposalFile | null", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn fichier(
    state: web::Data<ProgrammeState>,
    acteur: Actor,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let fichier = workspace::dossier(&state, acteur.0, ProposalId(chemin.into_inner())).await?;
    Ok(HttpResponse::Ok().json(fichier))
}

/// Écrire un message sur un dossier — **des deux côtés**.
#[utoipa::path(
    post,
    description = "`PostCommentPayload` ou `ReplyToCommentPayload` → `ProposalComment`. **Une seule route, deux appelants** : une réponse du déposant est **toujours** partagée et **jamais** une demande de correction — une organisation ne se demande pas des corrections à elle-même ; un message du comité porte sa visibilité, et une demande de correction y est **forcée en partagé** (écart n° 99), sans quoi elle bloquerait le dossier sans que le déposant sache pourquoi. **Seul un message partagé émet** `programme.comment.shared` : un message de comité ne sort pas du comité, par définition.",
    path = "/proposals/{id}/comments",
    tag = "Espace organisation",
    operation_id = "propositions_ecrire_un_message",
    params(("id" = Uuid, Path, description = "Identifiant du dossier")),
    request_body = Object,
    responses(
        (status = 200, description = "ProposalComment", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Dossier hors d'accès **ou inexistant**", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Corps vide", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn ecrire(
    requete: HttpRequest,
    state: web::Data<ProgrammeState>,
    acteur: Actor,
    chemin: web::Path<Uuid>,
    corps: web::Json<PostCommentPayload>,
) -> Result<HttpResponse> {
    let ctx = contexte_de(&requete, acteur.0);
    let message = comments::ecrire(
        &state,
        &ctx,
        acteur.0,
        ProposalId(chemin.into_inner()),
        corps.into_inner(),
    )
    .await?;

    Ok(HttpResponse::Ok().json(message))
}

/// Marquer une demande de correction résolue.
#[utoipa::path(
    post,
    description = "`ResolveCommentPayload` → `ProposalComment`. **Qui peut le faire n'est écrit nulle part dans le modèle** — `resolved_by` est une simple clé étrangère —, et l'écart n° 35 a été tranché en A5 : le **déposant pose**, c'est lui qui sait qu'il a corrigé. Le verbe porte le sens ; le champ `resolved` du contrat est redondant et n'est pas cru. Rien n'est émis : l'état visible est le compteur de demandes ouvertes, relu à chaque affichage.",
    path = "/proposal-comments/{id}/resolution",
    tag = "Espace organisation",
    operation_id = "propositions_resoudre_une_demande",
    params(("id" = Uuid, Path, description = "Identifiant du message")),
    request_body = Object,
    responses(
        (status = 200, description = "ProposalComment", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Message inexistant **ou hors d'accès**", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Le message n'est pas une demande de correction", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn resoudre(
    requete: HttpRequest,
    state: web::Data<ProgrammeState>,
    acteur: Actor,
    chemin: web::Path<Uuid>,
    _corps: web::Json<ResolveCommentPayload>,
) -> Result<HttpResponse> {
    let ctx = contexte_de(&requete, acteur.0);
    let message =
        comments::resoudre(&state, &ctx, acteur.0, CommentId(chemin.into_inner()), true).await?;

    Ok(HttpResponse::Ok().json(message))
}

/// Rouvrir une demande de correction — **le comité seul**.
#[utoipa::path(
    delete,
    description = "`ResolveCommentPayload` → `ProposalComment`. **Le comité garde la main pour retirer** : un déposant qui pourrait retirer sa propre résolution ne changerait rien d'utile, mais un déposant qui retirerait celle du comité effacerait un arbitrage. C'est une règle d'autorisation, elle appartient à la permission et non à un formulaire (écart n° 35).",
    path = "/proposal-comments/{id}/resolution",
    tag = "Espace organisation",
    operation_id = "propositions_rouvrir_une_demande",
    params(("id" = Uuid, Path, description = "Identifiant du message")),
    request_body = Object,
    responses(
        (status = 200, description = "ProposalComment", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Le déposant ne rouvre pas une demande de correction", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Message inexistant **ou hors d'accès**", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn rouvrir(
    requete: HttpRequest,
    state: web::Data<ProgrammeState>,
    acteur: Actor,
    chemin: web::Path<Uuid>,
    _corps: web::Json<ResolveCommentPayload>,
) -> Result<HttpResponse> {
    let ctx = contexte_de(&requete, acteur.0);
    let message = comments::resoudre(
        &state,
        &ctx,
        acteur.0,
        CommentId(chemin.into_inner()),
        false,
    )
    .await?;

    Ok(HttpResponse::Ok().json(message))
}
