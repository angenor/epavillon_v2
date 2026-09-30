//! Les questions du public sur une séance publiée.
//!
//! Composées champ par champ, jamais `to_jsonb` : ni l'auteur, ni les
//! intervenants visés, ni la modération ne sortent. Le public sait seulement si
//! une question est la sienne et s'il l'a soutenue.
//!
//! **Le décompte se lit sur les votes**, pas sur `upvotes` : aucun déclencheur
//! ne tient la colonne, et un vote effacé avec son auteur la ferait dériver.
//! Le service la recalcule à chaque vote, pour l'index de tri.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use sqlx::PgExecutor;
use uuid::Uuid;

use crate::domain::ids::{QuestionId, SessionId};

/// Ce qui décide si la séance prend des questions. Nulle quand la séance n'est
/// pas publique : la vue publique fait foi, pas un second filtre écrit ici.
pub struct SeanceQuestionnable {
    pub allows_questions: bool,
    pub status: String,
}

pub async fn seance_publique<'e>(
    executor: impl PgExecutor<'e>,
    session_id: SessionId,
) -> Result<Option<SeanceQuestionnable>> {
    let ligne = sqlx::query_as!(
        SeanceQuestionnable,
        r#"SELECT s.allows_questions, s.status::text AS "status!"
             FROM programme.sessions s
             JOIN programme.v_public_schedule v ON v.id = s.id
            WHERE s.id = $1"#,
        session_id.as_uuid()
    )
    .fetch_optional(executor)
    .await?;

    Ok(ligne)
}

/// `PublicSessionQuestion[]` — les questions visibles, les plus soutenues
/// d'abord, puis les plus anciennes. `seule` restreint à une question.
pub async fn visibles<'e>(
    executor: impl PgExecutor<'e>,
    session_id: SessionId,
    lecteur: Option<Uuid>,
    seule: Option<QuestionId>,
) -> Result<Vec<serde_json::Value>> {
    let lignes = sqlx::query_scalar!(
        r#"SELECT jsonb_build_object(
                      'id', q.id,
                      'session_id', q.session_id,
                      'body', q.body,
                      'vote_count', n.votes,
                      'has_voted', EXISTS (
                          SELECT 1 FROM programme.session_question_votes mv
                           WHERE mv.question_id = q.id AND mv.person_id = $2),
                      'is_mine', $2::uuid IS NOT NULL AND q.person_id IS NOT DISTINCT FROM $2,
                      'answered_at', q.answered_at,
                      'created_at', q.created_at,
                      'answers', COALESCE((
                          SELECT jsonb_agg(jsonb_build_object(
                                     'id', a.id,
                                     'body', a.body,
                                     'is_official', a.is_official,
                                     'created_at', a.created_at)
                                 ORDER BY a.created_at)
                            FROM programme.session_question_answers a
                           WHERE a.question_id = q.id), '[]'::jsonb)) AS "ligne!"
             FROM programme.session_questions q
             CROSS JOIN LATERAL (
                 SELECT count(*)::int4 AS votes
                   FROM programme.session_question_votes v
                  WHERE v.question_id = q.id) n
            WHERE q.session_id = $1
              AND q.is_visible
              AND ($3::uuid IS NULL OR q.id = $3)
            ORDER BY n.votes DESC, q.created_at, q.id"#,
        session_id.as_uuid(),
        lecteur,
        seule.map(QuestionId::as_uuid)
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes)
}

pub async fn poser(
    conn: &mut PgConnection,
    session_id: SessionId,
    auteur: Uuid,
    corps: &str,
) -> std::result::Result<QuestionId, sqlx::Error> {
    let id = sqlx::query_scalar!(
        "INSERT INTO programme.session_questions (session_id, person_id, body)
         VALUES ($1, $2, $3)
         RETURNING id",
        session_id.as_uuid(),
        auteur,
        corps
    )
    .fetch_one(conn)
    .await?;

    Ok(QuestionId(id))
}

/// Vraie si la question existe, est visible et appartient à la séance. Le
/// verrou sérialise les votes concurrents : sans lui, le recompte d'une
/// transaction ignorerait le vote de l'autre.
pub async fn verrouiller_si_visible(
    conn: &mut PgConnection,
    session_id: SessionId,
    question_id: QuestionId,
) -> Result<bool> {
    let trouvee = sqlx::query_scalar!(
        "SELECT id FROM programme.session_questions
          WHERE id = $1 AND session_id = $2 AND is_visible
          FOR UPDATE",
        question_id.as_uuid(),
        session_id.as_uuid()
    )
    .fetch_optional(conn)
    .await?;

    Ok(trouvee.is_some())
}

pub async fn voter(
    conn: &mut PgConnection,
    question_id: QuestionId,
    personne: Uuid,
) -> std::result::Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO programme.session_question_votes (question_id, person_id)
         VALUES ($1, $2)",
        question_id.as_uuid(),
        personne
    )
    .execute(conn)
    .await?;

    Ok(())
}

pub async fn retirer_le_vote(
    conn: &mut PgConnection,
    question_id: QuestionId,
    personne: Uuid,
) -> Result<()> {
    sqlx::query!(
        "DELETE FROM programme.session_question_votes
          WHERE question_id = $1 AND person_id = $2",
        question_id.as_uuid(),
        personne
    )
    .execute(conn)
    .await?;

    Ok(())
}

pub async fn recompter(conn: &mut PgConnection, question_id: QuestionId) -> Result<()> {
    sqlx::query!(
        "UPDATE programme.session_questions q
            SET upvotes = (SELECT count(*) FROM programme.session_question_votes v
                            WHERE v.question_id = q.id)
          WHERE q.id = $1",
        question_id.as_uuid()
    )
    .execute(conn)
    .await?;

    Ok(())
}
