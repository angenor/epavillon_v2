//! L'import de la source officielle et l'ordre du jour, au back-office.
//!
//! Même garde que le reste du back-office de Guide Négo : `Requires<SpaceManage>`
//! **sur la portée globale**, jamais `RequiresAnyScope` (tranché le 21/09). Le
//! rôle `admin` porte la permission sur un événement : une garde plus large
//! ouvrirait l'import de sa COP à l'administrateur d'une seule édition.
//!
//! Aucune route ne modifie une session : la source fait foi (FR-041).

use actix_web::{web, HttpRequest, HttpResponse};
use kernel::auth::Requires;
use kernel::error::Result;
use uuid::Uuid;

use crate::domain::admin_import::{
    AgendaItemThemePayload, EditionQuery, UpdateOfficialImportPayload,
};
use crate::domain::permissions::SpaceManage;
use crate::service::admin_import as service;
use crate::state::NegotiationState;

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/admin/negotiation/import", web::get().to(etat))
        .route("/admin/negotiation/import", web::put().to(regler))
        .route(
            "/admin/negotiation/import/read",
            web::post().to(lire_maintenant),
        )
        .route("/admin/negotiation/agenda-items", web::get().to(points))
        .route(
            "/admin/negotiation/agenda-items/{id}",
            web::put().to(rattacher),
        );
}

#[utoipa::path(
    get,
    description = "`OfficialImportAdmin` — le réglage de l'import de l'édition, sa santé et le journal des vingt dernières lectures.\n\n`serving` est lu de `negotiation.import_is_serving()`, la règle de coupure écrite une seule fois. `archives` énumère les jeux archivés embarqués dans le binaire. Une édition sans import se lit avec les valeurs par défaut, éteinte.",
    path = "/admin/negotiation/import",
    tag = "Back-office — sessions officielles",
    operation_id = "admin_negotiation_import_lire",
    params(("edition" = String, Query, description = "Slug de l'édition")),
    responses(
        (status = 200, description = "OfficialImportAdmin", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Sans `negotiation.space.manage` **sur la portée globale**", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Édition inconnue", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn etat(
    state: web::Data<NegotiationState>,
    _garde: Requires<SpaceManage>,
    edition: web::Query<EditionQuery>,
) -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(service::lire(&state, &edition.edition).await?))
}

#[utoipa::path(
    put,
    description = "`UpdateOfficialImportPayload` → `OfficialImportAdmin` — pose le réglage entier, et crée la ligne d'import si elle manque.\n\n**Allumer** pose la première lecture dans la même transaction ; **éteindre** coupe l'affichage aussitôt. Le seuil et l'intervalle valent dès la lecture suivante.\n\nUn réglage incomplet — lecteur `live` sans adresse, jeu archivé inconnu, intervalle hors de 60 s à un jour, seuil hors de 1 à 100 — sort en `NEGOTIATION_IMPORT_CONFIG_INVALID`, qui nomme le champ.",
    path = "/admin/negotiation/import",
    tag = "Back-office — sessions officielles",
    operation_id = "admin_negotiation_import_regler",
    params(("edition" = String, Query, description = "Slug de l'édition")),
    request_body = Object,
    responses(
        (status = 200, description = "OfficialImportAdmin", body = Object),
        (status = 400, description = "Réglage incomplet", body = crate::routes::openapi::ApiErrorBody),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Sans la permission sur la portée globale", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Édition inconnue", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn regler(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    garde: Requires<SpaceManage>,
    edition: web::Query<EditionQuery>,
    charge: web::Json<UpdateOfficialImportPayload>,
) -> Result<HttpResponse> {
    let contexte = crate::routes::contexte_de(&requete, garde.person_id);
    let reponse = service::regler(
        &state,
        &contexte,
        garde.person_id,
        &edition.edition,
        charge.into_inner(),
    )
    .await?;
    Ok(HttpResponse::Ok().json(reponse))
}

#[utoipa::path(
    post,
    description = "« Lire maintenant » — pose une lecture immédiate à sa propre clé, **sans replanifier** : deux appels font deux lectures, et la chaîne récurrente reste unique.\n\nImport éteint, la lecture a lieu et l'affichage reste coupé.",
    path = "/admin/negotiation/import/read",
    tag = "Back-office — sessions officielles",
    operation_id = "admin_negotiation_import_lire_maintenant",
    params(("edition" = String, Query, description = "Slug de l'édition")),
    responses(
        (status = 202, description = "Lecture posée"),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Sans la permission sur la portée globale", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Édition inconnue, ou sans import", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn lire_maintenant(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    garde: Requires<SpaceManage>,
    edition: web::Query<EditionQuery>,
) -> Result<HttpResponse> {
    let contexte = crate::routes::contexte_de(&requete, garde.person_id);
    service::lire_maintenant(&state, &contexte, &edition.edition).await?;
    Ok(HttpResponse::Accepted().finish())
}

#[utoipa::path(
    get,
    description = "`AgendaItemAdmin[]` — les points de l'ordre du jour de l'édition, ceux sans thématique d'abord, puis par code. `theme` est un code du vocabulaire `negotiation_theme`.",
    path = "/admin/negotiation/agenda-items",
    tag = "Back-office — sessions officielles",
    operation_id = "admin_negotiation_agenda_items",
    params(("edition" = String, Query, description = "Slug de l'édition")),
    responses(
        (status = 200, description = "AgendaItemAdmin[]", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Sans la permission sur la portée globale", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Édition inconnue", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn points(
    state: web::Data<NegotiationState>,
    _garde: Requires<SpaceManage>,
    edition: web::Query<EditionQuery>,
) -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(service::points(&state, &edition.edition).await?))
}

#[utoipa::path(
    put,
    description = "`AgendaItemThemePayload` → `AgendaItemAdmin` — rattache un point à une thématique, ou l'en détache (`null`). Les sessions du point en héritent à la lecture : rien n'est recopié sur elles.",
    path = "/admin/negotiation/agenda-items/{id}",
    tag = "Back-office — sessions officielles",
    operation_id = "admin_negotiation_agenda_item_rattacher",
    params(("id" = Uuid, Path, description = "Identifiant du point")),
    request_body = Object,
    responses(
        (status = 200, description = "AgendaItemAdmin", body = Object),
        (status = 400, description = "Thématique inconnue", body = crate::routes::openapi::ApiErrorBody),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Sans la permission sur la portée globale", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Point inconnu", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn rattacher(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    garde: Requires<SpaceManage>,
    id: web::Path<Uuid>,
    charge: web::Json<AgendaItemThemePayload>,
) -> Result<HttpResponse> {
    let contexte = crate::routes::contexte_de(&requete, garde.person_id);
    let point = service::rattacher(
        &state,
        &contexte,
        garde.person_id,
        id.into_inner(),
        charge.theme.as_deref(),
    )
    .await?;
    Ok(HttpResponse::Ok().json(point))
}
