//! Les routes du savoir : le paquet que le téléphone garde et le compte des
//! lectures, ouverts à tous ; les termes favoris, les retours et les
//! signalements de la personne connectée ; ses questions aux experts, avec
//! l'accès négociateur.

use actix_web::http::header::{HeaderValue, CACHE_CONTROL, ETAG, VARY};
use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use kernel::auth::{Actor, Requires};
use kernel::context::RequestContext;
use kernel::error::Result;
use serde::Deserialize;
use uuid::Uuid;

use crate::domain::permissions::SpaceAccess;
use crate::domain::savoir::lire_since;
use crate::domain::savoir_questions::MyQuestionInput;
use crate::domain::savoir_retours::{FaqFeedbackInput, FaqReportInput};
use crate::service::savoir_favoris;
use crate::service::savoir_lectures;
use crate::service::savoir_paquet as service;
use crate::service::savoir_questions;
use crate::service::savoir_retours;
use crate::state::NegotiationState;

const PUBLIC: &str = "public, no-cache";

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/negotiation/knowledge", web::get().to(paquet))
        .route(
            "/negotiation/faq/{id}/read",
            web::post().to(lire_une_entree),
        )
        .route("/negotiation/faq/{id}/feedback", web::put().to(voter))
        .route("/negotiation/faq/{id}/reports", web::post().to(signaler))
        .route("/negotiation/me/faq-feedback", web::get().to(mes_voix))
        .route("/negotiation/me/questions", web::post().to(poser_une_question))
        .route("/negotiation/me/questions", web::get().to(mes_questions))
        .route("/negotiation/me/glossary-favorites", web::get().to(favoris))
        .route(
            "/negotiation/me/glossary-favorites/{entry_id}",
            web::put().to(poser_un_favori),
        )
        .route(
            "/negotiation/me/glossary-favorites/{entry_id}",
            web::delete().to(retirer_un_favori),
        );
}

#[derive(Debug, Deserialize)]
pub struct Depuis {
    since: Option<String>,
}

