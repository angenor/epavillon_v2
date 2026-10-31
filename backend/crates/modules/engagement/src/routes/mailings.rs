//! Les envois de courriels du back-office.

use actix_web::{web, HttpRequest, HttpResponse};
use kernel::auth::Actor;
use kernel::error::{ApiError, Result};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::mailing::{CreateMailingPayload, CreateProposalMailingPayload, MailingPayload};
use crate::routes::contexte_de;
use crate::service::{mailing_dossier, mailings};
use crate::state::EngagementState;

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/admin/mailings", web::get().to(lister))
        .route("/admin/mailings", web::post().to(creer))
        // Avant `{id}` : sinon « address-suggestions » serait lu comme un identifiant.
        .route(
            "/admin/mailings/address-suggestions",
            web::get().to(suggestions),
        )
        .route("/admin/mailings/for-proposal", web::post().to(pour_dossier))
        .route("/admin/mailings/{id}", web::get().to(detail))
        .route("/admin/mailings/{id}", web::put().to(modifier))
        .route("/admin/mailings/{id}", web::delete().to(supprimer))
        .route("/admin/mailings/{id}/audience", web::get().to(audience))
        .route("/admin/mailings/{id}/preview", web::get().to(apercu))
        .route("/admin/mailings/{id}/test", web::post().to(essai))
        .route("/admin/mailings/{id}/send", web::post().to(envoyer))
        .route("/admin/mailings/{id}/cancel", web::post().to(annuler))
        .route("/admin/mailings/{id}/retry", web::post().to(relancer))
        .route("/admin/mailings/{id}/duplicate", web::post().to(dupliquer));
}

