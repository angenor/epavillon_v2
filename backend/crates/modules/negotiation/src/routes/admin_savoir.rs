//! Le back-office du savoir : FAQ, lexique, parcours. **Portée globale
//! partout.** Rédiger et publier : `Requires<KnowledgePublish>` ; dater une
//! vérification : `Requires<KnowledgeReview>` ; lire : `LectureSavoir`, l'une
//! **ou** l'autre.

use std::future::Future;
use std::pin::Pin;

use actix_web::dev::Payload;
use actix_web::{web, FromRequest, HttpMessage, HttpRequest, HttpResponse};
use kernel::auth::Requires;
use kernel::context::RequestContext;
use kernel::error::{ApiError, Result};
use uuid::Uuid;

use crate::domain::admin_savoir::{
    AdminFaqInput, AdminFaqVerifyInput, AdminGlossaryInput, AdminPathwayGroupInput,
    AdminPathwayOrderInput, AdminPathwayStepInput, FiltreSavoir,
};
use crate::domain::permissions::{KnowledgePublish, KnowledgeReview};
use crate::service::savoir_admin::{self as service, Droits, Transition};
use crate::state::NegotiationState;

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/admin/negotiation/faq", web::get().to(liste_faq))
        .route("/admin/negotiation/faq", web::post().to(creer_faq))
        .route("/admin/negotiation/faq/{id}", web::get().to(fiche_faq))
        .route("/admin/negotiation/faq/{id}", web::patch().to(modifier_faq))
        .route(
            "/admin/negotiation/faq/{id}",
            web::delete().to(supprimer_faq),
        )
        .route(
            "/admin/negotiation/faq/{id}/verify",
            web::post().to(verifier_faq),
        )
        .route(
            "/admin/negotiation/faq/{id}/publish",
            web::post().to(publier_faq),
        )
        .route(
            "/admin/negotiation/faq/{id}/to-review",
            web::post().to(faq_a_revoir),
        )
        .route(
            "/admin/negotiation/faq/{id}/unpublish",
            web::post().to(depublier_faq),
        )
        .route("/admin/negotiation/glossary", web::get().to(liste_lexique))
        .route("/admin/negotiation/glossary", web::post().to(creer_terme))
        .route(
            "/admin/negotiation/glossary/{id}",
            web::get().to(fiche_terme),
        )
        .route(
            "/admin/negotiation/glossary/{id}",
            web::patch().to(modifier_terme),
        )
        .route(
            "/admin/negotiation/glossary/{id}",
            web::delete().to(supprimer_terme),
        )
        .route(
            "/admin/negotiation/glossary/{id}/publish",
            web::post().to(publier_terme),
        )
        .route(
            "/admin/negotiation/glossary/{id}/to-review",
            web::post().to(terme_a_revoir),
        )
        .route(
            "/admin/negotiation/glossary/{id}/unpublish",
            web::post().to(depublier_terme),
        )
        .route("/admin/negotiation/pathway", web::get().to(parcours))
        .route(
            "/admin/negotiation/pathway/groups",
            web::post().to(creer_groupe),
        )
        .route(
            "/admin/negotiation/pathway/groups/{id}",
            web::patch().to(modifier_groupe),
        )
        .route(
            "/admin/negotiation/pathway/groups/{id}",
            web::delete().to(supprimer_groupe),
        )
        .route(
            "/admin/negotiation/pathway/steps",
            web::post().to(creer_etape),
        )
        .route(
            "/admin/negotiation/pathway/steps/{id}",
            web::patch().to(modifier_etape),
        )
        .route(
            "/admin/negotiation/pathway/steps/{id}",
            web::delete().to(supprimer_etape),
        )
        .route("/admin/negotiation/pathway/order", web::put().to(ordonner));
}

/// Lire les listes, les fiches et le parcours : publier **ou** vérifier, sur
/// la portée globale.
pub struct LectureSavoir {
    pub person_id: Uuid,
    pub droits: Droits,
}

