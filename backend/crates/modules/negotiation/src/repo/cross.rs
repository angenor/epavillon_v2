//! Lecture de `programme.sessions` — **en lecture seule**, jamais une écriture
//! (research R7). Le sélecteur du back-office y choisit l'activité du Pavillon
//! liée à une réunion.

use kernel::error::Result;
use serde_json::Value;
use sqlx::postgres::PgConnection;
use uuid::Uuid;

use crate::domain::admin_meetings::PavilionActivityOption;

/// Les activités de l'édition, par début ; les annulées aussi, que
/// l'administration reconnaît encore.
pub async fn activites_du_pavillon(
    conn: &mut PgConnection,
    event_id: Uuid,
) -> Result<Vec<PavilionActivityOption>> {
    let lignes = sqlx::query_as!(
        PavilionActivityOption,
        r#"SELECT s.id, s.title::jsonb AS "title!: Value", s.starts_at
             FROM programme.sessions s
            WHERE s.event_id = $1
            ORDER BY s.starts_at, s.id"#,
        event_id
    )
    .fetch_all(conn)
    .await?;
    Ok(lignes)
}