#[utoipa::path(
    get,
    description = "`KnowledgeBundle` — la FAQ, le parcours et le lexique publiés (`published` et `to_review`), les deux vocabulaires et « les plus lues », dans la langue demandée. `complete: true`.\n\nAvec `since` (le `served_at` d'une lecture précédente) : les entrées changées depuis `since − 5 min`, et dans `removed` celles qui ne sont plus publiées ; `complete: false`. Parcours, vocabulaires et `most_read` reviennent entiers.\n\n`ETag` calculé par `negotiation.knowledge_fingerprint()` et la langue ; **304** sur `If-None-Match`. `Cache-Control: public, no-cache`.",
    path = "/negotiation/knowledge",
    tag = "Guide Négo — savoir",
    operation_id = "negotiation_knowledge",
    params(("since" = Option<String>, Query, description = "`served_at` d'une lecture précédente, en RFC 3339")),
    responses(
        (status = 200, description = "KnowledgeBundle", body = Object),
        (status = 304, description = "Rien n'a changé depuis l'empreinte présentée"),
        (status = 422, description = "`since` illisible", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn paquet(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    depuis: web::Query<Depuis>,
) -> Result<HttpResponse> {
    let since = depuis.since.as_deref().map(lire_since).transpose()?;
    let locale = crate::routes::locale_de(&requete);

    let empreinte = service::empreinte(&state, &locale).await?;
    if crate::routes::inchange(&requete, &empreinte) {
        return Ok(HttpResponse::NotModified()
            .insert_header((ETAG, empreinte))
            .insert_header((CACHE_CONTROL, PUBLIC))
            .insert_header((VARY, HeaderValue::from_static("Accept-Language")))
            .finish());
    }

    let (paquet, empreinte) = service::paquet(&state, &locale, since).await?;
    Ok(HttpResponse::Ok()
        .insert_header((ETAG, empreinte))
        .insert_header((CACHE_CONTROL, PUBLIC))
        .insert_header((VARY, HeaderValue::from_static("Accept-Language")))
        .json(paquet))
}

#[utoipa::path(
    get,
    description = "`MyGlossaryFavorites` — les identifiants des termes favoris de la personne connectée, parmi les entrées servies (`published` et `to_review`). `ETag` et **304**.",
    path = "/negotiation/me/glossary-favorites",
    tag = "Guide Négo — savoir",
    operation_id = "negotiation_mes_termes_favoris",
    responses(
        (status = 200, description = "MyGlossaryFavorites", body = Object),
        (status = 304, description = "Rien n'a changé depuis l'empreinte présentée"),
        (status = 401, description = "Aucune session", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn favoris(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    acteur: Actor,
) -> Result<HttpResponse> {
    let (favoris, empreinte) = savoir_favoris::favoris(&state, acteur.0).await?;
    if crate::routes::inchange(&requete, &empreinte) {
        return Ok(HttpResponse::NotModified()
            .insert_header((ETAG, empreinte))
            .insert_header(crate::routes::PERSONNEL)
            .finish());
    }
    Ok(HttpResponse::Ok()
        .insert_header((ETAG, empreinte))
        .insert_header(crate::routes::PERSONNEL)
        .json(favoris))
}

#[utoipa::path(
    put,
    description = "Pose un terme favori. **Idempotent**. Entrée inconnue ou en brouillon : **404**.",
    path = "/negotiation/me/glossary-favorites/{entry_id}",
    tag = "Guide Négo — savoir",
    operation_id = "negotiation_poser_un_terme_favori",
    params(("entry_id" = Uuid, Path, description = "Identifiant de l'entrée du lexique")),
    responses(
        (status = 204, description = "Posé"),
        (status = 401, description = "Aucune session", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Entrée inconnue ou en brouillon", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn poser_un_favori(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    acteur: Actor,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let ctx = crate::routes::contexte_de(&requete, acteur.0);
    savoir_favoris::poser_un_favori(&state, &ctx, acteur.0, chemin.into_inner()).await?;
    Ok(HttpResponse::NoContent().finish())
}

#[utoipa::path(
    delete,
    description = "Retire un terme favori. **Idempotent**, même s'il n'existe pas.",
    path = "/negotiation/me/glossary-favorites/{entry_id}",
    tag = "Guide Négo — savoir",
    operation_id = "negotiation_retirer_un_terme_favori",
    params(("entry_id" = Uuid, Path, description = "Identifiant de l'entrée du lexique")),
    responses(
        (status = 204, description = "Retiré"),
        (status = 401, description = "Aucune session", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn retirer_un_favori(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    acteur: Actor,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let ctx = crate::routes::contexte_de(&requete, acteur.0);
    savoir_favoris::retirer_un_favori(&state, &ctx, acteur.0, chemin.into_inner()).await?;
    Ok(HttpResponse::NoContent().finish())
}

#[utoipa::path(
    post,
    description = "Compte une lecture de l'entrée de FAQ, pour le jour de Paris, sans rien retenir de qui lit. Le téléphone l'envoie une fois par entrée et par jour. Entrée inconnue ou en brouillon : **204** quand même, rien n'est compté.",
    path = "/negotiation/faq/{id}/read",
    tag = "Guide Négo — savoir",
    operation_id = "negotiation_lire_une_entree_de_faq",
    params(("id" = Uuid, Path, description = "Identifiant de l'entrée de FAQ")),
    responses((status = 204, description = "Lecture comptée, ou ignorée")),
)]
pub(crate) async fn lire_une_entree(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let ctx = requete
        .extensions()
        .get::<RequestContext>()
        .cloned()
        .unwrap_or_else(|| {
            RequestContext::new(
                RequestContext::generated_request_id(),
                crate::routes::locale_de(&requete),
            )
        });
    savoir_lectures::compter_une_lecture(&state, &ctx, chemin.into_inner()).await?;
    Ok(HttpResponse::NoContent().finish())
}

#[utoipa::path(
    put,
    description = "`FaqFeedbackInput` → `FaqFeedback` — « Cette réponse vous a-t-elle aidée ? ». Une voix par personne et par entrée : la dernière écrase la précédente. `missing_reason` (`too_vague`, `off_topic`, `outdated`) après « Non » seulement ; `outdated` ouvre aussi, une fois par personne et par entrée, un signalement `from_feedback` dans la file des experts. L'entrée n'est jamais modifiée.",
    path = "/negotiation/faq/{id}/feedback",
    tag = "Guide Négo — savoir",
    operation_id = "negotiation_retour_sur_une_entree_de_faq",
    params(("id" = Uuid, Path, description = "Identifiant de l'entrée de FAQ")),
    request_body = Object,
    responses(
        (status = 200, description = "FaqFeedback", body = Object),
        (status = 401, description = "Aucune session", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Entrée inconnue ou en brouillon", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Motif inconnu, ou motif après « Oui »", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn voter(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    acteur: Actor,
    chemin: web::Path<Uuid>,
    entree: web::Json<FaqFeedbackInput>,
) -> Result<HttpResponse> {
    let ctx = crate::routes::contexte_de(&requete, acteur.0);
    let voix = savoir_retours::voter(&state, &ctx, acteur.0, chemin.into_inner(), &entree).await?;
    Ok(HttpResponse::Ok().json(voix))
}

#[utoipa::path(
    post,
    description = "`FaqReportInput` → `FaqReportReceipt` — « Dépassé ou faux » : un à trois motifs (`rule_changed`, `wrong`, `source_mismatch`), une précision de 600 caractères au plus. Rejoué avec le même `client_ref` : **200** et le même reçu. Vingt par personne et par jour de Paris, au-delà **429**. Rejoint la file des experts, anonyme ; l'entrée n'est jamais modifiée.",
    path = "/negotiation/faq/{id}/reports",
    tag = "Guide Négo — savoir",
    operation_id = "negotiation_signaler_une_entree_de_faq",
    params(("id" = Uuid, Path, description = "Identifiant de l'entrée de FAQ")),
    request_body = Object,
    responses(
        (status = 201, description = "FaqReportReceipt", body = Object),
        (status = 200, description = "Rejeu : le reçu d'origine", body = Object),
        (status = 401, description = "Aucune session", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Entrée inconnue ou en brouillon", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Aucun motif, motif inconnu, précision trop longue", body = crate::routes::openapi::ApiErrorBody),
        (status = 429, description = "Plafond du jour atteint", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn signaler(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    acteur: Actor,
    chemin: web::Path<Uuid>,
    entree: web::Json<FaqReportInput>,
) -> Result<HttpResponse> {
    let ctx = crate::routes::contexte_de(&requete, acteur.0);
    let (recu, nouveau) =
        savoir_retours::signaler(&state, &ctx, acteur.0, chemin.into_inner(), &entree).await?;
    Ok(if nouveau {
        HttpResponse::Created().json(recu)
    } else {
        HttpResponse::Ok().json(recu)
    })
}

#[utoipa::path(
    get,
    description = "`MyFaqFeedback` — les voix de la personne connectée sur les entrées servies, pour réafficher « Merci. ». `ETag` et **304**.",
    path = "/negotiation/me/faq-feedback",
    tag = "Guide Négo — savoir",
    operation_id = "negotiation_mes_retours_sur_la_faq",
    responses(
        (status = 200, description = "MyFaqFeedback", body = Object),
        (status = 304, description = "Rien n'a changé depuis l'empreinte présentée"),
        (status = 401, description = "Aucune session", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn mes_voix(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    acteur: Actor,
) -> Result<HttpResponse> {
    let (voix, empreinte) = savoir_retours::mes_voix(&state, acteur.0).await?;
    if crate::routes::inchange(&requete, &empreinte) {
        return Ok(HttpResponse::NotModified()
            .insert_header((ETAG, empreinte))
            .insert_header(crate::routes::PERSONNEL)
            .finish());
    }
    Ok(HttpResponse::Ok()
        .insert_header((ETAG, empreinte))
        .insert_header(crate::routes::PERSONNEL)
        .json(voix))
}

#[utoipa::path(
    post,
    description = "`MyQuestionInput` → `MyQuestion` — pose une question aux experts de l'IFDD : une thématique de négociation (`theme_code`), 600 caractères au plus, et le consentement à rejoindre la FAQ, anonymisée. Réservé à l'accès négociateur : sans lui, **403**. Rejouée avec le même `client_ref` : **200** et la même question.",
    path = "/negotiation/me/questions",
    tag = "Guide Négo — savoir",
    operation_id = "negotiation_poser_une_question",
    request_body = Object,
    responses(
        (status = 201, description = "MyQuestion", body = Object),
        (status = 200, description = "Rejeu : la question d'origine", body = Object),
        (status = 400, description = "Thématique inconnue", body = crate::routes::openapi::ApiErrorBody),
        (status = 401, description = "Aucune session", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Sans l'accès négociateur", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Question vide ou trop longue", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn poser_une_question(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    acces: Requires<SpaceAccess>,
    entree: web::Json<MyQuestionInput>,
) -> Result<HttpResponse> {
    let locale = crate::routes::locale_de(&requete);
    let ctx = crate::routes::contexte_de(&requete, acces.person_id);
    let (question, nouvelle) =
        savoir_questions::poser(&state, &ctx, acces.person_id, &entree, &locale).await?;
    Ok(if nouvelle {
        HttpResponse::Created().json(question)
    } else {
        HttpResponse::Ok().json(question)
    })
}

#[utoipa::path(
    get,
    description = "`MyQuestionList` — les questions de la personne connectée, la plus récente d'abord, avec leur état et, une fois répondues, la réponse, son auteur et sa date. Réservé à l'accès négociateur. `ETag` et **304**.",
    path = "/negotiation/me/questions",
    tag = "Guide Négo — savoir",
    operation_id = "negotiation_mes_questions",
    responses(
        (status = 200, description = "MyQuestionList", body = Object),
        (status = 304, description = "Rien n'a changé depuis l'empreinte présentée"),
        (status = 401, description = "Aucune session", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Sans l'accès négociateur", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn mes_questions(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    acces: Requires<SpaceAccess>,
) -> Result<HttpResponse> {
    let locale = crate::routes::locale_de(&requete);
    let (questions, empreinte) =
        savoir_questions::miennes(&state, acces.person_id, &locale).await?;
    if crate::routes::inchange(&requete, &empreinte) {
        return Ok(HttpResponse::NotModified()
            .insert_header((ETAG, empreinte))
            .insert_header(crate::routes::PERSONNEL)
            .finish());
    }
    Ok(HttpResponse::Ok()
        .insert_header((ETAG, empreinte))
        .insert_header(crate::routes::PERSONNEL)
        .json(questions))
}