impl FromRequest for LectureSavoir {
    type Error = ApiError;
    type Future = Pin<Box<dyn Future<Output = Result<Self>>>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let acteur = req
            .extensions()
            .get::<RequestContext>()
            .and_then(|c| c.actor_id);
        let state = req.app_data::<web::Data<NegotiationState>>().cloned();
        Box::pin(async move {
            let person_id = acteur.ok_or_else(ApiError::unauthenticated)?;
            let state = state.ok_or_else(|| ApiError::internal("état du module absent"))?;
            let droits = service::exiger_la_lecture(&state, person_id).await?;
            Ok(LectureSavoir { person_id, droits })
        })
    }
}

async fn rendre_faq(
    state: &NegotiationState,
    requete: &HttpRequest,
    personne: Uuid,
    id: Uuid,
) -> Result<HttpResponse> {
    let droits = service::droits(state, personne).await?;
    let locale = crate::routes::locale_de(requete);
    Ok(HttpResponse::Ok().json(service::fiche_faq(state, &droits, id, &locale).await?))
}

async fn rendre_terme(
    state: &NegotiationState,
    requete: &HttpRequest,
    personne: Uuid,
    id: Uuid,
) -> Result<HttpResponse> {
    let droits = service::droits(state, personne).await?;
    let locale = crate::routes::locale_de(requete);
    Ok(HttpResponse::Ok().json(service::fiche_terme(state, &droits, id, &locale).await?))
}

async fn rendre_parcours(
    state: &NegotiationState,
    requete: &HttpRequest,
    personne: Uuid,
) -> Result<HttpResponse> {
    let droits = service::droits(state, personne).await?;
    let locale = crate::routes::locale_de(requete);
    Ok(HttpResponse::Ok().json(service::parcours(state, &droits, &locale).await?))
}

