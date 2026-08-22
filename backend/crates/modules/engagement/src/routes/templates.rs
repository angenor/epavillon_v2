//! Les cinq routes des modèles de messages.
//!
//! Le préfixe `/admin/message-templates` n'appartient qu'à ce module : il n'y a
//! rien à composer côté API, contrairement à `/sessions`.

use actix_web::{web, HttpRequest, HttpResponse};
use kernel::auth::Actor;
use kernel::error::Result;
use uuid::Uuid;

use crate::domain::template::TemplateVersionPayload;
use crate::routes::contexte_de;
use crate::service::templates::{self, PreviewPayload};
use crate::state::EngagementState;

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/admin/message-templates", web::get().to(lister))
        .route("/admin/message-templates/{id}", web::get().to(detail))
        .route(
            "/admin/message-templates/{id}/versions",
            web::post().to(ecrire_revision),
        )
        .route(
            "/admin/message-templates/{id}/versions/{version}/publish",
            web::post().to(publier),
        )
        .route(
            "/admin/message-templates/{id}/preview",
            web::post().to(apercu),
        );
}

/// La liste des modèles.
#[utoipa::path(
    get,
    description = "`MessageTemplateRow[]` — les modèles, avec le nombre de révisions de chacun et **celle qui est servie**. `current_version` nul dit qu'aucune n'est publiée : le type part alors avec le texte de secours du module, et la trace d'expédition le dit.\n\nLe compte des révisions vient de la même réponse que la liste : deux appels donneraient deux instants, et un écran annonçant « 3 révisions » sur une liste qui en montre quatre.",
    path = "/admin/message-templates",
    tag = "Back-office — modèles de messages",
    operation_id = "engagement_modeles_de_messages",
    responses(
        (status = 200, description = "MessageTemplateRow[]", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission de gérer les modèles absente", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn lister(
    state: web::Data<EngagementState>,
    Actor(acteur): Actor,
) -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(templates::lister(&state, acteur).await?))
}

/// Le détail d'un modèle.
#[utoipa::path(
    get,
    description = "`TemplateDetail` — les révisions de la plus récente à la plus ancienne, celle qui est **servie**, et **les variables que le type promet**.\n\nCette dernière liste n'est pas décorative : sans elle, l'écran ne peut annoncer les variables disponibles qu'en les devinant, et un administrateur découvrirait le refus à la publication, après avoir écrit son gabarit.",
    path = "/admin/message-templates/{id}",
    tag = "Back-office — modèles de messages",
    operation_id = "engagement_modele_de_message",
    params(("id" = Uuid, Path, description = "Identifiant du modèle")),
    responses(
        (status = 200, description = "TemplateDetail", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission de gérer les modèles absente", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Modèle inexistant", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn detail(
    state: web::Data<EngagementState>,
    Actor(acteur): Actor,
    id: web::Path<Uuid>,
) -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(templates::detail(&state, acteur, id.into_inner()).await?))
}