#[derive(Debug, Deserialize)]
pub struct ListeQuery {
    pub event_id: Option<Uuid>,
    pub proposal_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
struct Suppression {
    deleted: bool,
}

/// L'historique d'une édition, ou celui d'un dossier.
#[utoipa::path(
    get,
    description = "`MailingListScreen` : les envois de l'édition, du plus récent au plus ancien, avec leurs compteurs par état de destinataire, le destinataire principal, le nombre d'adresses en copie, le texte type choisi, la copie de l'équipe proposée par défaut, les variables offertes et le fuseau de l'édition. `proposal_id` (01/11) réduit la liste aux courriels écrits depuis ce dossier, sur son édition ; `event_id` est alors facultatif. L'un des deux est exigé.\n\nGardé par `engagement.mailing.send` **sur l'édition** (celle du dossier), et borné au périmètre d'administration : hors périmètre, l'édition ou le dossier n'existent pas (404) ; dans le périmètre sans la permission, 403.",
    path = "/admin/mailings",
    tag = "Back-office : envois de courriels",
    operation_id = "engagement_envois_lister",
    params(
        ("event_id" = Option<Uuid>, Query, description = "L'édition"),
        ("proposal_id" = Option<Uuid>, Query, description = "Le dossier : ses courriels seulement"),
    ),
    responses(
        (status = 200, description = "MailingListScreen", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente sur l'édition", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Édition ou dossier inexistants, ou hors périmètre", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "VALIDATION_FAILED", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn lister(
    state: web::Data<EngagementState>,
    Actor(acteur): Actor,
    requete: web::Query<ListeQuery>,
) -> Result<HttpResponse> {
    let ecran = match (requete.proposal_id, requete.event_id) {
        (Some(dossier), edition) => {
            mailing_dossier::ecran(&state, acteur, dossier, edition).await?
        }
        (None, Some(edition)) => mailings::ecran(&state, acteur, edition).await?,
        (None, None) => {
            return Err(ApiError::validation(
                "Précisez l'édition ou le dossier.",
                "event_id",
            ))
        }
    };
    Ok(HttpResponse::Ok().json(ecran))
}

/// Le brouillon d'un dossier (01/11).
#[utoipa::path(
    post,
    description = "`CreateProposalMailingPayload` → `Mailing` : un brouillon rattaché au dossier, dans son édition. Destinataire : l'adresse du déposant ; copie : celle de l'équipe proposée par défaut, la personne qui écrit, et la personne à contacter du dossier si c'en est une autre ; objet : le titre de l'activité. Le rattachement ouvre les variables `{{activite}}`, `{{date}}`, `{{heures}}` et `{{salle}}`, lues sur le dossier et sa séance retenue (`Mailing.proposal`).\n\nGardé par `engagement.mailing.send` **sur l'édition du dossier**, et borné au périmètre d'administration : dossier inexistant, supprimé ou hors périmètre, 404 ; dans le périmètre sans la permission, 403.",
    path = "/admin/mailings/for-proposal",
    tag = "Back-office : envois de courriels",
    operation_id = "engagement_envois_creer_pour_dossier",
    request_body = CreateProposalMailingPayload,
    responses(
        (status = 200, description = "Mailing", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente sur l'édition", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Dossier inexistant ou hors périmètre", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn pour_dossier(
    state: web::Data<EngagementState>,
    Actor(acteur): Actor,
    requete: HttpRequest,
    payload: web::Json<CreateProposalMailingPayload>,
) -> Result<HttpResponse> {
    let ctx = contexte_de(&requete, acteur);
    Ok(HttpResponse::Ok().json(mailing_dossier::creer(&state, &ctx, acteur, &payload).await?))
}

/// Créer un brouillon.
#[utoipa::path(
    post,
    description = "`Mailing` : un brouillon. Sans `cc`, la copie de l'équipe est proposée : les adresses du réglage `engagement.mailing.team_cc` et celle de la personne qui écrit. Le corps est **assaini** (liste blanche des courriels) ; une variable autre que `{{prenom}}`, `{{nom}}`, `{{organisation}}`, `{{email}}`, `{{edition}}` est refusée sur `body_html`. Une adresse invalide est refusée sur son champ, en la citant.",
    path = "/admin/mailings",
    tag = "Back-office : envois de courriels",
    operation_id = "engagement_envois_creer",
    request_body = CreateMailingPayload,
    responses(
        (status = 200, description = "Mailing", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente sur l'édition", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Édition inexistante ou hors périmètre", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "VALIDATION_FAILED", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn creer(
    state: web::Data<EngagementState>,
    Actor(acteur): Actor,
    requete: HttpRequest,
    payload: web::Json<CreateMailingPayload>,
) -> Result<HttpResponse> {
    let ctx = contexte_de(&requete, acteur);
    Ok(HttpResponse::Ok().json(mailings::creer(&state, &ctx, acteur, &payload).await?))
}

/// Un envoi.
#[utoipa::path(
    get,
    description = "`Mailing` : le contenu, les critères, les copies, les pièces jointes (rattachements `attachment` du module media, poids cumulé et plafond), et pour un envoi parti ses destinataires figés avec l'état de chacun (`queued`, `sending`, `sent`, `failed`, `suppressed`, `cancelled`) et l'erreur rendue par le serveur.",
    path = "/admin/mailings/{id}",
    tag = "Back-office : envois de courriels",
    operation_id = "engagement_envois_detail",
    params(("id" = Uuid, Path, description = "L'envoi")),
    responses(
        (status = 200, description = "Mailing", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Envoi inexistant ou hors droit", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn detail(
    state: web::Data<EngagementState>,
    Actor(acteur): Actor,
    id: web::Path<Uuid>,
) -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(mailings::detail(&state, acteur, id.into_inner()).await?))
}

/// Modifier un brouillon.
#[utoipa::path(
    put,
    description = "`Mailing` : écriture partielle, un champ absent reste tel quel. Un envoi parti ne se modifie plus (409, `ENGAGEMENT_MAILING_NOT_DRAFT`) : on le duplique.",
    path = "/admin/mailings/{id}",
    tag = "Back-office : envois de courriels",
    operation_id = "engagement_envois_modifier",
    params(("id" = Uuid, Path, description = "L'envoi")),
    request_body = MailingPayload,
    responses(
        (status = 200, description = "Mailing", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Envoi inexistant ou hors droit", body = crate::routes::openapi::ApiErrorBody),
        (status = 409, description = "ENGAGEMENT_MAILING_NOT_DRAFT", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "VALIDATION_FAILED", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn modifier(
    state: web::Data<EngagementState>,
    Actor(acteur): Actor,
    requete: HttpRequest,
    id: web::Path<Uuid>,
    payload: web::Json<MailingPayload>,
) -> Result<HttpResponse> {
    let ctx = contexte_de(&requete, acteur);
    Ok(HttpResponse::Ok()
        .json(mailings::modifier(&state, &ctx, acteur, id.into_inner(), &payload).await?))
}

/// Supprimer un envoi.
#[utoipa::path(
    delete,
    description = "`{ deleted }` : un brouillon, un envoi terminé, arrêté ou en échec disparaît, avec ses destinataires ; ses pièces jointes sont détachées (les fichiers restent, purgés plus tard s'ils ne servent plus). Un envoi en cours se refuse (409, `ENGAGEMENT_MAILING_IN_PROGRESS`) : on l'arrête d'abord.",
    path = "/admin/mailings/{id}",
    tag = "Back-office : envois de courriels",
    operation_id = "engagement_envois_supprimer",
    params(("id" = Uuid, Path, description = "L'envoi")),
    responses(
        (status = 200, description = "{ deleted }", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Envoi inexistant ou hors droit", body = crate::routes::openapi::ApiErrorBody),
        (status = 409, description = "ENGAGEMENT_MAILING_IN_PROGRESS", body = crate::routes::openapi::ApiErrorBody),
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
    mailings::supprimer(&state, &ctx, acteur, id.into_inner()).await?;
    Ok(HttpResponse::Ok().json(Suppression { deleted: true }))
}

#[derive(Debug, Deserialize)]
pub struct SuggestionsQuery {
    pub event_id: Uuid,
    #[serde(default)]
    pub q: String,
}

/// Compléter une adresse.
#[utoipa::path(
    get,
    description = "`MailingAddressSuggestion[]` : au plus huit personnes actives de la plateforme dont le prénom, le nom ou l'adresse contiennent chaque mot saisi, sans égard aux accents ni à la casse ; celles qui commencent par le premier mot d'abord. Nom et adresse seulement. Moins de deux caractères : liste vide.\n\nGardé par `engagement.mailing.send` **sur l'édition**, et borné au périmètre d'administration : hors périmètre, 404 ; dans le périmètre sans la permission, 403.",
    path = "/admin/mailings/address-suggestions",
    tag = "Back-office : envois de courriels",
    operation_id = "engagement_envois_suggestions",
    params(
        ("event_id" = Uuid, Query, description = "L'édition de l'envoi"),
        ("q" = String, Query, description = "La saisie"),
    ),
    responses(
        (status = 200, description = "MailingAddressSuggestion[]", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente sur l'édition", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Édition inexistante ou hors périmètre", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn suggestions(
    state: web::Data<EngagementState>,
    Actor(acteur): Actor,
    requete: web::Query<SuggestionsQuery>,
) -> Result<HttpResponse> {
    Ok(HttpResponse::Ok()
        .json(mailings::suggestions(&state, acteur, requete.event_id, &requete.q).await?))
}

/// La liste avant envoi.
#[utoipa::path(
    get,
    description = "`MailingAudience` : les adresses telles qu'elles partiraient maintenant, une par adresse (sans égard à la casse), avec les critères qui les ont produites ; le décompte, les adresses sur la liste de suppression (signalées, jamais sollicitées), celles sans prénom connu, et le nombre de messages remis au serveur d'envoi, copies comprises.",
    path = "/admin/mailings/{id}/audience",
    tag = "Back-office : envois de courriels",
    operation_id = "engagement_envois_destinataires",
    params(("id" = Uuid, Path, description = "L'envoi")),
    responses(
        (status = 200, description = "MailingAudience", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Envoi inexistant ou hors droit", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn audience(
    state: web::Data<EngagementState>,
    Actor(acteur): Actor,
    id: web::Path<Uuid>,
) -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(mailings::audience(&state, acteur, id.into_inner()).await?))
}

#[derive(Debug, Deserialize)]
pub struct ApercuQuery {
    pub email: Option<String>,
}

/// L'aperçu, gabarit compris.
#[utoipa::path(
    get,
    description = "`MailingPreview` : le courriel tel qu'un destinataire le recevra, rendu par la même fonction que le worker (gabarit, variables et valeurs de secours, boutons). `email` choisit le destinataire ; absent, le premier joignable. `has_account` faux : une personne sans compte, qui reçoit les valeurs de secours. N'envoie rien.",
    path = "/admin/mailings/{id}/preview",
    tag = "Back-office : envois de courriels",
    operation_id = "engagement_envois_apercu",
    params(
        ("id" = Uuid, Path, description = "L'envoi"),
        ("email" = Option<String>, Query, description = "Le destinataire vu"),
    ),
    responses(
        (status = 200, description = "MailingPreview", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Envoi inexistant ou hors droit", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn apercu(
    state: web::Data<EngagementState>,
    Actor(acteur): Actor,
    id: web::Path<Uuid>,
    requete: web::Query<ApercuQuery>,
) -> Result<HttpResponse> {
    Ok(HttpResponse::Ok()
        .json(mailings::apercu(&state, acteur, id.into_inner(), requete.email.as_deref()).await?))
}

/// L'envoi d'essai.
#[utoipa::path(
    post,
    description = "`MailingTestResult` : un essai part, par la file, à l'adresse de la personne connectée, avec ses propres valeurs de variables, sans copie ni copie cachée, objet préfixé « [Essai] ». Il ne figure pas dans l'historique.",
    path = "/admin/mailings/{id}/test",
    tag = "Back-office : envois de courriels",
    operation_id = "engagement_envois_essai",
    params(("id" = Uuid, Path, description = "L'envoi")),
    responses(
        (status = 200, description = "MailingTestResult", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Envoi inexistant ou hors droit", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "VALIDATION_FAILED", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn essai(
    state: web::Data<EngagementState>,
    Actor(acteur): Actor,
    requete: HttpRequest,
    id: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let ctx = contexte_de(&requete, acteur);
    Ok(HttpResponse::Ok().json(mailings::essai(&state, &ctx, acteur, id.into_inner()).await?))
}

/// Envoyer.
#[utoipa::path(
    post,
    description = "`Mailing` : fige les destinataires (la liste qui part est celle qu'on a vue) et met l'envoi en file. Le worker l'expédie par lots de dix, enchaînés sans attente (aucune limite d'envoi) ; **un message par destinataire**, chacun portant la copie, la copie cachée et l'adresse de réponse. Aucun destinataire joignable : 422 `ENGAGEMENT_MAILING_EMPTY`, le brouillon reste un brouillon. Une pièce jointe pas encore prête (analyse en cours, ou refusée) : 422 sur `attachments`. Chaque message porte toutes les pièces jointes.",
    path = "/admin/mailings/{id}/send",
    tag = "Back-office : envois de courriels",
    operation_id = "engagement_envois_envoyer",
    params(("id" = Uuid, Path, description = "L'envoi")),
    responses(
        (status = 200, description = "Mailing", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Envoi inexistant ou hors droit", body = crate::routes::openapi::ApiErrorBody),
        (status = 409, description = "ENGAGEMENT_MAILING_NOT_DRAFT", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "ENGAGEMENT_MAILING_EMPTY · VALIDATION_FAILED", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn envoyer(
    state: web::Data<EngagementState>,
    Actor(acteur): Actor,
    requete: HttpRequest,
    id: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let ctx = contexte_de(&requete, acteur);
    Ok(HttpResponse::Ok().json(mailings::envoyer(&state, &ctx, acteur, id.into_inner()).await?))
}

/// Arrêter un envoi en cours.
#[utoipa::path(
    post,
    description = "`Mailing` : un envoi en cours ou en échec s'arrête ; ce qui est parti reste parti, les destinataires encore en file passent à `cancelled`.",
    path = "/admin/mailings/{id}/cancel",
    tag = "Back-office : envois de courriels",
    operation_id = "engagement_envois_arreter",
    params(("id" = Uuid, Path, description = "L'envoi")),
    responses(
        (status = 200, description = "Mailing", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Envoi inexistant ou hors droit", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "VALIDATION_FAILED", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn annuler(
    state: web::Data<EngagementState>,
    Actor(acteur): Actor,
    requete: HttpRequest,
    id: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let ctx = contexte_de(&requete, acteur);
    Ok(HttpResponse::Ok().json(mailings::annuler(&state, &ctx, acteur, id.into_inner()).await?))
}

/// Dupliquer.
#[utoipa::path(
    post,
    description = "`Mailing` : un nouveau brouillon qui reprend objet, corps, critères, adresses et copies. Les critères seront réévalués à son envoi.",
    path = "/admin/mailings/{id}/duplicate",
    tag = "Back-office : envois de courriels",
    operation_id = "engagement_envois_dupliquer",
    params(("id" = Uuid, Path, description = "L'envoi à reprendre")),
    responses(
        (status = 200, description = "Mailing", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Envoi inexistant ou hors droit", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn dupliquer(
    state: web::Data<EngagementState>,
    Actor(acteur): Actor,
    requete: HttpRequest,
    id: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let ctx = contexte_de(&requete, acteur);
    Ok(HttpResponse::Ok().json(mailings::dupliquer(&state, &ctx, acteur, id.into_inner()).await?))
}

/// Relancer un envoi en échec.
#[utoipa::path(
    post,
    description = "`Mailing` : un envoi `failed` (serveur de courriel injoignable, travail d'envoi mort) repart pour ses **seuls destinataires encore en file** ; ceux qui sont partis, en échec ou écartés ne reçoivent rien de plus. Le motif du dernier échec reste lisible (`failure_reason`, `failed_at`), `relaunch_count` compte les relances. Refusé (422) si l'envoi n'est pas en échec ou que plus personne n'attend.",
    path = "/admin/mailings/{id}/retry",
    tag = "Back-office : envois de courriels",
    operation_id = "engagement_envois_relancer",
    params(("id" = Uuid, Path, description = "L'envoi")),
    responses(
        (status = 200, description = "Mailing", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Envoi inexistant ou hors droit", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "VALIDATION_FAILED", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn relancer(
    state: web::Data<EngagementState>,
    Actor(acteur): Actor,
    requete: HttpRequest,
    id: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let ctx = contexte_de(&requete, acteur);
    Ok(HttpResponse::Ok().json(mailings::relancer(&state, &ctx, acteur, id.into_inner()).await?))
}
