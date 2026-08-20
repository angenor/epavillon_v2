//! Back-office RGPD — la file des demandes et leur traitement.
//!
//! **Une seule garde, et elle est globale.** `Requires<PersonManage>` teste
//! `identity.person.manage` sur la portée globale : un administrateur détaché
//! sur une édition reçoit **403**, jamais une file filtrée. C'est la différence
//! avec les écrans d'utilisateurs, qui prennent un périmètre en argument — ici,
//! il n'y a pas de périmètre à prendre, et prétendre le contraire donnerait
//! l'illusion d'un traitement complet.

use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use kernel::auth::Requires;
use kernel::context::RequestContext;
use kernel::error::Result;
use serde::Deserialize;
use uuid::Uuid;

use crate::domain::permissions::PersonManage;
use crate::domain::privacy::PrivacyAction;
use crate::service::privacy;
use crate::state::IdentityState;

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/admin/privacy-requests")
            .route("", web::get().to(file))
            .route("/{id}", web::put().to(traiter)),
    );
}

/// **Aucun champ de droits.** Le site passe encore ses permissions et son
/// identifiant d'acteur à `handlePrivacyRequest` ; ils ne franchissent pas le
/// réseau, et s'ils le franchissaient, rien ici ne les lirait (FR-055).
///
/// `request_id` non plus : la demande visée vient de **l'URL**. Le site l'envoie
/// encore dans son corps — le lire laisserait traiter une autre demande que
/// celle qu'on regarde.
#[derive(Debug, Deserialize)]
pub struct HandlePrivacyRequestPayload {
    pub action: PrivacyAction,
    #[serde(default)]
    pub resolution: Option<String>,
}

#[utoipa::path(
    get,
    description = "`PrivacyQueueScreen`. **Portée globale exigée** : jamais une file filtrée.",
    path = "/admin/privacy-requests",
    tag = "Back-office — RGPD",
    operation_id = "file",
    responses(
        (status = 200, description = "PrivacyQueueScreen", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Portée globale exigée — un administrateur d'édition est refusé", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn file(
    state: web::Data<IdentityState>,
    _permission: Requires<PersonManage>,
) -> Result<HttpResponse> {
    let ecran = privacy::queue_screen(state.pool()).await?;
    Ok(HttpResponse::Ok().json(ecran))
}

/// Les quatre issues sortent en **200** : ce sont des refus prévus par le
/// contrat du site. Le refus d'autorisation, lui, est un 403 rendu par
/// l'extracteur, avant que ce gestionnaire existe.
#[utoipa::path(
    put,
    description = "`HandlePrivacyRequestPayload` → `PrivacyWriteResult`. **Les quatre issues sortent en 200.**",
    path = "/admin/privacy-requests/{id}",
    tag = "Back-office — RGPD",
    operation_id = "traiter",
    params(("id" = Uuid, Path, description = "Identifiant de la demande")),
    request_body = Object,
    responses(
        (status = 200, description = "PrivacyWriteResult — saved, anonymized, wrong_type, not_found", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission ou portée insuffisante", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Requête invalide", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn traiter(
    state: web::Data<IdentityState>,
    permission: Requires<PersonManage>,
    chemin: web::Path<Uuid>,
    corps: web::Json<HandlePrivacyRequestPayload>,
    requete: HttpRequest,
) -> Result<HttpResponse> {
    let ctx = requete
        .extensions()
        .get::<RequestContext>()
        .cloned()
        .unwrap_or_else(|| RequestContext::new(RequestContext::generated_request_id(), "fr"));

    let issue = privacy::handle(
        &state,
        &ctx,
        permission.person_id,
        chemin.into_inner(),
        corps.action,
        corps
            .resolution
            .as_deref()
            .map(str::trim)
            .filter(|r| !r.is_empty()),
    )
    .await?;

    Ok(HttpResponse::Ok().json(issue))
}