// ---------------------------------------------------------------------------
// FAQ
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    description = "`AdminFaqList` — toutes les entrées, brouillons compris, avec leurs signalements ouverts comptés. `q` cherche par trigrammes dans la question. Ouvert à qui publie ou vérifie, sur la portée globale.",
    path = "/admin/negotiation/faq",
    tag = "Back-office — savoir",
    operation_id = "admin_negotiation_faq",
    params(
        ("q" = Option<String>, Query, description = "Texte cherché dans la question"),
        ("section" = Option<String>, Query, description = "Code de rubrique (`faq_section`)"),
        ("status" = Option<String>, Query, description = "`draft`, `published` ou `to_review`"),
    ),
    responses(
        (status = 200, description = "AdminFaqList", body = Object),
        (status = 401, description = "Aucune session", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Ni publier ni vérifier", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn liste_faq(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    lecteur: LectureSavoir,
    filtre: web::Query<FiltreSavoir>,
) -> Result<HttpResponse> {
    let locale = crate::routes::locale_de(&requete);
    let liste = service::liste_faq(&state, &lecteur.droits, &filtre, &locale).await?;
    Ok(HttpResponse::Ok().json(liste))
}

#[utoipa::path(
    post,
    description = "`AdminFaqInput` → `AdminFaqEntry` — crée un **brouillon**. Rubrique et question en français exigées.",
    path = "/admin/negotiation/faq",
    tag = "Back-office — savoir",
    operation_id = "admin_negotiation_faq_creer",
    request_body = Object,
    responses(
        (status = 201, description = "AdminFaqEntry", body = Object),
        (status = 403, description = "Sans la permission de publier", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Texte, rubrique, source ou liée invalides", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn creer_faq(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    acteur: Requires<KnowledgePublish>,
    entree: web::Json<AdminFaqInput>,
) -> Result<HttpResponse> {
    let ctx = crate::routes::contexte_de(&requete, acteur.person_id);
    let id = service::creer_faq(&state, &ctx, &entree).await?;
    let droits = service::droits(&state, acteur.person_id).await?;
    let locale = crate::routes::locale_de(&requete);
    Ok(HttpResponse::Created().json(service::fiche_faq(&state, &droits, id, &locale).await?))
}

#[utoipa::path(
    get,
    description = "`AdminFaqEntry` — la fiche, textes non résolus, avec sources, liées, retours « Oui / Non » comptés et signalements, **sans aucun auteur**.",
    path = "/admin/negotiation/faq/{id}",
    tag = "Back-office — savoir",
    operation_id = "admin_negotiation_faq_entree",
    params(("id" = Uuid, Path, description = "Identifiant de l'entrée")),
    responses(
        (status = 200, description = "AdminFaqEntry", body = Object),
        (status = 403, description = "Ni publier ni vérifier", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Entrée inconnue", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn fiche_faq(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    lecteur: LectureSavoir,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let locale = crate::routes::locale_de(&requete);
    let fiche = service::fiche_faq(&state, &lecteur.droits, chemin.into_inner(), &locale).await?;
    Ok(HttpResponse::Ok().json(fiche))
}

#[utoipa::path(
    patch,
    description = "`AdminFaqInput` partiel → `AdminFaqEntry` : un champ absent ne change rien, `null` vide un champ facultatif. Sources et liées se remplacent en bloc.",
    path = "/admin/negotiation/faq/{id}",
    tag = "Back-office — savoir",
    operation_id = "admin_negotiation_faq_modifier",
    params(("id" = Uuid, Path, description = "Identifiant de l'entrée")),
    request_body = Object,
    responses(
        (status = 200, description = "AdminFaqEntry", body = Object),
        (status = 403, description = "Sans la permission de publier", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Entrée inconnue", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Texte, rubrique, source ou liée invalides", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn modifier_faq(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    acteur: Requires<KnowledgePublish>,
    chemin: web::Path<Uuid>,
    entree: web::Json<AdminFaqInput>,
) -> Result<HttpResponse> {
    let id = chemin.into_inner();
    let ctx = crate::routes::contexte_de(&requete, acteur.person_id);
    service::modifier_faq(&state, &ctx, id, &entree).await?;
    rendre_faq(&state, &requete, acteur.person_id, id).await
}

#[utoipa::path(
    delete,
    description = "Supprime un **brouillon jamais publié**. Une entrée publiée une fois se dépublie : `NEGOTIATION_KNOWLEDGE_PUBLISHED_UNDELETABLE`.",
    path = "/admin/negotiation/faq/{id}",
    tag = "Back-office — savoir",
    operation_id = "admin_negotiation_faq_supprimer",
    params(("id" = Uuid, Path, description = "Identifiant de l'entrée")),
    responses(
        (status = 204, description = "Supprimée"),
        (status = 403, description = "Sans la permission de publier", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Entrée inconnue", body = crate::routes::openapi::ApiErrorBody),
        (status = 409, description = "Déjà publiée une fois", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn supprimer_faq(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    acteur: Requires<KnowledgePublish>,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let ctx = crate::routes::contexte_de(&requete, acteur.person_id);
    service::supprimer_faq(&state, &ctx, chemin.into_inner()).await?;
    Ok(HttpResponse::NoContent().finish())
}

#[utoipa::path(
    post,
    description = "`AdminFaqVerifyInput` → `AdminFaqEntry` — l'expert date la vérification (aujourd'hui, heure de Paris, sans date) et la signe. Une entrée « À revoir » revient publiée.",
    path = "/admin/negotiation/faq/{id}/verify",
    tag = "Back-office — savoir",
    operation_id = "admin_negotiation_faq_verifier",
    params(("id" = Uuid, Path, description = "Identifiant de l'entrée")),
    request_body = Object,
    responses(
        (status = 200, description = "AdminFaqEntry", body = Object),
        (status = 403, description = "Sans la permission de vérifier", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Entrée inconnue", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn verifier_faq(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    acteur: Requires<KnowledgeReview>,
    chemin: web::Path<Uuid>,
    entree: web::Json<AdminFaqVerifyInput>,
) -> Result<HttpResponse> {
    let id = chemin.into_inner();
    let ctx = crate::routes::contexte_de(&requete, acteur.person_id);
    service::verifier_faq(&state, &ctx, id, &entree).await?;
    rendre_faq(&state, &requete, acteur.person_id, id).await
}

#[utoipa::path(
    post,
    description = "`AdminFaqEntry` — publie, vérifiée ou non. Sans réponse : 422.",
    path = "/admin/negotiation/faq/{id}/publish",
    tag = "Back-office — savoir",
    operation_id = "admin_negotiation_faq_publier",
    params(("id" = Uuid, Path, description = "Identifiant de l'entrée")),
    responses(
        (status = 200, description = "AdminFaqEntry", body = Object),
        (status = 403, description = "Sans la permission de publier", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Entrée inconnue", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Sans vérification datée, ou sans réponse", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn publier_faq(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    acteur: Requires<KnowledgePublish>,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let id = chemin.into_inner();
    let ctx = crate::routes::contexte_de(&requete, acteur.person_id);
    service::changer_faq(&state, &ctx, id, Transition::Publier).await?;
    rendre_faq(&state, &requete, acteur.person_id, id).await
}

#[utoipa::path(
    post,
    description = "`AdminFaqEntry` — met une entrée publiée « À revoir » : elle reste servie, avec sa mention.",
    path = "/admin/negotiation/faq/{id}/to-review",
    tag = "Back-office — savoir",
    operation_id = "admin_negotiation_faq_a_revoir",
    params(("id" = Uuid, Path, description = "Identifiant de l'entrée")),
    responses(
        (status = 200, description = "AdminFaqEntry", body = Object),
        (status = 403, description = "Sans la permission de publier", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Entrée inconnue", body = crate::routes::openapi::ApiErrorBody),
        (status = 409, description = "Entrée jamais publiée", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn faq_a_revoir(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    acteur: Requires<KnowledgePublish>,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let id = chemin.into_inner();
    let ctx = crate::routes::contexte_de(&requete, acteur.person_id);
    service::changer_faq(&state, &ctx, id, Transition::ARevoir).await?;
    rendre_faq(&state, &requete, acteur.person_id, id).await
}

#[utoipa::path(
    post,
    description = "`AdminFaqEntry` — revient au brouillon ; le téléphone la retire à sa relecture.",
    path = "/admin/negotiation/faq/{id}/unpublish",
    tag = "Back-office — savoir",
    operation_id = "admin_negotiation_faq_depublier",
    params(("id" = Uuid, Path, description = "Identifiant de l'entrée")),
    responses(
        (status = 200, description = "AdminFaqEntry", body = Object),
        (status = 403, description = "Sans la permission de publier", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Entrée inconnue", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn depublier_faq(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    acteur: Requires<KnowledgePublish>,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let id = chemin.into_inner();
    let ctx = crate::routes::contexte_de(&requete, acteur.person_id);
    service::changer_faq(&state, &ctx, id, Transition::Depublier).await?;
    rendre_faq(&state, &requete, acteur.person_id, id).await
}

// ---------------------------------------------------------------------------
// Lexique
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    description = "`AdminGlossaryList` — tous les termes, brouillons compris. `q` cherche par trigrammes dans le terme, et exactement dans l'acronyme et les variantes.",
    path = "/admin/negotiation/glossary",
    tag = "Back-office — savoir",
    operation_id = "admin_negotiation_glossary",
    params(
        ("q" = Option<String>, Query, description = "Texte cherché"),
        ("family" = Option<String>, Query, description = "Code de famille (`glossary_family`)"),
        ("status" = Option<String>, Query, description = "`draft`, `published` ou `to_review`"),
    ),
    responses(
        (status = 200, description = "AdminGlossaryList", body = Object),
        (status = 401, description = "Aucune session", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Ni publier ni vérifier", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn liste_lexique(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    lecteur: LectureSavoir,
    filtre: web::Query<FiltreSavoir>,
) -> Result<HttpResponse> {
    let locale = crate::routes::locale_de(&requete);
    let liste = service::liste_lexique(&state, &lecteur.droits, &filtre, &locale).await?;
    Ok(HttpResponse::Ok().json(liste))
}

#[utoipa::path(
    post,
    description = "`AdminGlossaryInput` → `AdminGlossaryEntry` — crée un **brouillon**. Le `slug` naît du terme ; envoyé, il est ignoré. Deux termes qui s'écrivent pareil : `NEGOTIATION_GLOSSARY_SLUG_TAKEN`.",
    path = "/admin/negotiation/glossary",
    tag = "Back-office — savoir",
    operation_id = "admin_negotiation_glossary_creer",
    request_body = Object,
    responses(
        (status = 201, description = "AdminGlossaryEntry", body = Object),
        (status = 403, description = "Sans la permission de publier", body = crate::routes::openapi::ApiErrorBody),
        (status = 409, description = "Ce terme s'écrit déjà ainsi", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Texte, famille, source ou lié invalides", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn creer_terme(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    acteur: Requires<KnowledgePublish>,
    entree: web::Json<AdminGlossaryInput>,
) -> Result<HttpResponse> {
    let ctx = crate::routes::contexte_de(&requete, acteur.person_id);
    let id = service::creer_terme(&state, &ctx, &entree).await?;
    let droits = service::droits(&state, acteur.person_id).await?;
    let locale = crate::routes::locale_de(&requete);
    Ok(HttpResponse::Created().json(service::fiche_terme(&state, &droits, id, &locale).await?))
}

#[utoipa::path(
    get,
    description = "`AdminGlossaryEntry` — la fiche d'un terme, textes non résolus, avec sources et termes liés.",
    path = "/admin/negotiation/glossary/{id}",
    tag = "Back-office — savoir",
    operation_id = "admin_negotiation_glossary_entree",
    params(("id" = Uuid, Path, description = "Identifiant du terme")),
    responses(
        (status = 200, description = "AdminGlossaryEntry", body = Object),
        (status = 403, description = "Ni publier ni vérifier", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Terme inconnu", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn fiche_terme(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    lecteur: LectureSavoir,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let locale = crate::routes::locale_de(&requete);
    let fiche = service::fiche_terme(&state, &lecteur.droits, chemin.into_inner(), &locale).await?;
    Ok(HttpResponse::Ok().json(fiche))
}

#[utoipa::path(
    patch,
    description = "`AdminGlossaryInput` partiel → `AdminGlossaryEntry`. Le `slug` ne se recalcule jamais, même si le terme change. Sources et liés se remplacent en bloc.",
    path = "/admin/negotiation/glossary/{id}",
    tag = "Back-office — savoir",
    operation_id = "admin_negotiation_glossary_modifier",
    params(("id" = Uuid, Path, description = "Identifiant du terme")),
    request_body = Object,
    responses(
        (status = 200, description = "AdminGlossaryEntry", body = Object),
        (status = 403, description = "Sans la permission de publier", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Terme inconnu", body = crate::routes::openapi::ApiErrorBody),
        (status = 409, description = "Ce terme s'écrit déjà ainsi", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Texte, famille, source ou lié invalides", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn modifier_terme(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    acteur: Requires<KnowledgePublish>,
    chemin: web::Path<Uuid>,
    entree: web::Json<AdminGlossaryInput>,
) -> Result<HttpResponse> {
    let id = chemin.into_inner();
    let ctx = crate::routes::contexte_de(&requete, acteur.person_id);
    service::modifier_terme(&state, &ctx, id, &entree).await?;
    rendre_terme(&state, &requete, acteur.person_id, id).await
}

#[utoipa::path(
    delete,
    description = "Supprime un **brouillon jamais publié**. Un terme publié une fois se dépublie.",
    path = "/admin/negotiation/glossary/{id}",
    tag = "Back-office — savoir",
    operation_id = "admin_negotiation_glossary_supprimer",
    params(("id" = Uuid, Path, description = "Identifiant du terme")),
    responses(
        (status = 204, description = "Supprimé"),
        (status = 403, description = "Sans la permission de publier", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Terme inconnu", body = crate::routes::openapi::ApiErrorBody),
        (status = 409, description = "Déjà publié une fois", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn supprimer_terme(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    acteur: Requires<KnowledgePublish>,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let ctx = crate::routes::contexte_de(&requete, acteur.person_id);
    service::supprimer_terme(&state, &ctx, chemin.into_inner()).await?;
    Ok(HttpResponse::NoContent().finish())
}

#[utoipa::path(
    post,
    description = "`AdminGlossaryEntry` — publie le terme.",
    path = "/admin/negotiation/glossary/{id}/publish",
    tag = "Back-office — savoir",
    operation_id = "admin_negotiation_glossary_publier",
    params(("id" = Uuid, Path, description = "Identifiant du terme")),
    responses(
        (status = 200, description = "AdminGlossaryEntry", body = Object),
        (status = 403, description = "Sans la permission de publier", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Terme inconnu", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn publier_terme(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    acteur: Requires<KnowledgePublish>,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let id = chemin.into_inner();
    let ctx = crate::routes::contexte_de(&requete, acteur.person_id);
    service::changer_terme(&state, &ctx, id, Transition::Publier).await?;
    rendre_terme(&state, &requete, acteur.person_id, id).await
}

#[utoipa::path(
    post,
    description = "`AdminGlossaryEntry` — met un terme publié « À revoir » ; il reste servi.",
    path = "/admin/negotiation/glossary/{id}/to-review",
    tag = "Back-office — savoir",
    operation_id = "admin_negotiation_glossary_a_revoir",
    params(("id" = Uuid, Path, description = "Identifiant du terme")),
    responses(
        (status = 200, description = "AdminGlossaryEntry", body = Object),
        (status = 403, description = "Sans la permission de publier", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Terme inconnu", body = crate::routes::openapi::ApiErrorBody),
        (status = 409, description = "Terme jamais publié", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn terme_a_revoir(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    acteur: Requires<KnowledgePublish>,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let id = chemin.into_inner();
    let ctx = crate::routes::contexte_de(&requete, acteur.person_id);
    service::changer_terme(&state, &ctx, id, Transition::ARevoir).await?;
    rendre_terme(&state, &requete, acteur.person_id, id).await
}

#[utoipa::path(
    post,
    description = "`AdminGlossaryEntry` — revient au brouillon ; le téléphone le retire à sa relecture.",
    path = "/admin/negotiation/glossary/{id}/unpublish",
    tag = "Back-office — savoir",
    operation_id = "admin_negotiation_glossary_depublier",
    params(("id" = Uuid, Path, description = "Identifiant du terme")),
    responses(
        (status = 200, description = "AdminGlossaryEntry", body = Object),
        (status = 403, description = "Sans la permission de publier", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Terme inconnu", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn depublier_terme(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    acteur: Requires<KnowledgePublish>,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let id = chemin.into_inner();
    let ctx = crate::routes::contexte_de(&requete, acteur.person_id);
    service::changer_terme(&state, &ctx, id, Transition::Depublier).await?;
    rendre_terme(&state, &requete, acteur.person_id, id).await
}

// ---------------------------------------------------------------------------
// Parcours — chaque écriture rend le parcours entier
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    description = "`AdminPathway` — les groupes et leurs étapes, publiés ou non, la cible de chaque lien nommée et les coches comptées.",
    path = "/admin/negotiation/pathway",
    tag = "Back-office — savoir",
    operation_id = "admin_negotiation_pathway",
    responses(
        (status = 200, description = "AdminPathway", body = Object),
        (status = 401, description = "Aucune session", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Ni publier ni vérifier", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn parcours(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    lecteur: LectureSavoir,
) -> Result<HttpResponse> {
    let locale = crate::routes::locale_de(&requete);
    Ok(HttpResponse::Ok().json(service::parcours(&state, &lecteur.droits, &locale).await?))
}

#[utoipa::path(
    post,
    description = "`AdminPathwayGroupInput` → `AdminPathway` — ajoute un groupe en fin de parcours.",
    path = "/admin/negotiation/pathway/groups",
    tag = "Back-office — savoir",
    operation_id = "admin_negotiation_pathway_groupe_creer",
    request_body = Object,
    responses(
        (status = 201, description = "AdminPathway", body = Object),
        (status = 403, description = "Sans la permission de publier", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Libellé manquant", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn creer_groupe(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    acteur: Requires<KnowledgePublish>,
    entree: web::Json<AdminPathwayGroupInput>,
) -> Result<HttpResponse> {
    let ctx = crate::routes::contexte_de(&requete, acteur.person_id);
    service::creer_groupe(&state, &ctx, &entree).await?;
    let droits = service::droits(&state, acteur.person_id).await?;
    let locale = crate::routes::locale_de(&requete);
    Ok(HttpResponse::Created().json(service::parcours(&state, &droits, &locale).await?))
}

#[utoipa::path(
    patch,
    description = "`AdminPathwayGroupInput` partiel → `AdminPathway` — renomme, publie ou dépublie un groupe.",
    path = "/admin/negotiation/pathway/groups/{id}",
    tag = "Back-office — savoir",
    operation_id = "admin_negotiation_pathway_groupe_modifier",
    params(("id" = Uuid, Path, description = "Identifiant du groupe")),
    request_body = Object,
    responses(
        (status = 200, description = "AdminPathway", body = Object),
        (status = 403, description = "Sans la permission de publier", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Groupe inconnu", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn modifier_groupe(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    acteur: Requires<KnowledgePublish>,
    chemin: web::Path<Uuid>,
    entree: web::Json<AdminPathwayGroupInput>,
) -> Result<HttpResponse> {
    let ctx = crate::routes::contexte_de(&requete, acteur.person_id);
    service::modifier_groupe(&state, &ctx, chemin.into_inner(), &entree).await?;
    rendre_parcours(&state, &requete, acteur.person_id).await
}

#[utoipa::path(
    delete,
    description = "Supprime un groupe vide. S'il porte des étapes : `NEGOTIATION_PATHWAY_GROUP_NOT_EMPTY`.",
    path = "/admin/negotiation/pathway/groups/{id}",
    tag = "Back-office — savoir",
    operation_id = "admin_negotiation_pathway_groupe_supprimer",
    params(("id" = Uuid, Path, description = "Identifiant du groupe")),
    responses(
        (status = 204, description = "Supprimé"),
        (status = 403, description = "Sans la permission de publier", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Groupe inconnu", body = crate::routes::openapi::ApiErrorBody),
        (status = 409, description = "Le groupe porte des étapes", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn supprimer_groupe(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    acteur: Requires<KnowledgePublish>,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let ctx = crate::routes::contexte_de(&requete, acteur.person_id);
    service::supprimer_groupe(&state, &ctx, chemin.into_inner()).await?;
    Ok(HttpResponse::NoContent().finish())
}

#[utoipa::path(
    post,
    description = "`AdminPathwayStepInput` → `AdminPathway` — ajoute une étape en fin de son groupe. Lien incohérent : `NEGOTIATION_PATHWAY_LINK_INVALID`.",
    path = "/admin/negotiation/pathway/steps",
    tag = "Back-office — savoir",
    operation_id = "admin_negotiation_pathway_etape_creer",
    request_body = Object,
    responses(
        (status = 201, description = "AdminPathway", body = Object),
        (status = 403, description = "Sans la permission de publier", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Libellé, groupe ou lien invalides", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn creer_etape(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    acteur: Requires<KnowledgePublish>,
    entree: web::Json<AdminPathwayStepInput>,
) -> Result<HttpResponse> {
    let ctx = crate::routes::contexte_de(&requete, acteur.person_id);
    service::creer_etape(&state, &ctx, &entree).await?;
    let droits = service::droits(&state, acteur.person_id).await?;
    let locale = crate::routes::locale_de(&requete);
    Ok(HttpResponse::Created().json(service::parcours(&state, &droits, &locale).await?))
}

#[utoipa::path(
    patch,
    description = "`AdminPathwayStepInput` partiel → `AdminPathway`. `link: null` retire le lien ; un autre groupe déplace l'étape à sa fin.",
    path = "/admin/negotiation/pathway/steps/{id}",
    tag = "Back-office — savoir",
    operation_id = "admin_negotiation_pathway_etape_modifier",
    params(("id" = Uuid, Path, description = "Identifiant de l'étape")),
    request_body = Object,
    responses(
        (status = 200, description = "AdminPathway", body = Object),
        (status = 403, description = "Sans la permission de publier", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Étape inconnue", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Libellé, groupe ou lien invalides", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn modifier_etape(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    acteur: Requires<KnowledgePublish>,
    chemin: web::Path<Uuid>,
    entree: web::Json<AdminPathwayStepInput>,
) -> Result<HttpResponse> {
    let ctx = crate::routes::contexte_de(&requete, acteur.person_id);
    service::modifier_etape(&state, &ctx, chemin.into_inner(), &entree).await?;
    rendre_parcours(&state, &requete, acteur.person_id).await
}

#[utoipa::path(
    delete,
    description = "Supprime une étape que personne n'a cochée. Cochée, elle se dépublie : `NEGOTIATION_KNOWLEDGE_PUBLISHED_UNDELETABLE`.",
    path = "/admin/negotiation/pathway/steps/{id}",
    tag = "Back-office — savoir",
    operation_id = "admin_negotiation_pathway_etape_supprimer",
    params(("id" = Uuid, Path, description = "Identifiant de l'étape")),
    responses(
        (status = 204, description = "Supprimée"),
        (status = 403, description = "Sans la permission de publier", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Étape inconnue", body = crate::routes::openapi::ApiErrorBody),
        (status = 409, description = "Étape déjà cochée", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn supprimer_etape(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    acteur: Requires<KnowledgePublish>,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let ctx = crate::routes::contexte_de(&requete, acteur.person_id);
    service::supprimer_etape(&state, &ctx, chemin.into_inner()).await?;
    Ok(HttpResponse::NoContent().finish())
}

#[utoipa::path(
    put,
    description = "`AdminPathwayOrderInput` → `AdminPathway` — l'ordre des groupes, et de chaque groupe ses étapes ; une étape citée sous un autre groupe y passe.",
    path = "/admin/negotiation/pathway/order",
    tag = "Back-office — savoir",
    operation_id = "admin_negotiation_pathway_ordonner",
    request_body = Object,
    responses(
        (status = 200, description = "AdminPathway", body = Object),
        (status = 403, description = "Sans la permission de publier", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Groupe ou étape inconnus", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn ordonner(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    acteur: Requires<KnowledgePublish>,
    entree: web::Json<AdminPathwayOrderInput>,
) -> Result<HttpResponse> {
    let ctx = crate::routes::contexte_de(&requete, acteur.person_id);
    service::ordonner(&state, &ctx, &entree).await?;
    rendre_parcours(&state, &requete, acteur.person_id).await
}
