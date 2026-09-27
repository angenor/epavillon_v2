//! Les coches du parcours : la clé est le couple personne-étape, et chaque geste
//! est idempotent (R7).

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use uuid::Uuid;

/// Seules les étapes publiées d'un groupe publié : une étape dépubliée sort de la
/// liste sans que sa coche soit effacée.
pub async fn lister(conn: &mut PgConnection, person_id: Uuid) -> Result<Vec<Uuid>> {
    Ok(sqlx::query_scalar!(
        "SELECT c.step_id FROM negotiation.pathway_checks c
           JOIN negotiation.pathway_steps s ON s.id = c.step_id
           JOIN negotiation.pathway_groups g ON g.id = s.group_id
          WHERE c.person_id = $1 AND s.is_published AND g.is_published
          ORDER BY c.step_id",
        person_id
    )
    .fetch_all(conn)
    .await?)
}

pub async fn publiee(conn: &mut PgConnection, step_id: Uuid) -> Result<bool> {
    Ok(sqlx::query_scalar!(
        r#"SELECT EXISTS (SELECT 1 FROM negotiation.pathway_steps s
                            JOIN negotiation.pathway_groups g ON g.id = s.group_id
                           WHERE s.id = $1 AND s.is_published AND g.is_published) AS "existe!""#,
        step_id
    )
    .fetch_one(conn)
    .await?)
}

pub async fn cocher(conn: &mut PgConnection, person_id: Uuid, step_id: Uuid) -> Result<()> {
    sqlx::query!(
        "INSERT INTO negotiation.pathway_checks (person_id, step_id) VALUES ($1, $2)
         ON CONFLICT DO NOTHING",
        person_id,
        step_id
    )
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn decocher(conn: &mut PgConnection, person_id: Uuid, step_id: Uuid) -> Result<()> {
    sqlx::query!(
        "DELETE FROM negotiation.pathway_checks WHERE person_id = $1 AND step_id = $2",
        person_id,
        step_id
    )
    .execute(conn)
    .await?;
    Ok(())
}
