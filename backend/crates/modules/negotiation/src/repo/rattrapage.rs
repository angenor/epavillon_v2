//! Le rattrapage des encarts, dans la transaction de la lecture (R7).

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::reports::ReportReason;
use crate::import::rattrapage::{rattrape, Affiche, Officielle};

/// Retire les encarts que la source a rattrapés. Rend leur nombre.
pub async fn appliquer(
    conn: &mut PgConnection,
    event_id: Uuid,
    lu_a: OffsetDateTime,
) -> Result<usize> {
    let lignes = sqlx::query!(
        r#"SELECT r.id, r.reason, r.proposed_start, r.proposed_venue,
                  (m.status = 'cancelled') AS "annulee!", m.start_at, m.venue_label
             FROM negotiation.session_reports r
             JOIN negotiation.meetings m ON m.id = r.meeting_id
            WHERE r.event_id = $1 AND r.published_at IS NOT NULL AND r.withdrawn_at IS NULL
              AND r.reason IN ('cancelled', 'time', 'venue')"#,
        event_id
    )
    .fetch_all(&mut *conn)
    .await?;

    let rattrapes: Vec<Uuid> = lignes
        .into_iter()
        .filter_map(|l| {
            let affiche = Affiche {
                id: l.id,
                reason: ReportReason::from_db(&l.reason)?,
                proposed_start: l.proposed_start,
                proposed_venue: l.proposed_venue,
            };
            let officielle = Officielle {
                annulee: l.annulee,
                debut: l.start_at,
                salle: l.venue_label,
            };
            rattrape(&affiche, &officielle).then_some(affiche.id)
        })
        .collect();
    if rattrapes.is_empty() {
        return Ok(0);
    }
    sqlx::query!(
        "UPDATE negotiation.session_reports
            SET withdrawn_at = $2, withdrawal = 'caught_up'
          WHERE id = ANY($1) AND withdrawn_at IS NULL",
        &rattrapes,
        lu_a
    )
    .execute(conn)
    .await?;
    Ok(rattrapes.len())
}
