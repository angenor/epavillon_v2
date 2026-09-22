//! Les suivis d'une personne : lire les vivants, en fermer, en ouvrir.
//!
//! **Un suivi se ferme, il ne se supprime pas** : `left_at` est posé, et la
//! ligne reste. Ce qu'une personne a suivi pendant une COP se relit.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use uuid::Uuid;

use crate::domain::themes::FollowedTheme;

const TAXONOMIE: &str = "negotiation_theme";

/// Les suivis vivants, dans l'ordre du vocabulaire.
pub async fn suivis(conn: &mut PgConnection, person_id: Uuid) -> Result<Vec<FollowedTheme>> {
    let lignes = sqlx::query!(
        r#"SELECT t.code AS "code!", s.followed_at AS "followed_at!"
             FROM negotiation.theme_subscriptions s
             JOIN reference.taxonomy_terms t ON t.id = s.theme_term_id
            WHERE s.person_id = $1 AND s.left_at IS NULL
            ORDER BY t.sort_order, t.code"#,
        person_id
    )
    .fetch_all(conn)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| FollowedTheme {
            code: l.code,
            followed_at: l.followed_at,
        })
        .collect())
}

/// Sérialise les remplacements d'une même personne, **qu'elle ait des suivis ou
/// non** : un `SELECT … FOR UPDATE` ne verrouille rien quand la table est vide
/// pour elle, et c'est précisément le premier choix que deux appareils peuvent
/// envoyer ensemble.
pub async fn verrouiller(conn: &mut PgConnection, person_id: Uuid) -> Result<()> {
    sqlx::query!(
        "SELECT pg_advisory_xact_lock(hashtextextended($1::text, 0)) AS verrou",
        person_id.to_string()
    )
    .fetch_one(conn)
    .await?;
    Ok(())
}

pub struct Terme {
    pub code: String,
    pub id: Uuid,
    pub is_active: bool,
}

/// Les termes du vocabulaire pour ces codes. **Ce qui manque à la réponse
/// n'existe pas** — ou relève d'un autre vocabulaire, ce qui revient au même.
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

/// Ferme les suivis vivants **absents** de la liste. Les autres ne sont pas
/// touchés : leur `followed_at` ne bouge pas, et l'audit ne s'encombre pas
/// d'une écriture sans changement.
pub async fn fermer_les_autres(
    conn: &mut PgConnection,
    person_id: Uuid,
    gardes: &[Uuid],
) -> Result<u64> {
    let fait = sqlx::query!(
        "UPDATE negotiation.theme_subscriptions
            SET left_at = now()
          WHERE person_id = $1
            AND left_at IS NULL
            AND theme_term_id <> ALL($2::uuid[])",
        person_id,
        gardes
    )
    .execute(conn)
    .await?;
    Ok(fait.rows_affected())
}

/// Ouvre un suivi pour chaque terme qui n'en a pas de vivant.
pub async fn ouvrir_les_nouveaux(
    conn: &mut PgConnection,
    person_id: Uuid,
    termes: &[Uuid],
) -> Result<u64> {
    let fait = sqlx::query!(
        "INSERT INTO negotiation.theme_subscriptions (person_id, theme_term_id)
         SELECT $1, t
           FROM unnest($2::uuid[]) AS t
          WHERE NOT EXISTS (
                SELECT 1 FROM negotiation.theme_subscriptions s
                 WHERE s.person_id = $1 AND s.theme_term_id = t AND s.left_at IS NULL
          )",
        person_id,
        termes
    )
    .execute(conn)
    .await?;
    Ok(fait.rows_affected())
}
