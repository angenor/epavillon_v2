//! Les opérations d'administration du module — **une seule, et elle est
//! ponctuelle**.
//!
//! `/admin/proposals` est un préfixe **propre à ce module** : personne d'autre
//! n'y dépose, il n'y a donc rien à composer dans l'API.

use actix_web::{web, HttpRequest, HttpResponse};
use kernel::auth::Requires;
use kernel::error::Result;

use crate::domain::permissions::ProposalReadAll;
use crate::routes::contexte_de;
use crate::service::backfill;
use crate::state::ProgrammeState;

/// Le scope `/admin/proposals`.
pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/transitions-backfill", web::post().to(deduire));
}

/// Semer les transitions déductibles des dossiers repris de la v1.
#[utoipa::path(
    post,
    description = "Nombre de dossiers traités et de lignes semées. **Portée GLOBALE exigée** : une reprise porte sur tout le corpus, et la borner à une édition n'aurait aucun sens. **Synchrone et rejouable** : la condition « journal vide » est dans la requête d'insertion, si bien qu'une seconde exécution rend zéro. **Elle n'émet AUCUN événement** — elle écrit dans le journal sans passer par la mise à jour de l'état, donc sans réveiller le déclencheur : émettre huit mille événements de dossiers décidés il y a deux ans déclencherait autant de courriels, le pire effet possible d'une reprise. Elle ne devine ni le passage par l'évaluation, ni une demande de correction : ce qui n'est pas dans les dates du dossier n'est pas déductible, et l'inventer serait pire qu'un trou.",
    path = "/admin/proposals/transitions-backfill",
    tag = "Back-office — propositions",
    operation_id = "propositions_deduire_les_transitions_v1",
    responses(
        (status = 200, description = "Dossiers traités et lignes semées", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Portée globale exigée", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn deduire(
    requete: HttpRequest,
    state: web::Data<ProgrammeState>,
    // `Requires` exige la permission **sur la portée globale** : un
    // administrateur détaché sur une édition ne lance pas une reprise.
    acteur: Requires<ProposalReadAll>,
) -> Result<HttpResponse> {
    let ctx = contexte_de(&requete, acteur.person_id);
    let resultat = backfill::deduire(&state, &ctx).await?;

    Ok(HttpResponse::Ok().json(resultat))
}
