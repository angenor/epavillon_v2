//! Les écritures de la régie du jour (02/11), toutes dans le schéma
//! `programme` : état d'une séance, présence, journal des gestes. L'auteur est
//! celui que pose `Db::write` (`platform.current_actor_id()`).

use kernel::error::Result;
use serde_json::Value;
use sqlx::postgres::PgConnection;
use time::OffsetDateTime;
use uuid::Uuid;

/// Verrou de la règle n° 4 : deux équipes qui démarrent deux directs au même
/// instant passent l'une après l'autre.
const VERROU_DU_DIRECT: i64 = 0x_72_65_67_69_65;

pub struct SeanceVerrouillee {
    pub id: Uuid,
    pub event_id: Uuid,
    pub status: String,
    pub is_published: bool,
    pub live_ended_at: Option<OffsetDateTime>,
    pub now: OffsetDateTime,
}

pub async fn verrouiller_seance(
    conn: &mut PgConnection,
    session_id: Uuid,
) -> Result<Option<SeanceVerrouillee>> {
    let ligne = sqlx::query!(
        r#"SELECT id, event_id, status::text AS "status!", (published_at IS NOT NULL) AS "is_published!",
                  live_ended_at, now() AS "now!"
             FROM programme.sessions WHERE id = $1 FOR UPDATE"#,
        session_id
    )
    .fetch_optional(&mut *conn)
    .await?;
    Ok(ligne.map(|l| SeanceVerrouillee {
        id: l.id,
        event_id: l.event_id,
        status: l.status,
        is_published: l.is_published,
        live_ended_at: l.live_ended_at,
        now: l.now,
    }))
}

pub async fn verrouiller_le_direct(conn: &mut PgConnection) -> Result<()> {
    sqlx::query!("SELECT pg_advisory_xact_lock($1)", VERROU_DU_DIRECT)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

pub async fn presence(conn: &mut PgConnection, session_id: Uuid) -> Result<Option<bool>> {
    Ok(sqlx::query_scalar!(
        "SELECT is_present FROM programme.session_presence WHERE session_id = $1",
        session_id
    )
    .fetch_optional(&mut *conn)
    .await?)
}

/// `None` efface la présence : elle redevient inconnue.
pub async fn poser_presence(
    conn: &mut PgConnection,
    session_id: Uuid,
    presente: Option<bool>,
) -> Result<()> {
    match presente {
        None => {
            sqlx::query!(
                "DELETE FROM programme.session_presence WHERE session_id = $1",
                session_id
            )
            .execute(&mut *conn)
            .await?;
        }
        Some(p) => {
            sqlx::query!(
                "INSERT INTO programme.session_presence (session_id, is_present, noted_by)
                 VALUES ($1, $2, platform.current_actor_id())
                 ON CONFLICT (session_id) DO UPDATE
                    SET is_present = EXCLUDED.is_present, noted_at = now(),
                        noted_by = EXCLUDED.noted_by",
                session_id,
                p
            )
            .execute(&mut *conn)
            .await?;
        }
    }
    Ok(())
}

pub async fn changer_statut(conn: &mut PgConnection, session_id: Uuid, statut: &str) -> Result<()> {
    sqlx::query!(
        "UPDATE programme.sessions SET status = $2::text::programme.session_status WHERE id = $1",
        session_id,
        statut
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// Annule la séance avec la raison de la régie. Aucun motif public : la note
/// reste à l'équipe.
pub async fn annuler(
    conn: &mut PgConnection,
    session_id: Uuid,
    raison: &str,
    note: Option<&str>,
) -> Result<()> {
    sqlx::query!(
        "UPDATE programme.sessions
            SET status = 'cancelled', cancellation_reason_code = $2, cancellation_note = $3
          WHERE id = $1",
        session_id,
        raison,
        note
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

pub async fn inscrits(conn: &mut PgConnection, session_id: Uuid) -> Result<i64> {
    Ok(sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM programme.registrations
            WHERE session_id = $1 AND status IN ('registered', 'attended')"#,
        session_id
    )
    .fetch_one(&mut *conn)
    .await?)
}

pub async fn raison_connue(conn: &mut PgConnection, code: &str) -> Result<bool> {
    Ok(sqlx::query_scalar!(
        r#"SELECT EXISTS (SELECT 1 FROM reference.taxonomy_terms
                           WHERE taxonomy_code = 'session_cancellation_reason'
                             AND code = $1 AND is_active) AS "connue!""#,
        code
    )
    .fetch_one(&mut *conn)
    .await?)
}

pub struct Geste {
    pub id: Uuid,
    pub event_id: Uuid,
    pub session_id: Uuid,
    pub action_code: String,
    pub data: Value,
    pub acted_at: OffsetDateTime,
    pub undone_at: Option<OffsetDateTime>,
}

pub async fn inscrire_geste(
    conn: &mut PgConnection,
    event_id: Uuid,
    session_id: Uuid,
    code: &str,
    data: Value,
) -> Result<Geste> {
    let l = sqlx::query!(
        "INSERT INTO programme.control_room_actions (event_id, session_id, action_code, data, acted_by)
         VALUES ($1, $2, $3, $4, platform.current_actor_id())
         RETURNING id, event_id, session_id, action_code, data, acted_at, undone_at",
        event_id,
        session_id,
        code,
        data
    )
    .fetch_one(&mut *conn)
    .await?;
    Ok(Geste {
        id: l.id,
        event_id: l.event_id,
        session_id: l.session_id,
        action_code: l.action_code,
        data: l.data,
        acted_at: l.acted_at,
        undone_at: l.undone_at,
    })
}

pub async fn session_du_geste(conn: &mut PgConnection, action_id: Uuid) -> Result<Option<Uuid>> {
    Ok(sqlx::query_scalar!(
        "SELECT session_id FROM programme.control_room_actions WHERE id = $1",
        action_id
    )
    .fetch_optional(&mut *conn)
    .await?)
}

pub async fn verrouiller_geste(conn: &mut PgConnection, action_id: Uuid) -> Result<Option<Geste>> {
    let l = sqlx::query!(
        "SELECT id, event_id, session_id, action_code, data, acted_at, undone_at
           FROM programme.control_room_actions WHERE id = $1 FOR UPDATE",
        action_id
    )
    .fetch_optional(&mut *conn)
    .await?;
    Ok(l.map(|l| Geste {
        id: l.id,
        event_id: l.event_id,
        session_id: l.session_id,
        action_code: l.action_code,
        data: l.data,
        acted_at: l.acted_at,
        undone_at: l.undone_at,
    }))
}

/// Pose `undone_at` et fusionne `ajout` dans `data` (le choix fait en défaisant).
pub async fn marquer_defait(
    conn: &mut PgConnection,
    action_id: Uuid,
    ajout: Value,
) -> Result<OffsetDateTime> {
    Ok(sqlx::query_scalar!(
        r#"UPDATE programme.control_room_actions
              SET undone_at = now(), undone_by = platform.current_actor_id(), data = data || $2
            WHERE id = $1
        RETURNING undone_at AS "undone_at!""#,
        action_id,
        ajout
    )
    .fetch_one(&mut *conn)
    .await?)
}
