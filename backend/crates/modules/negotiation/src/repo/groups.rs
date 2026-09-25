//! Les groupes suivis : même patron que `repo/themes.rs`. Un suivi se ferme,
//! il ne se supprime pas.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use uuid::Uuid;

pub use crate::repo::themes::Terme;

const TAXONOMIE: &str = "negotiation_group";

/// Les codes suivis, dans l'ordre du vocabulaire.
pub async fn suivis(conn: &mut PgConnection, person_id: Uuid) -> Result<Vec<String>> {
    let codes = sqlx::query_scalar!(
        r#"SELECT t.code AS "code!"
             FROM negotiation.group_subscriptions s
             JOIN reference.taxonomy_terms t ON t.id = s.group_term_id
            WHERE s.person_id = $1 AND s.left_at IS NULL
            ORDER BY t.sort_order, t.code"#,
        person_id
    )
    .fetch_all(conn)
    .await?;
    Ok(codes)
}

/// Sérialise les remplacements d'une même personne, qu'elle suive un groupe ou
/// non. Clé distincte de celle des thématiques : deux choix indépendants.
pub async fn verrouiller(conn: &mut PgConnection, person_id: Uuid) -> Result<()> {
    sqlx::query!(
        "SELECT pg_advisory_xact_lock(hashtextextended('groups:' || $1::text, 0)) AS verrou",
        person_id.to_string()
    )
    .fetch_one(conn)
    .await?;
    Ok(())
}

pub async fn termes(conn: &mut PgConnection, codes: &[String]) -> Result<Vec<Terme>> {
    let lignes = sqlx::query!(
        r#"SELECT code AS "code!", id AS "id!", is_active AS "is_active!"
             FROM reference.taxonomy_terms
            WHERE taxonomy_code = $1 AND code = ANY($2::text[])"#,
        TAXONOMIE,
        codes
    )
    .fetch_all(conn)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| Terme {
            code: l.code,
            id: l.id,
            is_active: l.is_active,
        })
        .collect())
}

pub async fn fermer_les_autres(
    conn: &mut PgConnection,
    person_id: Uuid,
    gardes: &[Uuid],
) -> Result<u64> {
    let fait = sqlx::query!(
        "UPDATE negotiation.group_subscriptions
            SET left_at = now()
          WHERE person_id = $1
            AND left_at IS NULL
            AND group_term_id <> ALL($2::uuid[])",
        person_id,
        gardes
    )
    .execute(conn)
    .await?;
    Ok(fait.rows_affected())
}

pub async fn ouvrir_les_nouveaux(
    conn: &mut PgConnection,
    person_id: Uuid,
    termes: &[Uuid],
) -> Result<u64> {
    let fait = sqlx::query!(
        "INSERT INTO negotiation.group_subscriptions (person_id, group_term_id)
         SELECT $1, t
           FROM unnest($2::uuid[]) AS t
          WHERE NOT EXISTS (
                SELECT 1 FROM negotiation.group_subscriptions s
                 WHERE s.person_id = $1 AND s.group_term_id = t AND s.left_at IS NULL
          )",
        person_id,
        termes
    )
    .execute(conn)
    .await?;
    Ok(fait.rows_affected())
}
