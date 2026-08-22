//! Les deux routes du calendrier des rappels, **sans le préfixe**.
//!
//! `/sessions` appartient au module Programmation depuis B5. Deux `web::scope`
//! du même préfixe **ne se complètent pas** : Actix retient le premier dont le
//! préfixe correspond et rend 404 si la route n'y figure pas, sans essayer le
//! suivant. Le scope est donc composé une seule fois par l'API, à partir des
//! deux modules — et aucune route de B5 ne change de chemin.

use actix_web::{web, HttpResponse};
use kernel::auth::Actor;
use kernel::error::Result;
use uuid::Uuid;

use crate::service::schedule;
use crate::state::EngagementState;

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/{id}/reminders", web::get().to(calendrier))
        .route("/{id}/reminder-rule", web::get().to(regle_applicable));
}

/// Le calendrier des rappels d'une séance.
#[utoipa::path(
    get,
    description = "`{ slots, has_rule }` — **une ligne par (décalage, canal), et pas un nom.** Quarante inscrits et quatre décalages rendent **quatre** lignes portant chacune quarante destinataires, jamais cent soixante : l'organisation qui anime a droit au NOMBRE de destinataires, pas à leur identité. La garantie est portée par la signature de la fonction du modèle, pas par la discipline d'un appelant.\n\nLes lignes sont rangées **du décalage le plus lointain au plus proche**, en minutes : `'1 day'` et `'24 hours'` sont le même intervalle pour la base et deux chaînes différentes pour un écran, ce qui suffirait à afficher deux fois le même rappel.\n\nL'état d'une ligne est celui de la ligne **la moins avancée** du groupe : une seule personne qui attend encore son courriel suffit à dire « en attente ». « Parti » ne se dit pas tant qu'il reste quelqu'un.\n\n**`has_rule` distingue « aucune règle » de « tout est parti »** : une liste vide muette se confond avec un envoi réussi, et les deux situations demandent des mots différents à l'écran.\n\nGardé par l'**adhésion active** à l'organisation qui anime, ou par `programme.registration.manage` sur l'édition de la séance — jamais par un périmètre d'administration, une organisation n'administrant rien.",
    path = "/sessions/{id}/reminders",
    tag = "Rappels — calendrier",
    operation_id = "engagement_calendrier_des_rappels",
    params(("id" = Uuid, Path, description = "Identifiant de la séance")),
    responses(
        (status = 200, description = "{ slots, has_rule }", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Ni adhésion active, ni droit de gérer les inscriptions", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Séance inexistante", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn calendrier(
    state: web::Data<EngagementState>,
    Actor(acteur): Actor,
    id: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let calendrier = schedule::calendrier(&state, acteur, id.into_inner()).await?;
    Ok(HttpResponse::Ok().json(calendrier))
}

/// La règle applicable à une séance, avec son origine.
#[utoipa::path(
    get,
    description = "`ApplicableReminderRule | null` — la règle qui gouverne **effectivement** cette séance, et `null` quand aucune ne s'applique.\n\n**Une règle de séance REMPLACE celle de son édition**, sans cumul. La réponse porte donc l'**origine** — `session` ou `event` — et l'identifiant dont elle vient : sans elle, une règle de séance à deux décalages ne se distingue pas d'une règle d'édition qu'on aurait tronquée, et la non-cumulation cesse d'être vérifiable de l'extérieur.\n\nMême garde que le calendrier : adhésion active, ou droit de gérer les inscriptions de l'édition.",
    path = "/sessions/{id}/reminder-rule",
    tag = "Rappels — calendrier",
    operation_id = "engagement_regle_de_rappel_applicable",
    params(("id" = Uuid, Path, description = "Identifiant de la séance")),
    responses(
        (status = 200, description = "ApplicableReminderRule | null", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Ni adhésion active, ni droit de gérer les inscriptions", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Séance inexistante", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn regle_applicable(
    state: web::Data<EngagementState>,
    Actor(acteur): Actor,
    id: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let regle = schedule::regle_applicable(&state, acteur, id.into_inner()).await?;
    Ok(HttpResponse::Ok().json(regle))
}
