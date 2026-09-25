//! Les groupes de négociation suivis — le patron de `routes/themes.rs`.

use actix_web::http::header::{ETAG, IF_MATCH};
use actix_web::{web, HttpRequest, HttpResponse};
use kernel::auth::Actor;
use kernel::error::Result;

use crate::domain::groups::{GroupsPayload, MyGroups};
use crate::service::groups::{self, Remplacement};
use crate::state::NegotiationState;

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/negotiation/me/groups", web::get().to(mes_groupes))
        .route("/negotiation/me/groups", web::put().to(suivre_des_groupes));
}

#[utoipa::path(
    get,
    description = "`MyGroups` — les groupes de négociation que la personne connectée suit : des codes du vocabulaire `negotiation_group`, jamais de libellés, et l'empreinte de cet état. Aucun groupe : `200` et une liste vide.\n\n`ETag` calculé sur les codes triés ; **304** sur `If-None-Match`.",
    path = "/negotiation/me/groups",
    tag = "Guide Négo — sessions officielles",
    operation_id = "negotiation_mes_groupes",
    responses(
        (status = 200, description = "MyGroups", body = Object),
        (status = 304, description = "Rien n'a changé depuis l'empreinte présentée"),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn mes_groupes(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    acteur: Actor,
) -> Result<HttpResponse> {
    let mes = groups::mes_groupes(&state, acteur.0).await?;

    if crate::routes::inchange(&requete, &mes.etag) {
        return Ok(HttpResponse::NotModified()
            .insert_header((ETAG, mes.etag))
            .insert_header(crate::routes::PERSONNEL)
            .finish());
    }

    Ok(rendre(mes))
}

#[utoipa::path(
    put,
    description = "`GroupsPayload` → `MyGroups` — remplacer **en bloc** la liste des groupes suivis. La liste vide est permise : « aucun groupe » est un choix. Rejouer le même corps n'écrit rien de plus.\n\n`If-Match` porte l'empreinte de l'état sur lequel le choix a été pris ; différente de l'état courant : **412**, aucune écriture. Un code inconnu est refusé en le nommant.",
    path = "/negotiation/me/groups",
    tag = "Guide Négo — sessions officielles",
    operation_id = "negotiation_suivre_des_groupes",
    request_body = Object,
    responses(
        (status = 200, description = "MyGroups", body = Object),
        (status = 400, description = "Groupe inconnu — le message nomme le code", body = crate::routes::openapi::ApiErrorBody),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 412, description = "L'état a changé depuis l'empreinte présentée en If-Match", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Corps malformé", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn suivre_des_groupes(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    acteur: Actor,
    charge: web::Json<GroupsPayload>,
) -> Result<HttpResponse> {
    let contexte = crate::routes::contexte_de(&requete, acteur.0);
    let si_correspond = crate::routes::entete(&requete, IF_MATCH.as_str());

    let mes = groups::remplacer(
        &state,
        &contexte,
        Remplacement {
            person_id: acteur.0,
            codes: &charge.groups,
            si_correspond: si_correspond.as_deref(),
        },
    )
    .await?;

    Ok(rendre(mes))
}

fn rendre(mes: MyGroups) -> HttpResponse {
    HttpResponse::Ok()
        .insert_header((ETAG, mes.etag.clone()))
        .insert_header(crate::routes::PERSONNEL)
        .json(mes)
}
