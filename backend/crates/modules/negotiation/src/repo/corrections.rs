//! Les notes de correction : posées sur une page, retirées en datant, jamais
//! supprimées.

use kernel::error::Result;
use serde_json::Value;
use sqlx::postgres::PgConnection;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct NoteLue {
    pub id: Uuid,
    pub document_id: Uuid,
    pub page_index: i32,
    pub passage: Option<String>,
    pub body: Value,
    pub author_id: Uuid,
    pub author_name: String,
    pub created_at: OffsetDateTime,
    pub withdrawn_at: Option<OffsetDateTime>,
    pub withdrawn_by: Option<Uuid>,
    pub withdrawn_by_name: Option<String>,
    /// Le document est-il réservé ? La lecture publique en dépend.
    pub restricted: bool,
}

/// Les notes vivantes de tous les documents publiés.
pub async fn vivantes(conn: &mut PgConnection) -> Result<Vec<NoteLue>> {
    let lignes = sqlx::query!(
        r#"SELECT n.id, n.document_id, n.page_index, n.passage, n.body AS "body!: Value",
                  n.author_id, concat_ws(' ', a.first_name, a.last_name) AS "author_name!",
                  n.created_at, d.is_restricted
             FROM negotiation.correction_notes n
             JOIN negotiation.documents d ON d.id = n.document_id AND d.published_at IS NOT NULL
             JOIN identity.people a ON a.id = n.author_id
            WHERE n.withdrawn_at IS NULL
            ORDER BY n.document_id, n.page_index, n.created_at, n.id"#
    )
    .fetch_all(conn)
    .await?;
    Ok(lignes
        .into_iter()
        .map(|l| NoteLue {
            id: l.id,
            document_id: l.document_id,
            page_index: l.page_index,
            passage: l.passage,
            body: l.body,
            author_id: l.author_id,
            author_name: l.author_name,
            created_at: l.created_at,
            withdrawn_at: None,
            withdrawn_by: None,
            withdrawn_by_name: None,
            restricted: l.is_restricted,
        })
        .collect())
}

/// Toutes les notes d'un document, vivantes et retirées.
pub async fn du_document(conn: &mut PgConnection, document_id: Uuid) -> Result<Vec<NoteLue>> {
    let lignes = sqlx::query!(
        r#"SELECT n.id, n.document_id, n.page_index, n.passage, n.body AS "body!: Value",
                  n.author_id, concat_ws(' ', a.first_name, a.last_name) AS "author_name!",
                  n.created_at, n.withdrawn_at, n.withdrawn_by,
                  CASE WHEN r.id IS NULL THEN NULL
                       ELSE concat_ws(' ', r.first_name, r.last_name) END AS withdrawn_by_name,
                  d.is_restricted
             FROM negotiation.correction_notes n
             JOIN negotiation.documents d ON d.id = n.document_id
             JOIN identity.people a ON a.id = n.author_id
             LEFT JOIN identity.people r ON r.id = n.withdrawn_by
            WHERE n.document_id = $1
            ORDER BY n.page_index, n.created_at, n.id"#,
        document_id
    )
    .fetch_all(conn)
    .await?;
    Ok(lignes
        .into_iter()
        .map(|l| NoteLue {
            id: l.id,
            document_id: l.document_id,
            page_index: l.page_index,
            passage: l.passage,
            body: l.body,
            author_id: l.author_id,
            author_name: l.author_name,
            created_at: l.created_at,
            withdrawn_at: l.withdrawn_at,
            withdrawn_by: l.withdrawn_by,
            withdrawn_by_name: l.withdrawn_by_name,
            restricted: l.is_restricted,
        })
        .collect())
}

pub async fn poser(
    conn: &mut PgConnection,
    document_id: Uuid,
    page_index: i32,
    passage: Option<&str>,
    body: &Value,
    auteur: Uuid,
) -> Result<Uuid> {
    let id = sqlx::query_scalar!(
        "INSERT INTO negotiation.correction_notes (document_id, page_index, passage, body, author_id)
         VALUES ($1, $2, $3, $4::jsonb::platform.i18n_text, $5)
         RETURNING id",
        document_id,
        page_index,
        passage,
        body,
        auteur
    )
    .fetch_one(conn)
    .await?;
    Ok(id)
}

/// Idempotent : une note déjà retirée garde sa première date de retrait.
/// Rend le document de la note, ou `None` si elle n'existe pas.
pub async fn retirer(conn: &mut PgConnection, note_id: Uuid, par: Uuid) -> Result<Option<Uuid>> {
    let document = sqlx::query_scalar!(
        "UPDATE negotiation.correction_notes
            SET withdrawn_at = COALESCE(withdrawn_at, now()),
                withdrawn_by = COALESCE(withdrawn_by, $2)
          WHERE id = $1
         RETURNING document_id",
        note_id,
        par
    )
    .fetch_optional(conn)
    .await?;
    Ok(document)
}
