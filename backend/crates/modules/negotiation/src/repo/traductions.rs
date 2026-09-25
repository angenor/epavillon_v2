//! Les traductions de titres : le modèle qui rédige, et l'écriture d'un lot.

use kernel::error::Result;
use sqlx::postgres::PgConnection;

pub const CLE_MODELE: &str = "ai.drafting_model";

/// Le modèle de rédaction d'ADR-005, relu à chaque traduction.
pub async fn modele(conn: &mut PgConnection) -> Result<Option<String>> {
    let modele = sqlx::query_scalar!(
        r#"SELECT (s.value #>> '{}') AS "modele?" FROM platform.settings s WHERE s.key = $1"#,
        CLE_MODELE
    )
    .fetch_optional(conn)
    .await?
    .flatten()
    .filter(|m| !m.trim().is_empty());
    Ok(modele)
}

/// Un titre déjà traduit ne l'est pas deux fois : la première traduction reste.
pub async fn ecrire(
    conn: &mut PgConnection,
    titres: &[String],
    traductions: &[String],
    modele: &str,
) -> Result<u64> {
    let ecrites = sqlx::query!(
        "INSERT INTO negotiation.title_translations (source_text, text_fr, model)
         SELECT u.titre, u.traduction, $3
           FROM unnest($1::text[], $2::text[]) AS u(titre, traduction)
         ON CONFLICT (source_text) DO NOTHING",
        titres,
        traductions,
        modele
    )
    .execute(conn)
    .await?
    .rows_affected();
    Ok(ecrites)
}
