//! Les termes favoris : la clé est le couple personne-entrée, et chaque geste
//! est idempotent.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use uuid::Uuid;

/// Seules les entrées servies : un terme repassé en brouillon sort de la liste
/// sans que sa ligne soit effacée.
pub async fn lister(conn: &mut PgConnection, person_id: Uuid) -> Result<Vec<Uuid>> {
    let ids = sqlx::query_scalar!(
        "SELECT f.entry_id FROM negotiation.glossary_favorites f
           JOIN negotiation.glossary_entries e ON e.id = f.entry_id
          WHERE f.person_id = $1 AND e.status <> 'draft'
          ORDER BY f.created_at DESC, f.entry_id",
        person_id
    )
    .fetch_all(conn)
    .await?;
    Ok(ids)
}

pub async fn servie(conn: &mut PgConnection, entry_id: Uuid) -> Result<bool> {
    let existe = sqlx::query_scalar!(
        r#"SELECT EXISTS (SELECT 1 FROM negotiation.glossary_entries
                           WHERE id = $1 AND status <> 'draft') AS "existe!""#,
        entry_id
    )
    .fetch_one(conn)
    .await?;
    Ok(existe)
}

pub async fn poser(conn: &mut PgConnection, person_id: Uuid, entry_id: Uuid) -> Result<()> {
    sqlx::query!(
        "INSERT INTO negotiation.glossary_favorites (person_id, entry_id) VALUES ($1, $2)
         ON CONFLICT DO NOTHING",
        person_id,
        entry_id
    )
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn retirer(conn: &mut PgConnection, person_id: Uuid, entry_id: Uuid) -> Result<()> {
    sqlx::query!(
        "DELETE FROM negotiation.glossary_favorites WHERE person_id = $1 AND entry_id = $2",
        person_id,
        entry_id
    )
    .execute(conn)
    .await?;
    Ok(())
}
