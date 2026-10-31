//! `engagement.mail_texts` (01/11).

use kernel::error::Result;
use sqlx::{PgConnection, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;

pub struct LigneTexte {
    pub id: Uuid,
    pub name: String,
    pub category: Option<String>,
    pub subject: String,
    pub body_html: String,
    pub sort_order: i16,
    pub event_id: Option<Uuid>,
    pub created_by_name: Option<String>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

/// Les textes utilisables pour une édition : les siens et les communs, rangés
/// par catégorie (les sans catégorie à la fin), rang, puis nom.
pub async fn lister(pool: &PgPool, event_id: Uuid) -> Result<Vec<LigneTexte>> {
    let lignes = sqlx::query_as!(
        LigneTexte,
        r#"SELECT t.id, t.name, t.category, t.subject, t.body_html, t.sort_order, t.event_id,
                  (SELECT p.display_name FROM identity.people p WHERE p.id = t.created_by) AS created_by_name,
                  t.created_at, t.updated_at
             FROM engagement.mail_texts t
            WHERE t.event_id IS NULL OR t.event_id = $1
            ORDER BY lower(t.category) NULLS LAST, t.sort_order, lower(t.name)"#,
        event_id
    )
    .fetch_all(pool)
    .await?;
    Ok(lignes)
}

pub async fn lire(pool: &PgPool, id: Uuid) -> Result<Option<LigneTexte>> {
    let ligne = sqlx::query_as!(
        LigneTexte,
        r#"SELECT t.id, t.name, t.category, t.subject, t.body_html, t.sort_order, t.event_id,
                  (SELECT p.display_name FROM identity.people p WHERE p.id = t.created_by) AS created_by_name,
                  t.created_at, t.updated_at
             FROM engagement.mail_texts t
            WHERE t.id = $1"#,
        id
    )
    .fetch_optional(pool)
    .await?;
    Ok(ligne)
}

pub struct ValeursTexte<'a> {
    pub name: &'a str,
    pub category: Option<&'a str>,
    pub subject: &'a str,
    pub body_html: &'a str,
    pub sort_order: i16,
    pub event_id: Option<Uuid>,
}

pub async fn creer(conn: &mut PgConnection, acteur: Uuid, v: &ValeursTexte<'_>) -> Result<Uuid> {
    let id = sqlx::query_scalar!(
        r#"INSERT INTO engagement.mail_texts
               (name, category, subject, body_html, sort_order, event_id, created_by)
           VALUES ($1, $2, $3, $4, $5, $6, $7)
           RETURNING id"#,
        v.name,
        v.category,
        v.subject,
        v.body_html,
        v.sort_order,
        v.event_id,
        acteur
    )
    .fetch_one(conn)
    .await?;
    Ok(id)
}

pub async fn modifier(conn: &mut PgConnection, id: Uuid, v: &ValeursTexte<'_>) -> Result<()> {
    sqlx::query!(
        r#"UPDATE engagement.mail_texts
              SET name = $2, category = $3, subject = $4, body_html = $5,
                  sort_order = $6, event_id = $7
            WHERE id = $1"#,
        id,
        v.name,
        v.category,
        v.subject,
        v.body_html,
        v.sort_order,
        v.event_id
    )
    .execute(conn)
    .await?;
    Ok(())
}

/// Les envois qui le citaient gardent leur objet et leur corps (`ON DELETE SET
/// NULL` sur `mailings.mail_text_id`).
pub async fn supprimer(conn: &mut PgConnection, id: Uuid) -> Result<()> {
    sqlx::query!("DELETE FROM engagement.mail_texts WHERE id = $1", id)
        .execute(conn)
        .await?;
    Ok(())
}
