//! La forme lisible, page par page. Une nouvelle extraction remplace toutes les
//! pages d'un document, dans la transaction qui la conclut.

use kernel::error::Result;
use serde_json::Value;
use sqlx::postgres::PgConnection;
use uuid::Uuid;

pub struct PageAEcrire {
    pub index: i32,
    pub label: String,
    pub blocks: Value,
    pub plain_text: String,
    pub image_key: Option<String>,
    pub image_bytes: Option<i32>,
    pub has_origin_block: bool,
}

#[derive(Debug, Clone)]
pub struct PageLue {
    pub index: i32,
    pub label: String,
    pub blocks: Value,
    pub image_key: Option<String>,
    pub image_bytes: Option<i32>,
    pub has_origin_block: bool,
}

pub async fn remplacer(
    conn: &mut PgConnection,
    document_id: Uuid,
    pages: &[PageAEcrire],
) -> Result<()> {
    sqlx::query!(
        "DELETE FROM negotiation.document_pages WHERE document_id = $1",
        document_id
    )
    .execute(&mut *conn)
    .await?;
    for p in pages {
        sqlx::query!(
            "INSERT INTO negotiation.document_pages
                 (document_id, page_index, label, blocks, plain_text, image_key, image_bytes, has_origin_block)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            document_id,
            p.index,
            p.label,
            p.blocks,
            p.plain_text,
            p.image_key,
            p.image_bytes,
            p.has_origin_block
        )
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}

pub async fn lire(conn: &mut PgConnection, document_id: Uuid) -> Result<Vec<PageLue>> {
    let lignes = sqlx::query!(
        r#"SELECT page_index, label, blocks, image_key, image_bytes, has_origin_block
             FROM negotiation.document_pages
            WHERE document_id = $1
            ORDER BY page_index"#,
        document_id
    )
    .fetch_all(conn)
    .await?;
    Ok(lignes
        .into_iter()
        .map(|l| PageLue {
            index: l.page_index,
            label: l.label,
            blocks: l.blocks,
            image_key: l.image_key,
            image_bytes: l.image_bytes,
            has_origin_block: l.has_origin_block,
        })
        .collect())
}

/// Les clés d'image d'un document, pour effacer celles qu'une nouvelle
/// extraction rend obsolètes.
pub async fn cles_d_images(conn: &mut PgConnection, document_id: Uuid) -> Result<Vec<String>> {
    let cles = sqlx::query_scalar!(
        r#"SELECT image_key AS "image_key!"
             FROM negotiation.document_pages
            WHERE document_id = $1 AND image_key IS NOT NULL"#,
        document_id
    )
    .fetch_all(conn)
    .await?;
    Ok(cles)
}
