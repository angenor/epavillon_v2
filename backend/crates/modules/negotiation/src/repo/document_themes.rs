//! Les thématiques d'un document, par `reference.entity_terms` — patron de
//! `programme/src/repo/themes.rs`. Le couple de l'entité est écrit en littéral,
//! jamais reçu, et seuls les termes actifs de `negotiation_theme` se posent.

use kernel::error::{ApiError, ErrorCode, Result};
use sqlx::postgres::PgConnection;
use uuid::Uuid;

const TAXONOMIE: &str = "negotiation_theme";

/// Les codes d'un document, termes désactivés écartés, dans l'ordre posé.
pub async fn codes(conn: &mut PgConnection, document_id: Uuid) -> Result<Vec<String>> {
    let codes = sqlx::query_scalar!(
        r#"SELECT t.code AS "code!"
             FROM reference.entity_terms et
             JOIN reference.taxonomy_terms t ON t.id = et.term_id
            WHERE et.entity_schema = 'negotiation' AND et.entity_table = 'documents'
              AND et.entity_id = $1 AND t.taxonomy_code = $2 AND t.is_active
            ORDER BY et.sort_order, t.code"#,
        document_id,
        TAXONOMIE
    )
    .fetch_all(conn)
    .await?;
    Ok(codes)
}

/// Retire tous les liens de thématique du document, dans la transaction
/// appelante : c'est ce que fait aussi la suppression d'un brouillon.
pub async fn retirer(conn: &mut PgConnection, document_id: Uuid) -> Result<()> {
    sqlx::query!(
        "DELETE FROM reference.entity_terms et
          USING reference.taxonomy_terms t
          WHERE t.id = et.term_id AND t.taxonomy_code = $2
            AND et.entity_schema = 'negotiation' AND et.entity_table = 'documents'
            AND et.entity_id = $1",
        document_id,
        TAXONOMIE
    )
    .execute(conn)
    .await?;
    Ok(())
}

/// Remplace en bloc. Un code inconnu, d'un autre vocabulaire ou désactivé est
/// refusé en le nommant.
pub async fn remplacer(conn: &mut PgConnection, document_id: Uuid, codes: &[String]) -> Result<()> {
    retirer(conn, document_id).await?;
    let mut uniques: Vec<String> = Vec::new();
    for c in codes {
        if !uniques.contains(c) {
            uniques.push(c.clone());
        }
    }
    if uniques.is_empty() {
        return Ok(());
    }
    let poses = sqlx::query_scalar!(
        r#"WITH pose AS (
               INSERT INTO reference.entity_terms (entity_schema, entity_table, entity_id, term_id, sort_order)
               SELECT 'negotiation', 'documents', $1, t.id, c.rang::smallint
                 FROM unnest($3::text[]) WITH ORDINALITY AS c(code, rang)
                 JOIN reference.taxonomy_terms t
                   ON t.code = c.code AND t.taxonomy_code = $2 AND t.is_active
               ON CONFLICT DO NOTHING
               RETURNING term_id
           )
           SELECT count(*) AS "n!" FROM pose"#,
        document_id,
        TAXONOMIE,
        &uniques
    )
    .fetch_one(&mut *conn)
    .await?;
    if poses < uniques.len() as i64 {
        let connus = sqlx::query_scalar!(
            r#"SELECT code AS "code!" FROM reference.taxonomy_terms
                WHERE taxonomy_code = $1 AND is_active AND code = ANY($2)"#,
            TAXONOMIE,
            &uniques
        )
        .fetch_all(&mut *conn)
        .await?;
        let refuse = uniques
            .iter()
            .find(|c| !connus.contains(c))
            .cloned()
            .unwrap_or_default();
        return Err(ApiError::with_message(
            ErrorCode::NegotiationDocumentUnknownTheme,
            format!("Cette thématique n'existe pas : {refuse}."),
        )
        .field("themes"));
    }
    Ok(())
}
