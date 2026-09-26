//! Les thématiques suivies — **deux routes, un même corps**.
//!
//! L'empreinte se calcule sur les codes triés, jamais sur le corps rendu : elle
//! dit **l'état**, et deux appareils dans deux langues la voient identique. Le
//! `PUT` la reçoit en `If-Match` et refuse en `412` ce qui arrive trop tard.

use actix_web::http::header::{ETAG, IF_MATCH};
use actix_web::{web, HttpRequest, HttpResponse};
use kernel::auth::Actor;
use kernel::error::Result;

use crate::domain::notifications::ThemeNotificationsPayload;
use crate::domain::themes::{MyThemes, ThemesPayload};
use crate::service::themes::{self, Remplacement};
use crate::state::NegotiationState;

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/negotiation/me/themes", web::get().to(mes_thematiques))
        .route(
            "/negotiation/me/themes",
            web::put().to(suivre_des_thematiques),
        )
        .route(
            "/negotiation/me/themes/notifications",
            web::put().to(notifier_des_thematiques),
        );
}

#[utoipa::path(
    get,
    description = "`MyThemes` — les thématiques de négociation que la personne connectée suit : **des codes et leur date de suivi, jamais de libellés**. Les libellés viennent de `GET /reference/taxonomies/negotiation_theme/terms`, seule source, que le client lit de toute façon.\n\nSans thématique suivie : `200` et une liste vide, jamais un 404.\n\nL'`ETag` est calculé **sur les codes triés** — l'état, pas sa représentation —, et rend **304** sur `If-None-Match`. C'est cette même empreinte qu'un choix pris hors connexion renvoie en `If-Match`.",
    path = "/negotiation/me/themes",
    tag = "Guide Négo — thématiques",
    operation_id = "negotiation_mes_thematiques",
    responses(
        (status = 200, description = "MyThemes", body = Object),
        (status = 304, description = "Rien n'a changé depuis l'empreinte présentée"),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn mes_thematiques(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    acteur: Actor,
) -> Result<HttpResponse> {
    let mes = themes::mes_thematiques(&state, acteur.0).await?;
    let empreinte = mes.empreinte();

    if crate::routes::inchange(&requete, &empreinte) {
        return Ok(HttpResponse::NotModified()
            .insert_header((ETAG, empreinte))
            .insert_header(crate::routes::PERSONNEL)
            .finish());
    }

    Ok(rendre(mes, empreinte))
}

#[utoipa::path(
    put,
    description = "`ThemesPayload` → `MyThemes` — remplacer **en bloc** la liste des thématiques suivies. Jamais un ajout ni un retrait unitaire : le corps porte la liste entière, et rejouer le même corps donne le même état sans rien écrire de plus.\n\n`If-Match` porte l'empreinte de l'état sur lequel le choix a été pris. Absent, accepté — l'écran en ligne vient de lire. Différent de l'état courant : **412**, aucune écriture ; l'application abandonne l'intention, relit, et le dit. C'est ce qui empêche un choix parti en retard d'un téléphone d'effacer un choix plus récent fait sur une tablette.\n\nLes suivis absents de la liste sont **fermés**, jamais supprimés. Un code inconnu, ou d'un autre vocabulaire, est refusé en nommant le code. Un terme retiré du vocabulaire reste à qui le suivait et ne se choisit plus.",
    path = "/negotiation/me/themes",
    tag = "Guide Négo — thématiques",
    operation_id = "negotiation_suivre_des_thematiques",
    request_body = Object,
    responses(
        (status = 200, description = "MyThemes", body = Object),
        (status = 400, description = "Liste vide, ou thématique inconnue — le message nomme le code", body = crate::routes::openapi::ApiErrorBody),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 412, description = "L'état a changé depuis l'empreinte présentée en If-Match", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Corps malformé", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn suivre_des_thematiques(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    acteur: Actor,
    charge: web::Json<ThemesPayload>,
) -> Result<HttpResponse> {
    let contexte = crate::routes::contexte_de(&requete, acteur.0);
    let si_correspond = crate::routes::entete(&requete, IF_MATCH.as_str());

    let mes = themes::remplacer(
        &state,
        &contexte,
        Remplacement {
            person_id: acteur.0,
            codes: &charge.codes,
            si_correspond: si_correspond.as_deref(),
        },
    )
    .await?;

    let empreinte = mes.empreinte();
    Ok(rendre(mes, empreinte))
}

#[utoipa::path(
    put,
    description = "`ThemeNotificationsPayload` → `MyThemes` — les thématiques dont la personne veut être prévenue des changements, **parmi celles qu'elle suit** ; la liste entière, jamais un delta. Vide : tout éteint. Une ligne allumée élargit, elle ne coupe jamais : les sessions de « Mon agenda » préviennent toujours. Quitter une thématique l'éteint avec elle.\n\n`notify` entre dans l'empreinte de `GET /negotiation/me/themes`.",
    path = "/negotiation/me/themes/notifications",
    tag = "Guide Négo — thématiques",
    operation_id = "negotiation_notifier_des_thematiques",
    request_body = Object,
    responses(
        (status = 200, description = "MyThemes", body = Object),
        (status = 400, description = "Thématique non suivie — le message nomme le code", body = crate::routes::openapi::ApiErrorBody),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Corps malformé", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn notifier_des_thematiques(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    acteur: Actor,
    charge: web::Json<ThemeNotificationsPayload>,
) -> Result<HttpResponse> {
    let contexte = crate::routes::contexte_de(&requete, acteur.0);
    let mes = themes::notifier(&state, &contexte, acteur.0, &charge.codes).await?;
    let empreinte = mes.empreinte();
    Ok(rendre(mes, empreinte))
}

fn rendre(mes: MyThemes, empreinte: String) -> HttpResponse {
    HttpResponse::Ok()
        .insert_header((ETAG, empreinte))
        .insert_header(crate::routes::PERSONNEL)
        .json(mes)
}
