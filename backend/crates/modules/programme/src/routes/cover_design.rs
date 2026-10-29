//! L'atelier de l'image de couverture : une lecture, une écriture, chacune
//! rendant tout ce que l'atelier compose.

use actix_web::{web, HttpRequest, HttpResponse};
use kernel::auth::Actor;
use kernel::error::Result;
use uuid::Uuid;

use crate::domain::ids::ProposalId;
use crate::service::cover_design::{self, SaveCoverDesignPayload};
use crate::state::ProgrammeState;

pub fn chemins_de_dossier(cfg: &mut web::ServiceConfig) {
    cfg.route("/{id}/cover-studio", web::get().to(lire))
        .route("/{id}/cover-design", web::put().to(enregistrer));
}

/// Ce que lit l'atelier de l'image de couverture.
#[utoipa::path(
    get,
    description = "`CoverStudioContext` : les choix enregistrés de l'atelier (`design`, nul tant qu'il n'a jamais été enregistré), la photo de départ du dossier (rôle `cover_source`), le bandeau et la vignette de l'édition (rôles `banner` et `thumbnail`), le logo de l'organisation porteuse (rôle `logo`). Chaque image a la forme `AttachedImage`, nulle sans image servable. Les codes de modèle et de couleur sont ceux du site : la base n'en garde que la forme. Une option absente de la ligne se lit fausse. Accès de la fiche : adhésion active à l'organisation porteuse (ou la déposante en attente), ou lecture générale dans le périmètre.",
    path = "/proposals/{id}/cover-studio",
    tag = "Espace organisation",
    operation_id = "propositions_atelier_couverture",
    params(("id" = Uuid, Path, description = "Identifiant du dossier")),
    responses(
        (status = 200, description = "CoverStudioContext", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Dossier inexistant **ou hors d'accès**", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn lire(
    state: web::Data<ProgrammeState>,
    acteur: Actor,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let contexte = cover_design::lire(&state, acteur.0, ProposalId(chemin.into_inner())).await?;
    Ok(HttpResponse::Ok().json(contexte))
}

/// Enregistrer les choix de l'atelier.
#[utoipa::path(
    put,
    description = "`SaveCoverDesignPayload` → `CoverStudioContext`. Remplace les choix ; la ligne naît au premier enregistrement. Les options sont écrites complètes, une option absente du corps vaut faux. L'image elle-même se dépose et se rattache par le module média (`cover`, et `cover_source` pour la photo de départ) : cette route n'en garde que la recette. Organisation porteuse (adhésion active, ou la déposante en attente pour son propre dossier) ou `programme.proposal.decide` sur l'édition, dans le périmètre ; refusé sur un dossier rejeté, retiré ou annulé ; **permis après la fin de l'édition**.",
    path = "/proposals/{id}/cover-design",
    tag = "Espace organisation",
    operation_id = "propositions_enregistrer_atelier_couverture",
    params(("id" = Uuid, Path, description = "Identifiant du dossier")),
    request_body = Object,
    responses(
        (status = 200, description = "CoverStudioContext", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Dossier inexistant **ou hors d'accès**", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Code de modèle ou de couleur mal formé : VALIDATION_FAILED ; dossier rejeté, retiré ou annulé : PROPOSAL_NOT_EDITABLE", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn enregistrer(
    requete: HttpRequest,
    state: web::Data<ProgrammeState>,
    acteur: Actor,
    chemin: web::Path<Uuid>,
    corps: web::Json<SaveCoverDesignPayload>,
) -> Result<HttpResponse> {
    let ctx = crate::routes::contexte_de(&requete, acteur.0);
    let contexte = cover_design::enregistrer(
        &state,
        &ctx,
        acteur.0,
        ProposalId(chemin.into_inner()),
        corps.into_inner(),
    )
    .await?;
    Ok(HttpResponse::Ok().json(contexte))
}
