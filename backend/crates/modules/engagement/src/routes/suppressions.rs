//! La liste de suppression : la garde-fou de la réputation d'envoi.

use actix_web::{web, HttpRequest, HttpResponse};
use kernel::auth::Actor;
use kernel::error::Result;
use serde::Serialize;

use crate::routes::contexte_de;
use crate::service::deliverability::{self, RechercheQuery, SuppressionPayload};
use crate::state::EngagementState;

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/admin/email-suppressions", web::get().to(lister))
        .route("/admin/email-suppressions", web::post().to(poser))
        .route(
            "/admin/email-suppressions/{email}",
            web::delete().to(retirer),
        );
}

#[derive(Debug, Serialize)]
struct Retrait {
    removed: bool,
}

/// La liste.
#[utoipa::path(
    get,
    description = "`EmailSuppression[]` — les adresses écartées du circuit, de la plus récente à la plus ancienne.\n\n**Une suppression échue reste visible.** Elle n'écarte plus rien — `is_email_suppressed()` compare `expires_at` à maintenant —, mais savoir qu'une adresse a rebondi le mois dernier a de la valeur. Aucun travail récurrent ne les efface : une purge programmée serait un second dispositif à tenir d'accord avec la fonction du modèle.",
    path = "/admin/email-suppressions",
    tag = "Délivrabilité",
    operation_id = "engagement_liste_de_suppression",
    params(("q" = Option<String>, Query, description = "Fragment d'adresse")),
    responses(
        (status = 200, description = "EmailSuppression[]", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn lister(
    state: web::Data<EngagementState>,
    Actor(acteur): Actor,
    recherche: web::Query<RechercheQuery>,
) -> Result<HttpResponse> {
    let lignes = deliverability::lister(&state, acteur, recherche.q.as_deref()).await?;
    Ok(HttpResponse::Ok().json(lignes))
}

/// Inscrire une adresse.
#[utoipa::path(
    post,
    description = "`EmailSuppression` — **aucun module n'écrira plus à cette adresse**, y compris ceux livrés avant ce jalon : la garde enveloppe le contrat d'envoi du noyau, et aucun d'eux n'a été modifié.\n\nUne adresse déjà inscrite est **mise à jour**, jamais refusée : un second rebond ne doit pas produire un conflit, et le motif le plus récent est celui qui explique le mieux pourquoi la personne ne reçoit plus rien.\n\n`expires_at` lève la suppression toute seule le moment venu — une boîte pleine n'est pas une adresse morte.\n\nL'inscription émet `engagement.email.suppressed`, dont la charge utile porte l'adresse **hachée** : l'outbox est durable, indexée et relayée, et une adresse électronique est une donnée personnelle.",
    path = "/admin/email-suppressions",
    tag = "Délivrabilité",
    operation_id = "engagement_supprimer_une_adresse",
    request_body = SuppressionPayload,
    responses(
        (status = 200, description = "EmailSuppression", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "VALIDATION_FAILED", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn poser(
    state: web::Data<EngagementState>,
    Actor(acteur): Actor,
    requete: HttpRequest,
    payload: web::Json<SuppressionPayload>,
) -> Result<HttpResponse> {
    let ctx = contexte_de(&requete, acteur);
    Ok(HttpResponse::Ok().json(deliverability::poser(&state, &ctx, acteur, &payload).await?))
}

/// Retirer une adresse.
#[utoipa::path(
    delete,
    description = "`{ removed }` — l'adresse redevient joignable. `false` dit qu'elle n'y était pas : ce n'est pas une erreur, et rendre 404 obligerait l'écran à traiter comme un échec un état qui est celui qu'on voulait.",
    path = "/admin/email-suppressions/{email}",
    tag = "Délivrabilité",
    operation_id = "engagement_retirer_une_suppression",
    params(("email" = String, Path, description = "L'adresse à libérer")),
    responses(
        (status = 200, description = "{ removed }", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn retirer(
    state: web::Data<EngagementState>,
    Actor(acteur): Actor,
    requete: HttpRequest,
    email: web::Path<String>,
) -> Result<HttpResponse> {
    let ctx = contexte_de(&requete, acteur);
    let removed = deliverability::retirer(&state, &ctx, acteur, &email).await?;
    Ok(HttpResponse::Ok().json(Retrait { removed }))
}
