//! Les questions du public, sous `/sessions/{id}/questions`. La lecture est
//! publique ; poser et soutenir exigent une session.

use actix_web::{web, HttpRequest, HttpResponse};
use kernel::auth::Actor;
use kernel::error::Result;
use uuid::Uuid;

use crate::domain::ids::{QuestionId, SessionId};
use crate::routes::{contexte_de, session_facultative};
use crate::service::questions::{self, AskQuestionPayload};
use crate::state::ProgrammeState;

pub fn chemins_de_seance(cfg: &mut web::ServiceConfig) {
    cfg.route("/{id}/questions", web::get().to(lire))
        .route("/{id}/questions", web::post().to(poser))
        .route("/{id}/questions/{question_id}/vote", web::post().to(voter))
        .route(
            "/{id}/questions/{question_id}/vote",
            web::delete().to(retirer_le_vote),
        );
}

/// Les questions visibles d'une séance publiée.
#[utoipa::path(
    get,
    description = "`PublicSessionQuestion[]` — les questions **visibles** d'une séance publiée, les plus soutenues d'abord puis les plus anciennes, chacune avec son nombre de soutiens et ses réponses (`PublicSessionQuestionAnswer[]`). **Sans session** : `has_voted` et `is_mine` valent alors faux. L'auteur n'est jamais exposé, ni les intervenants visés, ni la modération. Une séance non publiée rend le même 404 qu'une séance inconnue.\n\n`ETag` sur le corps ; **304** sur `If-None-Match` ; `Cache-Control: private, no-cache`.",
    path = "/sessions/{id}/questions",
    tag = "Programmation publique",
    operation_id = "questions_du_public_lire",
    params(("id" = Uuid, Path, description = "Identifiant de la séance")),
    responses(
        (status = 200, description = "PublicSessionQuestion[]", body = Object),
        (status = 304, description = "Rien n'a changé depuis l'empreinte présentée"),
        (status = 404, description = "Séance inconnue **ou non publiée** — indiscernables", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn lire(
    requete: HttpRequest,
    state: web::Data<ProgrammeState>,
    id: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let lignes = questions::lire(
        &state,
        SessionId(id.into_inner()),
        session_facultative(&requete),
    )
    .await?;

    crate::routes::json_revalide(&requete, &lignes, "private, no-cache")
}

/// Poser une question.
#[utoipa::path(
    post,
    description = "`AskQuestionPayload` → `PublicSessionQuestion`, en **201**. La question est visible dès sa pose ; l'équipe peut la masquer ensuite. Refusée en **409** quand la séance ne prend pas de questions ou qu'elle est annulée.",
    path = "/sessions/{id}/questions",
    tag = "Programmation publique",
    operation_id = "questions_du_public_poser",
    params(("id" = Uuid, Path, description = "Identifiant de la séance")),
    request_body = AskQuestionPayload,
    responses(
        (status = 201, description = "PublicSessionQuestion", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Séance inconnue **ou non publiée**", body = crate::routes::openapi::ApiErrorBody),
        (status = 409, description = "La séance ne prend pas de questions", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Texte hors bornes : entre 3 et 2000 caractères", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn poser(
    requete: HttpRequest,
    state: web::Data<ProgrammeState>,
    acteur: Actor,
    id: web::Path<Uuid>,
    corps: web::Json<AskQuestionPayload>,
) -> Result<HttpResponse> {
    let ctx = contexte_de(&requete, acteur.0);
    let question = questions::poser(
        &state,
        &ctx,
        SessionId(id.into_inner()),
        acteur.0,
        &corps.body,
    )
    .await?;

    Ok(HttpResponse::Created().json(question))
}

/// Soutenir une question.
#[utoipa::path(
    post,
    description = "`PublicSessionQuestion` — la question soutenue, décompte à jour. **Un soutien par personne** : un second rend **409**.",
    path = "/sessions/{id}/questions/{question_id}/vote",
    tag = "Programmation publique",
    operation_id = "questions_du_public_voter",
    params(
        ("id" = Uuid, Path, description = "Identifiant de la séance"),
        ("question_id" = Uuid, Path, description = "Identifiant de la question"),
    ),
    responses(
        (status = 200, description = "PublicSessionQuestion", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Séance non publiée, ou question inconnue, masquée ou d'une autre séance", body = crate::routes::openapi::ApiErrorBody),
        (status = 409, description = "Déjà soutenue, ou séance fermée aux questions", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn voter(
    requete: HttpRequest,
    state: web::Data<ProgrammeState>,
    acteur: Actor,
    chemin: web::Path<(Uuid, Uuid)>,
) -> Result<HttpResponse> {
    let (seance, question) = chemin.into_inner();
    let ctx = contexte_de(&requete, acteur.0);
    let question = questions::voter(
        &state,
        &ctx,
        SessionId(seance),
        QuestionId(question),
        acteur.0,
    )
    .await?;

    Ok(HttpResponse::Ok().json(question))
}

/// Retirer son soutien.
#[utoipa::path(
    delete,
    description = "`PublicSessionQuestion` — la question, décompte à jour. **Sans effet** si la personne ne la soutenait pas : l'état voulu est atteint.",
    path = "/sessions/{id}/questions/{question_id}/vote",
    tag = "Programmation publique",
    operation_id = "questions_du_public_retirer_le_vote",
    params(
        ("id" = Uuid, Path, description = "Identifiant de la séance"),
        ("question_id" = Uuid, Path, description = "Identifiant de la question"),
    ),
    responses(
        (status = 200, description = "PublicSessionQuestion", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Séance non publiée, ou question inconnue, masquée ou d'une autre séance", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn retirer_le_vote(
    requete: HttpRequest,
    state: web::Data<ProgrammeState>,
    acteur: Actor,
    chemin: web::Path<(Uuid, Uuid)>,
) -> Result<HttpResponse> {
    let (seance, question) = chemin.into_inner();
    let ctx = contexte_de(&requete, acteur.0);
    let question = questions::retirer_le_vote(
        &state,
        &ctx,
        SessionId(seance),
        QuestionId(question),
        acteur.0,
    )
    .await?;

    Ok(HttpResponse::Ok().json(question))
}
