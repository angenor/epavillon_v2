//! Où est un objet du média, et s'il est prêt : lu par `media.object_location()`,
//! le contrat que le schéma propriétaire offre. Ce module ne lit jamais
//! `media.assets`.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Emplacement {
    pub bucket: String,
    pub object_key: String,
    pub status: String,
    pub byte_size: i64,
    pub mime_type: String,
    pub original_filename: Option<String>,
}

pub async fn emplacement(conn: &mut PgConnection, asset_id: Uuid) -> Result<Option<Emplacement>> {
    let ligne = sqlx::query!(
        r#"SELECT bucket AS "bucket!", object_key AS "object_key!", status::text AS "status!",
                  byte_size AS "byte_size!", mime_type AS "mime_type!", original_filename
             FROM media.object_location($1)"#,
        asset_id
    )
    .fetch_optional(conn)
    .await?;
    Ok(ligne.map(|l| Emplacement {
        bucket: l.bucket,
        object_key: l.object_key,
        status: l.status,
        byte_size: l.byte_size,
        mime_type: l.mime_type,
        original_filename: l.original_filename,
    }))
}
