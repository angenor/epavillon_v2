//! L'état de l'extraction d'un document : une ligne par document, qui suit son
//! fichier du moment.

use kernel::error::Result;
use serde_json::Value;
use sqlx::postgres::PgConnection;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Rendu {
    pub asset_id: Uuid,
    pub status: String,
    pub page_count: Option<i32>,
    pub outline: Option<Value>,
    pub is_reflowable: Option<bool>,
    pub quality: Option<Value>,
    /// NULL : suit le verdict. La règle est dans `document_reading_modes()`.
    pub large_text_choice: Option<bool>,
    pub has_text: bool,
    pub large_text: bool,
    pub reading_bytes: Option<i64>,
    pub extractor: Option<String>,
    pub failure_reason: Option<String>,
    pub attempts: i32,
    pub extracted_at: Option<OffsetDateTime>,
}

pub async fn lire(conn: &mut PgConnection, document_id: Uuid) -> Result<Option<Rendu>> {
    let ligne = sqlx::query!(
        r#"SELECT r.asset_id, r.status::text AS "status!", r.page_count, r.outline, r.is_reflowable,
                  r.quality, r.large_text_choice, m.has_text AS "has_text!", m.large_text AS "large_text!",
                  r.reading_bytes, r.extractor, r.failure_reason, r.attempts, r.extracted_at
             FROM negotiation.document_renditions r
             CROSS JOIN LATERAL negotiation.document_reading_modes(r.document_id) m
            WHERE r.document_id = $1"#,
        document_id
    )
    .fetch_optional(conn)
    .await?;
    Ok(ligne.map(|l| Rendu {
        asset_id: l.asset_id,
        status: l.status,
        page_count: l.page_count,
        outline: l.outline,
        is_reflowable: l.is_reflowable,
        quality: l.quality,
        large_text_choice: l.large_text_choice,
        has_text: l.has_text,
        large_text: l.large_text,
        reading_bytes: l.reading_bytes,
        extractor: l.extractor,
        failure_reason: l.failure_reason,
        attempts: l.attempts,
        extracted_at: l.extracted_at,
    }))
}

/// Demande une extraction pour ce fichier : l'état repart de `pending`, le
/// verdict précédent s'efface, et seul le travail de cette demande pourra
/// conclure. Le choix « Texte agrandi » de l'administratrice est gardé : il ne
/// dépend pas du fichier.
pub async fn demander(
    conn: &mut PgConnection,
    document_id: Uuid,
    asset_id: Uuid,
    demande: Uuid,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO negotiation.document_renditions (document_id, asset_id, request_id)
         VALUES ($1, $2, $3)
         ON CONFLICT (document_id) DO UPDATE
            SET asset_id = EXCLUDED.asset_id,
                request_id = EXCLUDED.request_id,
                status = 'pending',
                page_count = NULL,
                outline = NULL,
                is_reflowable = NULL,
                quality = NULL,
                reading_bytes = NULL,
                extractor = NULL,
                failure_reason = NULL,
                extracted_at = NULL",
        document_id,
        asset_id,
        demande
    )
    .execute(conn)
    .await?;
    Ok(())
}

/// Passe l'extraction de ce fichier à `extracting`. Faux : le document a changé
/// de fichier ou d'extraction demandée depuis la mise en file, et ce travail
/// n'a plus d'objet.
pub async fn commencer(
    conn: &mut PgConnection,
    document_id: Uuid,
    asset_id: Uuid,
    demande: Uuid,
) -> Result<bool> {
    let faite = sqlx::query!(
        "UPDATE negotiation.document_renditions
            SET status = 'extracting', attempts = attempts + 1
          WHERE document_id = $1 AND asset_id = $2 AND request_id = $3",
        document_id,
        asset_id,
        demande
    )
    .execute(conn)
    .await?;
    Ok(faite.rows_affected() == 1)
}

pub struct Verdict<'a> {
    pub page_count: i32,
    pub outline: &'a Value,
    pub is_reflowable: bool,
    pub quality: &'a Value,
    pub extractor: &'a str,
}

pub async fn reussir(
    conn: &mut PgConnection,
    document_id: Uuid,
    asset_id: Uuid,
    demande: Uuid,
    v: &Verdict<'_>,
) -> Result<bool> {
    let faite = sqlx::query!(
        "UPDATE negotiation.document_renditions
            SET status = 'ready', page_count = $3, outline = $4, is_reflowable = $5,
                quality = $6, extractor = $7,
                failure_reason = NULL, extracted_at = now()
          WHERE document_id = $1 AND asset_id = $2 AND request_id = $8",
        document_id,
        asset_id,
        v.page_count,
        v.outline,
        v.is_reflowable,
        v.quality,
        v.extractor,
        demande
    )
    .execute(conn)
    .await?;
    Ok(faite.rows_affected() == 1)
}

pub async fn echouer(
    conn: &mut PgConnection,
    document_id: Uuid,
    asset_id: Uuid,
    demande: Uuid,
    motif: &str,
) -> Result<()> {
    sqlx::query!(
        "UPDATE negotiation.document_renditions
            SET status = 'failed', failure_reason = $4
          WHERE document_id = $1 AND asset_id = $2 AND request_id = $3",
        document_id,
        asset_id,
        demande,
        motif
    )
    .execute(conn)
    .await?;
    Ok(())
}

/// La taille annoncée de la copie gardée : le PDF et la lecture servie.
pub async fn poser_le_poids(conn: &mut PgConnection, document_id: Uuid, octets: i64) -> Result<()> {
    sqlx::query!(
        "UPDATE negotiation.document_renditions SET reading_bytes = $2 WHERE document_id = $1",
        document_id,
        octets
    )
    .execute(conn)
    .await?;
    Ok(())
}

/// `None` rend la main au verdict de l'extraction.
pub async fn poser_le_choix_texte_agrandi(
    conn: &mut PgConnection,
    document_id: Uuid,
    choix: Option<bool>,
) -> Result<bool> {
    let faite = sqlx::query!(
        "UPDATE negotiation.document_renditions SET large_text_choice = $2 WHERE document_id = $1",
        document_id,
        choix
    )
    .execute(conn)
    .await?;
    Ok(faite.rows_affected() == 1)
}
