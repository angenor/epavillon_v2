//! « Mon agenda » : la clé est le couple personne-session, chaque geste est
//! idempotent, et aucun chevauchement n'est refusé (règle n° 2).

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use uuid::Uuid;

use crate::domain::agenda::{AgendaEntry, NetworkAgendaEntry};

pub async fn lister(conn: &mut PgConnection, person_id: Uuid) -> Result<Vec<AgendaEntry>> {
    let lignes = sqlx::query!(
        r#"SELECT e.meeting_id,
                  (e.remind_before IS NOT NULL AND m.status <> 'cancelled') AS "remind!",
                  e.added_at
             FROM negotiation.agenda_entries e
             JOIN negotiation.meetings m ON m.id = e.meeting_id
            WHERE e.person_id = $1
            ORDER BY m.start_at, e.meeting_id"#,
        person_id
    )
    .fetch_all(conn)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| AgendaEntry {
            session_id: l.meeting_id,
            remind: l.remind,
            added_at: l.added_at,
        })
        .collect())
}

pub struct SessionVisee {
    pub annulee: bool,
    pub dans_lagenda: bool,
}

/// La session importée, et sa présence dans l'agenda. Nulle si elle n'existe
/// pas ou n'est pas une session officielle.
pub async fn session(
    conn: &mut PgConnection,
    person_id: Uuid,
    meeting_id: Uuid,
) -> Result<Option<SessionVisee>> {
    let ligne = sqlx::query!(
        r#"SELECT (m.status = 'cancelled') AS "annulee!",
                  EXISTS (SELECT 1 FROM negotiation.agenda_entries e
                           WHERE e.person_id = $1 AND e.meeting_id = m.id) AS "dans_lagenda!"
             FROM negotiation.meetings m
            WHERE m.id = $2 AND m.kind = 'negotiation_session' AND m.source_key IS NOT NULL"#,
        person_id,
        meeting_id
    )
    .fetch_optional(conn)
    .await?;

    Ok(ligne.map(|l| SessionVisee {
        annulee: l.annulee,
        dans_lagenda: l.dans_lagenda,
    }))
}

/// Ajoute, ou change le rappel. Un rejeu identique n'écrit rien : l'audit ne
/// s'encombre pas d'une mise à jour sans changement.
pub async fn poser(
    conn: &mut PgConnection,
    person_id: Uuid,
    meeting_id: Uuid,
    rappel: bool,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO negotiation.agenda_entries (person_id, meeting_id, remind_before)
         VALUES ($1, $2, CASE WHEN $3 THEN interval '15 minutes' END)
         ON CONFLICT (person_id, meeting_id) DO UPDATE
            SET remind_before = EXCLUDED.remind_before
          WHERE negotiation.agenda_entries.remind_before
                IS DISTINCT FROM EXCLUDED.remind_before",
        person_id,
        meeting_id,
        rappel
    )
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn retirer(conn: &mut PgConnection, person_id: Uuid, meeting_id: Uuid) -> Result<()> {
    sqlx::query!(
        "DELETE FROM negotiation.agenda_entries WHERE person_id = $1 AND meeting_id = $2",
        person_id,
        meeting_id
    )
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn lister_reseau(
    conn: &mut PgConnection,
    person_id: Uuid,
) -> Result<Vec<NetworkAgendaEntry>> {
    let lignes = sqlx::query!(
        r#"SELECT e.network_meeting_id,
                  (e.remind_before IS NOT NULL AND n.withdrawn_at IS NULL) AS "remind!",
                  e.added_at
             FROM negotiation.network_agenda_entries e
             JOIN negotiation.network_meetings n ON n.id = e.network_meeting_id
            WHERE e.person_id = $1
            ORDER BY n.day, n.start_at NULLS LAST, e.network_meeting_id"#,
        person_id
    )
    .fetch_all(conn)
    .await?;
    Ok(lignes
        .into_iter()
        .map(|l| NetworkAgendaEntry {
            network_meeting_id: l.network_meeting_id,
            remind: l.remind,
            added_at: l.added_at,
        })
        .collect())
}

/// Une réunion non annoncée publiée et non retirée.
pub async fn reunion_affichee(conn: &mut PgConnection, id: Uuid) -> Result<bool> {
    Ok(sqlx::query_scalar!(
        r#"SELECT EXISTS (
               SELECT 1 FROM negotiation.network_meetings n
                 JOIN negotiation.session_reports r ON r.id = n.report_id
                WHERE n.id = $1 AND n.withdrawn_at IS NULL AND r.published_at IS NOT NULL
           ) AS "existe!""#,
        id
    )
    .fetch_one(conn)
    .await?)
}

pub async fn poser_reunion(
    conn: &mut PgConnection,
    person_id: Uuid,
    network_meeting_id: Uuid,
    rappel: bool,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO negotiation.network_agenda_entries (person_id, network_meeting_id, remind_before)
         VALUES ($1, $2, CASE WHEN $3 THEN interval '15 minutes' END)
         ON CONFLICT (person_id, network_meeting_id) DO UPDATE
            SET remind_before = EXCLUDED.remind_before
          WHERE negotiation.network_agenda_entries.remind_before
                IS DISTINCT FROM EXCLUDED.remind_before",
        person_id,
        network_meeting_id,
        rappel
    )
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn retirer_reunion(
    conn: &mut PgConnection,
    person_id: Uuid,
    network_meeting_id: Uuid,
) -> Result<()> {
    sqlx::query!(
        "DELETE FROM negotiation.network_agenda_entries
          WHERE person_id = $1 AND network_meeting_id = $2",
        person_id,
        network_meeting_id
    )
    .execute(conn)
    .await?;
    Ok(())
}