/// Écrire une révision.
#[utoipa::path(
    post,
    description = "`TemplateVersion` — le corps HTML est **assaini à l'écriture**, langue par langue, contre une liste blanche propre au courriel : tableaux et styles en ligne y sont admis, parce que les clients de messagerie ignorent les feuilles de style.\n\n**Un `href=\"{{lien}}\"` survit** : pour un analyseur d'URL, une variable est une adresse relative, et la normaliser détruirait le lien — un défaut qui ne se voit qu'à la réception du courriel.\n\n**Une révision écrite n'est PAS servie.** Publier est un second geste : sans cette séparation, enregistrer une correction à moitié faite l'enverrait à deux mille personnes.\n\nLe numéro de révision n'est pas reçu, il est **posé** : deux administrateurs qui enregistrent en même temps ne doivent pas se disputer un numéro.",
    path = "/admin/message-templates/{id}/versions",
    tag = "Back-office — modèles de messages",
    operation_id = "engagement_ecrire_revision_de_modele",
    params(("id" = Uuid, Path, description = "Identifiant du modèle")),
    request_body = TemplateVersionPayload,
    responses(
        (status = 200, description = "TemplateVersion", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission de gérer les modèles absente", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Modèle inexistant", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "VALIDATION_FAILED", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn ecrire_revision(
    state: web::Data<EngagementState>,
    Actor(acteur): Actor,
    requete: HttpRequest,
    id: web::Path<Uuid>,
    payload: web::Json<TemplateVersionPayload>,
) -> Result<HttpResponse> {
    let ctx = contexte_de(&requete, acteur);
    let revision =
        templates::ecrire_revision(&state, &ctx, acteur, id.into_inner(), &payload).await?;
    Ok(HttpResponse::Ok().json(revision))
}

/// Publier une révision — ou revenir à une précédente.
#[utoipa::path(
    post,
    description = "`TemplateDetail` — **publier fait avancer un pointeur, et republier une révision antérieure est le retour arrière.** Rien n'est jamais effacé.\n\n**Refusée si le gabarit cite une variable que le type ne promet pas**, en la nommant et en listant celles qui le sont. Le refus arrive ici et non à l'envoi : à l'envoi, il serait trop tard pour corriger sans que personne n'ait rien reçu — le courriel partirait avec un trou, « Bonjour  , », et le trou ne se verrait qu'à la réception.",
    path = "/admin/message-templates/{id}/versions/{version}/publish",
    tag = "Back-office — modèles de messages",
    operation_id = "engagement_publier_revision_de_modele",
    params(
        ("id" = Uuid, Path, description = "Identifiant du modèle"),
        ("version" = i16, Path, description = "Numéro de la révision"),
    ),
    responses(
        (status = 200, description = "TemplateDetail", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission de gérer les modèles absente", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "ENGAGEMENT_TEMPLATE_VERSION_UNKNOWN", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "ENGAGEMENT_TEMPLATE_VARIABLE_UNKNOWN · ENGAGEMENT_NOTIFICATION_TYPE_UNKNOWN", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn publier(
    state: web::Data<EngagementState>,
    Actor(acteur): Actor,
    requete: HttpRequest,
    chemin: web::Path<(Uuid, i16)>,
) -> Result<HttpResponse> {
    let (template_id, version) = chemin.into_inner();
    let ctx = contexte_de(&requete, acteur);
    let detail = templates::publier(&state, &ctx, acteur, template_id, version).await?;
    Ok(HttpResponse::Ok().json(detail))
}

/// L'aperçu — il n'envoie rien.
#[utoipa::path(
    post,
    description = "`{ fr, en }` — le rendu dans les **deux langues**, avec des valeurs d'exemple. **N'envoie rien, n'écrit aucune trace d'expédition, n'appelle pas l'expéditeur.**\n\nSans `version`, la révision servie est rendue — ou la plus récente si aucune n'est publiée, un brouillon devant se relire avant d'être publié.\n\n**Une variable absente ne fait pas échouer l'aperçu** : elle prend une valeur d'exemple visible, `« prenom »`. Un aperçu sert à regarder une mise en page ; refuser de la montrer parce qu'un exemple manque le rendrait inutile. À l'envoi, la règle est l'inverse — un trou part chez deux mille personnes, et l'échec est la bonne réponse.\n\nUne langue absente du gabarit se replie sur le français.",
    path = "/admin/message-templates/{id}/preview",
    tag = "Back-office — modèles de messages",
    operation_id = "engagement_apercu_de_modele",
    params(("id" = Uuid, Path, description = "Identifiant du modèle")),
    request_body = PreviewPayload,
    responses(
        (status = 200, description = "{ fr, en }", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission de gérer les modèles absente", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Modèle ou révision inexistante", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn apercu(
    state: web::Data<EngagementState>,
    Actor(acteur): Actor,
    id: web::Path<Uuid>,
    payload: web::Json<PreviewPayload>,
) -> Result<HttpResponse> {
    let apercu = templates::apercu(&state, acteur, id.into_inner(), &payload).await?;
    Ok(HttpResponse::Ok().json(apercu))
}
