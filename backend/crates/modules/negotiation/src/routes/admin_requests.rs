//! La file des demandes d'accès, et les deux décisions.
//!
//! Même garde que le reste du back-office de Guide Négo :
//! **`Requires<SpaceManage>`, sur la portée globale**. Une demande d'accès à un
//! espace de négociation n'appartient à aucune édition, et un administrateur
//! d'événement n'a rien à y trancher (FR-044, SC-008).
//!
//! **Chaque décision est une transaction** : l'état, l'accès, le réseau et le
//! courriel, ou rien. Le courriel ne peut donc pas annoncer un accès qui
//! n'aurait pas été accordé (FR-028).

use actix_web::{web, HttpRequest, HttpResponse};
use kernel::auth::Requires;
use kernel::error::Result;
use serde::Deserialize;
use uuid::Uuid;

use crate::domain::permissions::SpaceManage;
use crate::domain::requests::DecideAccessRequestPayload;
use crate::repo::codes::{LISTE_LIMITE_DEFAUT, LISTE_LIMITE_MAX};
use crate::repo::requests::FiltreDemandes;
use crate::service::admin_requests;
use crate::state::NegotiationState;

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/admin/negotiation/access-requests", web::get().to(file))
        .route(
            "/admin/negotiation/access-requests/{id}/approve",
            web::post().to(admettre),
        )
        .route(
            "/admin/negotiation/access-requests/{id}/reject",
            web::post().to(refuser),
        );
}

/// Le filtre, **nommé en français** : il apparaît dans une adresse qu'on
/// partage.
#[derive(Debug, Deserialize)]
pub struct FileQuery {
    /// `pending`, `approved`, `rejected` ou `cancelled`. Absent : toutes.
    pub etat: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[utoipa::path(
    get,
    description = "`AccessRequestQueue` — la file des demandes d'accès : nom, pays, heure d'envoi, code éventuel et message.\n\nLes demandes **en attente viennent en tête**, et dans l'ordre où elles se sont formées : une file se traite par le début.\n\n`pending` compte les demandes en attente **tous filtres confondus** — c'est la pastille du menu, et elle ne doit pas tomber à zéro parce qu'on regarde les refusées.\n\nLe code affiché est celui que la demande portait en mode « code et approbation » : il a été reconnu, il n'a pas suffi à ouvrir, et c'est ce que l'administrateur lit d'abord.",
    path = "/admin/negotiation/access-requests",
    tag = "Back-office — admission",
    operation_id = "admin_negotiation_demandes_file",
    params(
        ("etat" = Option<String>, Query, description = "pending, approved, rejected ou cancelled"),
        ("limit" = Option<i64>, Query, description = "Défaut 25, maximum 100"),
        ("offset" = Option<i64>, Query, description = "Décalage"),
    ),
    responses(
        (status = 200, description = "AccessRequestQueue", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Sans `negotiation.space.manage` **sur la portée globale**", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn file(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    _garde: Requires<SpaceManage>,
    query: web::Query<FileQuery>,
) -> Result<HttpResponse> {
    let filtre = FiltreDemandes {
        etat: query.etat.as_deref().filter(|e| !e.is_empty()),
        limit: query
            .limit
            .unwrap_or(LISTE_LIMITE_DEFAUT)
            .clamp(1, LISTE_LIMITE_MAX),
        offset: query.offset.unwrap_or(0).max(0),
    };

    let file = admin_requests::file(&state, &filtre, &crate::routes::locale_de(&requete)).await?;
    Ok(HttpResponse::Ok().json(file))
}

#[utoipa::path(
    post,
    description = "`DecideAccessRequestPayload` — admet une demande.\n\n**Une seule transaction** : l'état de la demande, l'attribution du rôle `negotiator` avec la portée demandée, l'appartenance au réseau si la demande portait un code qui en ouvrait un, l'inscription à l'annuaire de l'espace, et le courriel mis en file. Rien ne part si elle échoue (FR-027, FR-028).\n\nL'événement `negotiation.access_request.approved` est émis **par la base**, dans la même transaction.\n\nUne demande déjà tranchée sort en `NEGOTIATION_ACCESS_REQUEST_DECIDED` : la transition est refusée par le trigger du modèle, l'API la traduit.",
    path = "/admin/negotiation/access-requests/{id}/approve",
    tag = "Back-office — admission",
    operation_id = "admin_negotiation_demandes_admettre",
    params(("id" = Uuid, Path, description = "Demande d'accès")),
    request_body = Object,
    responses(
        (status = 204, description = "Demande admise"),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Sans la permission sur la portée globale", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Demande inexistante", body = crate::routes::openapi::ApiErrorBody),
        (status = 409, description = "Demande déjà tranchée", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn admettre(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    garde: Requires<SpaceManage>,
    chemin: web::Path<Uuid>,
    charge: Option<web::Json<DecideAccessRequestPayload>>,
) -> Result<HttpResponse> {
    let contexte = crate::routes::contexte_de(&requete, garde.person_id);
    let motif = motif_de(&charge);

    admin_requests::admettre(
        &state,
        &contexte,
        garde.person_id,
        chemin.into_inner(),
        motif.as_deref(),
        &contexte.locale,
    )
    .await?;

    Ok(HttpResponse::NoContent().finish())
}

#[utoipa::path(
    post,
    description = "`DecideAccessRequestPayload` — refuse une demande, avec un motif facultatif.\n\nLe motif est **repris tel quel** dans le courriel : c'est la seule chose que la personne lira pour comprendre. Sans motif, le message ne fait pas semblant d'en avoir un — il dit ce qui reste possible, demander le code en cours à son réseau.\n\nUn refus n'est pas définitif : une nouvelle demande est **une nouvelle ligne**, ce qui conserve l'historique des décisions.",
    path = "/admin/negotiation/access-requests/{id}/reject",
    tag = "Back-office — admission",
    operation_id = "admin_negotiation_demandes_refuser",
    params(("id" = Uuid, Path, description = "Demande d'accès")),
    request_body = Object,
    responses(
        (status = 204, description = "Demande refusée"),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Sans la permission sur la portée globale", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Demande inexistante", body = crate::routes::openapi::ApiErrorBody),
        (status = 409, description = "Demande déjà tranchée", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn refuser(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    garde: Requires<SpaceManage>,
    chemin: web::Path<Uuid>,
    charge: Option<web::Json<DecideAccessRequestPayload>>,
) -> Result<HttpResponse> {
    let contexte = crate::routes::contexte_de(&requete, garde.person_id);
    let motif = motif_de(&charge);

    admin_requests::refuser(
        &state,
        &contexte,
        garde.person_id,
        chemin.into_inner(),
        motif.as_deref(),
        &contexte.locale,
    )
    .await?;

    Ok(HttpResponse::NoContent().finish())
}

/// **Un corps absent est une décision sans motif**, pas une requête malformée :
/// l'écran n'oblige personne à écrire pour admettre.
fn motif_de(charge: &Option<web::Json<DecideAccessRequestPayload>>) -> Option<String> {
    charge
        .as_ref()
        .and_then(|c| c.reason.as_deref())
        .map(str::trim)
        .filter(|m| !m.is_empty())
        .map(str::to_owned)
}
