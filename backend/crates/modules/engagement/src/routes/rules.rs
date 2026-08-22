//! Les trois routes d'administration des règles de rappel.
//!
//! Le préfixe `/admin/reminder-rules` n'appartient qu'à ce module : il n'y a
//! rien à composer côté API, contrairement à `/sessions`.

use actix_web::{web, HttpRequest, HttpResponse};
use kernel::auth::Actor;
use kernel::error::Result;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::routes::contexte_de;
use crate::service::rules::{self, ReminderRulePayload};
use crate::state::EngagementState;

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/admin/reminder-rules", web::get().to(lister))
        .route("/admin/reminder-rules", web::put().to(ecrire))
        .route("/admin/reminder-rules/{id}", web::delete().to(supprimer));
}

#[derive(Debug, Deserialize)]
pub struct EditionQuery {
    pub event_id: Uuid,
}

/// Ce que la coupure rend.
#[derive(Debug, Serialize)]
struct Coupure {
    /// Les rappels **encore à traiter** qui ont été annulés. Zéro est une
    /// réponse : la règle pouvait n'avoir rien matérialisé.
    cancelled_reminders: i64,
}

/// Les règles d'une édition.
#[utoipa::path(
    get,
    description = "`ReminderRule[]` — la règle de l'édition **et** celles de ses séances, dans cet ordre. Les décalages sont rendus **en minutes**, rangés du plus lointain au plus proche : `'1 day'` et `'24 hours'` sont le même intervalle pour la base et deux chaînes différentes pour un écran, ce qui suffirait à afficher deux fois le même rappel.\n\nGardé par `engagement.reminder.manage` **sur la portée de l'édition**, et borné par le périmètre d'administration : un compte détaché sur une COP ne lit pas les règles de celle d'à côté, y compris en forgeant l'URL.",
    path = "/admin/reminder-rules",
    tag = "Back-office — règles de rappel",
    operation_id = "engagement_regles_de_rappel",
    params(("event_id" = Uuid, Query, description = "L'édition dont on lit les règles")),
    responses(
        (status = 200, description = "ReminderRule[]", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Édition inexistante ou hors périmètre", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn lister(
    state: web::Data<EngagementState>,
    Actor(acteur): Actor,
    edition: web::Query<EditionQuery>,
) -> Result<HttpResponse> {
    let regles = rules::lister(&state, acteur, edition.event_id).await?;
    Ok(HttpResponse::Ok().json(regles))
}

/// Écrire — ou modifier — la règle d'une portée.
#[utoipa::path(
    put,
    description = "`ReminderRule` — **une LISTE de décalages, jamais un décalage seul.** Les quatre valeurs du défaut — 2 jours, 1 jour, 1 heure, 30 minutes — sont **cumulées** : ce n'est pas un choix parmi quatre, les quatre rappels partent. Une écriture qui n'accepterait qu'une valeur ferait croire le contraire, et la faute ne se verrait qu'au jour de la séance.\n\n**Une règle de séance REMPLACE celle de son édition**, sans cumul — c'est ce qui permet de savoir ce qui va partir.\n\n**Une seconde écriture pour la même portée MODIFIE la première** : l'unicité du modèle est traitée comme une modification, jamais comme une erreur. Rendre un conflit dirait « impossible » là où l'on voulait simplement changer ses décalages.\n\n**La portée est exactement l'une des deux** — une édition ou une séance, jamais les deux, jamais aucune. Le refus sort sur le champ `scope`, et celui des décalages sur `offsets`, en disant lequel des quatre cas s'applique : trop peu, trop, négatif, ou **répété** — ce dernier étant absorbé en silence par la clé d'unicité du modèle, l'écran annonçant alors un envoi de plus qu'il n'y en aurait.",
    path = "/admin/reminder-rules",
    tag = "Back-office — règles de rappel",
    operation_id = "engagement_ecrire_regle_de_rappel",
    request_body = ReminderRulePayload,
    responses(
        (status = 200, description = "ReminderRule", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Édition ou séance inexistante, ou hors périmètre", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "ENGAGEMENT_REMINDER_OFFSETS_INVALID · ENGAGEMENT_REMINDER_SCOPE_INVALID", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn ecrire(
    state: web::Data<EngagementState>,
    Actor(acteur): Actor,
    requete: HttpRequest,
    payload: web::Json<ReminderRulePayload>,
) -> Result<HttpResponse> {
    let ctx = contexte_de(&requete, acteur);
    let regle = rules::ecrire(&state, &ctx, acteur, &payload).await?;
    Ok(HttpResponse::Ok().json(regle))
}

/// Couper une règle.
#[utoipa::path(
    delete,
    description = "`{ cancelled_reminders }` — supprime la règle **et annule les rappels encore à traiter qu'elle gouvernait**, en rendant leur nombre.\n\nLes annuler est ce qui distingue une coupure d'un simple oubli : sans cela, les rappels **déjà matérialisés** partiraient quand même, et l'administrateur qui vient de retirer la règle les verrait arriver sans comprendre.\n\nPour **couper sans supprimer**, écrire la règle avec `is_active: false` : les rappels déjà posés restent alors en place.",
    path = "/admin/reminder-rules/{id}",
    tag = "Back-office — règles de rappel",
    operation_id = "engagement_supprimer_regle_de_rappel",
    params(("id" = Uuid, Path, description = "Identifiant de la règle")),
    responses(
        (status = 200, description = "{ cancelled_reminders }", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Règle inexistante, ou édition hors périmètre", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn supprimer(
    state: web::Data<EngagementState>,
    Actor(acteur): Actor,
    requete: HttpRequest,
    id: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let ctx = contexte_de(&requete, acteur);
    let annules = rules::supprimer(&state, &ctx, acteur, id.into_inner()).await?;
    Ok(HttpResponse::Ok().json(Coupure {
        cancelled_reminders: annules,
    }))
}
