//! Les visites de l'espace d'une organisation : la page les déclare, le
//! back-office les relit.

use actix_web::{web, HttpResponse};
use kernel::auth::{Actor, Perimeter};
use kernel::error::Result;
use serde::Deserialize;

use crate::domain::ids::OrganizationId;
use crate::domain::permissions::ORGANIZATION_READ;
use crate::domain::visits::SpaceVisit;
use crate::service::visits;
use crate::state::OrgState;

/// Sous `/organizations`, avant `/{id}` : sans quoi « space-visits » serait lu
/// comme un identifiant.
pub fn organisations(cfg: &mut web::ServiceConfig) {
    cfg.route("/space-visits", web::post().to(noter));
}

/// Sous `/admin/organizations`.
pub fn admin(cfg: &mut web::ServiceConfig) {
    cfg.route("/{id}/visits", web::get().to(lire));
}

/// Une page de l'espace s'est ouverte.
#[utoipa::path(
    post,
    description = "Note l'ouverture de « Mon organisation » (`organization_id`) ou d'un dossier (`proposal_id`), une ligne par personne, page et jour. Rien n'est noté pour qui n'est pas membre actif, et la réponse ne le dit pas.",
    path = "/organizations/space-visits",
    tag = "Organisations",
    operation_id = "organisation_visite_de_l_espace",
    request_body = Object,
    responses(
        (status = 204, description = "Notée, ou ignorée"),
        (status = 401, description = "Aucune session", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Ni organisation ni dossier, ou les deux", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn noter(
    state: web::Data<OrgState>,
    acteur: Actor,
    corps: web::Json<SpaceVisit>,
    requete: actix_web::HttpRequest,
) -> Result<HttpResponse> {
    visits::noter(
        &state,
        &crate::routes::contexte(&requete, acteur),
        acteur.0,
        corps.into_inner(),
    )
    .await?;
    Ok(HttpResponse::NoContent().finish())
}

#[derive(Debug, Deserialize)]
pub struct FenetreQuery {
    /// 7, 30… ; 0 ou absent au-delà de 366 : depuis le début.
    pub days: Option<u16>,
}

/// L'onglet « Visites » de la fiche.
#[utoipa::path(
    get,
    description = "`OrganizationVisits | null` — membres, connexions, pages ouvertes de l'espace, modifications et refus des membres, sur la fenêtre demandée. Hors périmètre : `null`, comme la fiche.",
    path = "/admin/organizations/{id}/visits",
    tag = "Back-office — organisations",
    operation_id = "admin_organisation_visites",
    params(
        ("id" = uuid::Uuid, Path, description = "Organisation"),
        ("days" = Option<u16>, Query, description = "Fenêtre en jours ; 0 : depuis le début. Défaut 30"),
    ),
    responses(
        (status = 200, description = "OrganizationVisits | null", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente, ou périmètre d'administration vide", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn lire(
    state: web::Data<OrgState>,
    perimetre: Perimeter,
    chemin: web::Path<uuid::Uuid>,
    query: web::Query<FenetreQuery>,
) -> Result<HttpResponse> {
    kernel::auth::require_permission_anywhere(state.pool(), perimetre.person_id, ORGANIZATION_READ)
        .await?;

    let jours = match query.days {
        None => 30,
        Some(j) if j > 366 => 0,
        Some(j) => j,
    };
    let onglet = visits::lire(
        state.pool(),
        &perimetre.scope,
        OrganizationId(chemin.into_inner()),
        jours,
    )
    .await?;

    Ok(HttpResponse::Ok().json(onglet))
}
