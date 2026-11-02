//! Les réunions Zoom vues du poste de direct : état, présents, échecs
//! d'inscription. Deux requêtes pour toutes les activités du poste.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use uuid::Uuid;

use crate::domain::desk::{motif_d_echec, LiveDeskMeeting, RegistrationFailure};

/// Échecs détaillés par réunion ; au-delà, le compte suffit.
const ECHECS_MONTRES: i64 = 10;

/// La réunion la plus récente de chaque activité, une réunion vivante avant
/// une réunion en échec : la création ratée doit se voir tant qu'aucune autre
/// ne l'a remplacée.
pub async fn reunions(
    conn: &mut PgConnection,
    session_ids: &[Uuid],
) -> Result<Vec<(Uuid, LiveDeskMeeting)>> {
    let lignes = sqlx::query!(
        r#"SELECT DISTINCT ON (x.sid)
                  x.sid AS "session_id!", m.id, m.status::text AS "status!",
                  m.sync_status::text AS "sync_status!", m.last_sync_error, m.actual_start_at,
                  COALESCE(c.present_count, 0) AS "present_count!",
                  COALESCE(c.joined_count, 0) AS "joined_count!",
                  (SELECT count(*) FROM live.meeting_participants p
                    WHERE p.meeting_id = m.id AND p.cancelled_at IS NULL
                      AND p.sync_status = 'synced') AS "registered_count!",
                  (SELECT count(*) FROM live.meeting_participants p
                    WHERE p.meeting_id = m.id AND p.cancelled_at IS NULL
                      AND p.sync_status IN ('failed', 'abandoned')) AS "failure_count!"
             FROM unnest($1::uuid[]) AS x(sid)
             -- Une activité de journée montre la réunion de la journée (02/11).
             JOIN live.meetings m
               ON m.session_id = x.sid OR m.track_id = programme.day_track_of(x.sid)
             LEFT JOIN live.meeting_room_counts c ON c.meeting_id = m.id
            WHERE m.status <> 'cancelled'
            ORDER BY x.sid, (m.status <> 'failed') DESC, m.created_at DESC"#,
        session_ids
    )
    .fetch_all(&mut *conn)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| {
            (
                l.session_id,
                LiveDeskMeeting {
                    meeting_id: l.id,
                    status: l.status,
                    sync_status: l.sync_status,
                    last_sync_error: l.last_sync_error,
                    actual_start_at: l.actual_start_at,
                    present_count: l.present_count,
                    joined_count: l.joined_count,
                    registered_count: l.registered_count,
                    failure_count: l.failure_count,
                    failures: Vec::new(),
                },
            )
        })
        .collect())
}

pub async fn echecs(
    conn: &mut PgConnection,
    meeting_ids: &[Uuid],
) -> Result<Vec<(Uuid, RegistrationFailure)>> {
    let lignes = sqlx::query!(
        r#"SELECT meeting_id AS "meeting_id!", id AS "id!", display_name AS "display_name!",
                  email AS "email!", sync_status AS "sync_status!", last_error,
                  sync_attempts AS "sync_attempts!", next_retry_at
             FROM (SELECT p.meeting_id, p.id, p.display_name, p.email::text AS email,
                          p.sync_status::text AS sync_status, p.last_error, p.sync_attempts,
                          p.next_retry_at,
                          row_number() OVER (PARTITION BY p.meeting_id
                                             ORDER BY p.last_attempt_at DESC NULLS LAST, p.id) AS rang
                     FROM live.meeting_participants p
                    WHERE p.meeting_id = ANY ($1) AND p.cancelled_at IS NULL
                      AND p.sync_status IN ('failed', 'abandoned')) x
            WHERE rang <= $2
            ORDER BY meeting_id, rang"#,
        meeting_ids,
        ECHECS_MONTRES
    )
    .fetch_all(&mut *conn)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| {
            (
                l.meeting_id,
                RegistrationFailure {
                    participant_id: l.id,
                    display_name: l.display_name,
                    email: l.email,
                    reason: motif_d_echec(l.last_error.as_deref()),
                    sync_status: l.sync_status,
                    detail: l.last_error,
                    attempts: l.sync_attempts,
                    next_retry_at: l.next_retry_at,
                },
            )
        })
        .collect())
}

/// Zoom a-t-il déjà envoyé une entrée en salle signée ? Sans quoi un « 0
/// présent » ne veut rien dire.
pub async fn presences_recues(conn: &mut PgConnection) -> Result<bool> {
    let recu = sqlx::query_scalar!(
        r#"SELECT EXISTS (SELECT 1 FROM live.provider_webhook_events
                           WHERE event_type = 'meeting.participant_joined'
                             AND signature_verified) AS "recu!""#
    )
    .fetch_one(conn)
    .await?;
    Ok(recu)
}
