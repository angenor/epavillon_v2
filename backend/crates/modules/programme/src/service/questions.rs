//! Les questions du public : lire, poser, soutenir.
//!
//! Une séance non publiée rend le même 404 qu'une séance inconnue, comme sa
//! page. La longueur d'une question est bornée par la base ; le service traduit
//! son refus, il ne le rejoue pas.

use kernel::context::RequestContext;
use kernel::error::{ApiError, ErrorCode, Result};
use kernel::pg_error;
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::ids::{QuestionId, SessionId};
use crate::repo::questions::{self, SeanceQuestionnable};
use crate::state::ProgrammeState;

/// `{ body }` — le texte de la question, entre 3 et 2000 caractères.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct AskQuestionPayload {
    pub body: String,
}

fn fermee() -> ApiError {
    ApiError::with_message(
        ErrorCode::Conflict,
        "Cette séance ne prend pas de questions.",
    )
}

fn exiger_ouverte(seance: &SeanceQuestionnable) -> Result<()> {
    if !seance.allows_questions || seance.status == "cancelled" {
        return Err(fermee());
    }
    Ok(())
}

async fn seance_publique(
    state: &ProgrammeState,
    session_id: SessionId,
) -> Result<SeanceQuestionnable> {
    questions::seance_publique(state.pool(), session_id)
        .await?
        .ok_or_else(ApiError::not_found)
}

pub async fn lire(
    state: &ProgrammeState,
    session_id: SessionId,
    lecteur: Option<Uuid>,
) -> Result<Vec<serde_json::Value>> {
    seance_publique(state, session_id).await?;
    questions::visibles(state.pool(), session_id, lecteur, None).await
}

pub async fn poser(
    state: &ProgrammeState,
    ctx: &RequestContext,
    session_id: SessionId,
    auteur: Uuid,
    corps: &str,
) -> Result<serde_json::Value> {
    exiger_ouverte(&seance_publique(state, session_id).await?)?;

    let mut tx = state.db().write(ctx).await?;
    let id = questions::poser(&mut tx, session_id, auteur, corps.trim())
        .await
        .map_err(|e| match pg_error::sqlstate(&e).as_deref() {
            Some("23514") => {
                ApiError::validation("Une question compte entre 3 et 2000 caractères.", "body")
            }
            _ => pg_error::translate(&e),
        })?;
    let question = une(&mut tx, session_id, auteur, id).await?;
    tx.commit().await?;

    Ok(question)
}

pub async fn voter(
    state: &ProgrammeState,
    ctx: &RequestContext,
    session_id: SessionId,
    question_id: QuestionId,
    personne: Uuid,
) -> Result<serde_json::Value> {
    exiger_ouverte(&seance_publique(state, session_id).await?)?;

    let mut tx = state.db().write(ctx).await?;
    if !questions::verrouiller_si_visible(&mut tx, session_id, question_id).await? {
        return Err(ApiError::not_found());
    }
    questions::voter(&mut tx, question_id, personne)
        .await
        .map_err(|e| match pg_error::sqlstate(&e).as_deref() {
            Some("23505") => {
                ApiError::with_message(ErrorCode::Conflict, "Vous soutenez déjà cette question.")
            }
            _ => pg_error::translate(&e),
        })?;
    questions::recompter(&mut tx, question_id).await?;
    let question = une(&mut tx, session_id, personne, question_id).await?;
    tx.commit().await?;

    Ok(question)
}

/// Sans effet si la personne n'avait pas voté : retirer ce qui n'existe pas
/// laisse l'état voulu.
pub async fn retirer_le_vote(
    state: &ProgrammeState,
    ctx: &RequestContext,
    session_id: SessionId,
    question_id: QuestionId,
    personne: Uuid,
) -> Result<serde_json::Value> {
    seance_publique(state, session_id).await?;

    let mut tx = state.db().write(ctx).await?;
    if !questions::verrouiller_si_visible(&mut tx, session_id, question_id).await? {
        return Err(ApiError::not_found());
    }
    questions::retirer_le_vote(&mut tx, question_id, personne).await?;
    questions::recompter(&mut tx, question_id).await?;
    let question = une(&mut tx, session_id, personne, question_id).await?;
    tx.commit().await?;

    Ok(question)
}

async fn une(
    conn: &mut sqlx::PgConnection,
    session_id: SessionId,
    lecteur: Uuid,
    question_id: QuestionId,
) -> Result<serde_json::Value> {
    questions::visibles(conn, session_id, Some(lecteur), Some(question_id))
        .await?
        .into_iter()
        .next()
        .ok_or_else(ApiError::not_found)
}
