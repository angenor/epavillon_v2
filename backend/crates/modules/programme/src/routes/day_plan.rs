//! Le jour J d'une activité retenue : une lecture, quatre écritures, chacune
//! rendant le jour J entier pour que la fiche se redessine d'une réponse.

use actix_web::{web, HttpRequest, HttpResponse};
use kernel::auth::Actor;
use kernel::error::Result;
use uuid::Uuid;

use crate::domain::ids::ProposalId;
use crate::service::day_plan::{
    self, AddSocialPostPayload, SaveFocalPointsPayload, SaveTechnicalNeedsPayload,
};
use crate::state::ProgrammeState;

pub fn chemins_de_dossier(cfg: &mut web::ServiceConfig) {
    cfg.route("/{id}/day-plan", web::get().to(lire))
        .route("/{id}/focal-points", web::put().to(points_focaux))
        .route("/{id}/technical-needs", web::put().to(besoins))
        .route("/{id}/social-posts", web::post().to(ajouter_publication))
        .route(
            "/{id}/social-posts/{post_id}",
            web::delete().to(retirer_publication),
        );
}

/// Le jour J d'un dossier.
#[utoipa::path(
    get,
    description = "`ProposalDayPlan` : points focaux, besoins techniques, publications et date limite indicative des besoins (dix jours avant la première séance non annulée, nulle sans séance). **Tant qu'aucun point focal n'est enregistré, le déposant est proposé** (`id: null`, `is_default: true`, téléphone de sa fiche). `technical_needs.updated_at` nul : rien de déclaré, pas même « aucun besoin ». La plateforme d'une publication se déduit de son adresse. Accès de la fiche : adhésion active à l'organisation porteuse (ou la déposante en attente), ou lecture générale dans le périmètre.",
    path = "/proposals/{id}/day-plan",
    tag = "Espace organisation",
    operation_id = "propositions_jour_j",
    params(("id" = Uuid, Path, description = "Identifiant du dossier")),
    responses(
        (status = 200, description = "ProposalDayPlan", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Dossier inexistant **ou hors d'accès**", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn lire(
    state: web::Data<ProgrammeState>,
    acteur: Actor,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let plan = day_plan::lire(&state, acteur.0, ProposalId(chemin.into_inner())).await?;
    Ok(HttpResponse::Ok().json(plan))
}

/// Remplacer les points focaux.
#[utoipa::path(
    put,
    description = "`SaveFocalPointsPayload` → `ProposalDayPlan`. **Remplace la liste entière**, dans l'ordre reçu ; une liste vide est refusée (au moins une personne à joindre). Chaque entrée désigne un intervenant du dossier (`speaker_id`), une personne connue du dossier (`person_id` : déposant, contact ou membre actif), ou personne (collègue saisi à la main, nom obligatoire). Nom ou téléphone laissés vides reprennent ceux de la fiche. Une même personne deux fois est refusée. Organisation porteuse ou `programme.proposal.decide` sur l'édition ; dossier retenu ; refusé après la fin de l'édition.",
    path = "/proposals/{id}/focal-points",
    tag = "Espace organisation",
    operation_id = "propositions_points_focaux",
    params(("id" = Uuid, Path, description = "Identifiant du dossier")),
    request_body = Object,
    responses(
        (status = 200, description = "ProposalDayPlan", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Dossier inexistant **ou hors d'accès**", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Liste vide, nom manquant, doublon : VALIDATION_FAILED ; intervenant ou personne étrangers au dossier : PROPOSAL_UNKNOWN_REFERENCE ; dossier non retenu ou édition terminée : PROPOSAL_NOT_EDITABLE", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn points_focaux(
    requete: HttpRequest,
    state: web::Data<ProgrammeState>,
    acteur: Actor,
    chemin: web::Path<Uuid>,
    corps: web::Json<SaveFocalPointsPayload>,
) -> Result<HttpResponse> {
    let ctx = crate::routes::contexte_de(&requete, acteur.0);
    let plan = day_plan::enregistrer_points_focaux(
        &state,
        &ctx,
        acteur.0,
        ProposalId(chemin.into_inner()),
        corps.into_inner(),
    )
    .await?;
    Ok(HttpResponse::Ok().json(plan))
}

/// Déclarer les besoins techniques.
#[utoipa::path(
    put,
    description = "`SaveTechnicalNeedsPayload` → `ProposalDayPlan`. La ligne naît au premier enregistrement, même « aucun besoin ». Le détail d'une case décochée est effacé. Organisation porteuse ou `programme.proposal.decide` sur l'édition ; dossier retenu ; refusé après la fin de l'édition. La date limite est indicative : rien n'est refusé après.",
    path = "/proposals/{id}/technical-needs",
    tag = "Espace organisation",
    operation_id = "propositions_besoins_techniques",
    params(("id" = Uuid, Path, description = "Identifiant du dossier")),
    request_body = Object,
    responses(
        (status = 200, description = "ProposalDayPlan", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Dossier inexistant **ou hors d'accès**", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Texte trop long : VALIDATION_FAILED ; dossier non retenu ou édition terminée : PROPOSAL_NOT_EDITABLE", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn besoins(
    requete: HttpRequest,
    state: web::Data<ProgrammeState>,
    acteur: Actor,
    chemin: web::Path<Uuid>,
    corps: web::Json<SaveTechnicalNeedsPayload>,
) -> Result<HttpResponse> {
    let ctx = crate::routes::contexte_de(&requete, acteur.0);
    let plan = day_plan::enregistrer_besoins(
        &state,
        &ctx,
        acteur.0,
        ProposalId(chemin.into_inner()),
        corps.into_inner(),
    )
    .await?;
    Ok(HttpResponse::Ok().json(plan))
}

/// Ajouter une publication.
#[utoipa::path(
    post,
    description = "`AddSocialPostPayload` → `ProposalDayPlan`. Seule une adresse http(s) complète est acceptée ; la plateforme se déduit de l'adresse à la lecture. Un lien déjà présent rend `CONFLICT`. Organisation porteuse ou `programme.proposal.decide` sur l'édition ; dossier retenu ; **permis après la fin de l'édition** (le bilan se fait après).",
    path = "/proposals/{id}/social-posts",
    tag = "Espace organisation",
    operation_id = "propositions_ajouter_une_publication",
    params(("id" = Uuid, Path, description = "Identifiant du dossier")),
    request_body = Object,
    responses(
        (status = 200, description = "ProposalDayPlan", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Dossier inexistant **ou hors d'accès**", body = crate::routes::openapi::ApiErrorBody),
        (status = 409, description = "Lien déjà présent : CONFLICT", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Adresse invalide : VALIDATION_FAILED ; dossier non retenu : PROPOSAL_NOT_EDITABLE", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn ajouter_publication(
    requete: HttpRequest,
    state: web::Data<ProgrammeState>,
    acteur: Actor,
    chemin: web::Path<Uuid>,
    corps: web::Json<AddSocialPostPayload>,
) -> Result<HttpResponse> {
    let ctx = crate::routes::contexte_de(&requete, acteur.0);
    let plan = day_plan::ajouter_publication(
        &state,
        &ctx,
        acteur.0,
        ProposalId(chemin.into_inner()),
        corps.into_inner(),
    )
    .await?;
    Ok(HttpResponse::Ok().json(plan))
}

/// Retirer une publication.
#[utoipa::path(
    delete,
    description = "Retrait d'une publication → `ProposalDayPlan`. Mêmes droits que l'ajout, permis après la fin de l'édition.",
    path = "/proposals/{id}/social-posts/{post_id}",
    tag = "Espace organisation",
    operation_id = "propositions_retirer_une_publication",
    params(
        ("id" = Uuid, Path, description = "Identifiant du dossier"),
        ("post_id" = Uuid, Path, description = "Identifiant de la publication"),
    ),
    responses(
        (status = 200, description = "ProposalDayPlan", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Publication inexistante, d'un autre dossier, **ou hors d'accès**", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Dossier non retenu : PROPOSAL_NOT_EDITABLE", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn retirer_publication(
    requete: HttpRequest,
    state: web::Data<ProgrammeState>,
    acteur: Actor,
    chemin: web::Path<(Uuid, Uuid)>,
) -> Result<HttpResponse> {
    let (dossier, publication) = chemin.into_inner();
    let ctx = crate::routes::contexte_de(&requete, acteur.0);
    let plan =
        day_plan::retirer_publication(&state, &ctx, acteur.0, ProposalId(dossier), publication)
            .await?;
    Ok(HttpResponse::Ok().json(plan))
}
