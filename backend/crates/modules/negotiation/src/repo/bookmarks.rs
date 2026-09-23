//! Les favoris : la clé est le couple personne-document, et chaque geste est
//! idempotent — poser deux fois, retirer deux fois, sans erreur.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use uuid::Uuid;

use crate::domain::documents::DocumentBookmark;

pub async fn lister(conn: &mut PgConnection, person_id: Uuid) -> Result<Vec<DocumentBookmark>> {
    let lignes = sqlx::query!(
        "SELECT document_id, created_at FROM negotiation.document_bookmarks
          WHERE person_id = $1 ORDER BY created_at DESC, document_id",
        person_id
    )
    .fetch_all(conn)
    .await?;
    Ok(lignes
        .into_iter()
        .map(|l| DocumentBookmark {
            document_id: l.document_id,
            created_at: l.created_at,
        })
        .collect())
}

pub async fn poser(conn: &mut PgConnection, person_id: Uuid, document_id: Uuid) -> Result<()> {
    sqlx::query!(
        "INSERT INTO negotiation.document_bookmarks (person_id, document_id) VALUES ($1, $2)
         ON CONFLICT DO NOTHING",
        person_id,
        document_id
    )
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn retirer(conn: &mut PgConnection, person_id: Uuid, document_id: Uuid) -> Result<()> {
    sqlx::query!(
        "DELETE FROM negotiation.document_bookmarks WHERE person_id = $1 AND document_id = $2",
        person_id,
        document_id
    )
    .execute(conn)
    .await?;
    Ok(())
}
