//! Le lexique au back-office : toutes les entrées, brouillons compris.

use kernel::error::Result;
use serde_json::Value;
use sqlx::postgres::PgConnection;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::admin_savoir::{AdminGlossaryRow, AdminKnowledgeRef};
use crate::domain::savoir::KnowledgeStatus;

pub async fn famille_id(conn: &mut PgConnection, code: &str) -> Result<Option<Uuid>> {
    Ok(sqlx::query_scalar!(
        "SELECT id FROM reference.taxonomy_terms WHERE taxonomy_code = 'glossary_family' AND code = $1",
        code
    )
    .fetch_optional(conn)
    .await?)
}

/// `q` se cherche par trigrammes sur le terme, et exactement sur l'acronyme.
pub async fn lignes(
    conn: &mut PgConnection,
    locale: &str,
    q: Option<&str>,
    famille: Option<&str>,
    statut: Option<&str>,
) -> Result<Vec<AdminGlossaryRow>> {
    let lignes = sqlx::query!(
        r#"SELECT g.id, g.slug, t.code AS "family_code!", g.term, g.acronym,
                  platform.t(g.translation, $1) AS "translation!", g.status::text AS "status!",
                  g.first_published_at, g.updated_at
             FROM negotiation.glossary_entries g
             JOIN reference.taxonomy_terms t ON t.id = g.family_term_id
             LEFT JOIN LATERAL (SELECT platform.normalize_label($2) AS n) q ON true
            WHERE ($2::text IS NULL OR q.n IS NULL
                   OR g.term_norm LIKE '%' || q.n || '%' OR g.term_norm % q.n
                   OR g.acronym_norm = q.n OR q.n = ANY (g.variants_norm))
              AND ($3::text IS NULL OR t.code = $3)
              AND ($4::text IS NULL OR g.status::text = $4)
            ORDER BY CASE WHEN q.n IS NULL THEN 0 ELSE similarity(g.term_norm, q.n) END DESC,
                     g.term_norm, g.id"#,
        locale,
        q,
        famille,
        statut
    )
    .fetch_all(conn)
    .await?;
    Ok(lignes
        .into_iter()
        .map(|l| AdminGlossaryRow {
            id: l.id,
            slug: l.slug,
            family_code: l.family_code,
            term: l.term,
            acronym: l.acronym,
            translation: l.translation,
            status: KnowledgeStatus::depuis(&l.status),
            first_published_at: l.first_published_at,
            updated_at: l.updated_at,
        })
        .collect())
}

pub struct Fiche {
    pub id: Uuid,
    pub slug: String,
    pub family_code: String,
    pub term: String,
    pub acronym: Option<String>,
    pub variants: Vec<String>,
    pub translation: Value,
    pub definition: Value,
    pub heard_in_room: Option<String>,
    pub status: String,
    pub first_published_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

pub async fn fiche(conn: &mut PgConnection, id: Uuid) -> Result<Option<Fiche>> {
    Ok(sqlx::query_as!(
        Fiche,
        r#"SELECT g.id, g.slug, t.code AS "family_code!", g.term, g.acronym, g.variants,
                  g.translation::jsonb AS "translation!: Value",
                  g.definition::jsonb AS "definition!: Value", g.heard_in_room,
                  g.status::text AS "status!", g.first_published_at, g.created_at, g.updated_at
             FROM negotiation.glossary_entries g
             JOIN reference.taxonomy_terms t ON t.id = g.family_term_id
            WHERE g.id = $1"#,
        id
    )
    .fetch_optional(conn)
    .await?)
}

pub async fn statut_verrouille(conn: &mut PgConnection, id: Uuid) -> Result<Option<String>> {
    Ok(sqlx::query_scalar!(
        r#"SELECT status::text AS "s!" FROM negotiation.glossary_entries WHERE id = $1 FOR UPDATE"#,
        id
    )
    .fetch_optional(conn)
    .await?)
}

pub async fn liees(conn: &mut PgConnection, id: Uuid) -> Result<Vec<AdminKnowledgeRef>> {
    let lignes = sqlx::query!(
        r#"SELECT g.id, g.term AS "label!", g.status::text AS "status!"
             FROM negotiation.glossary_related r
             JOIN negotiation.glossary_entries g ON g.id = r.related_id
            WHERE r.entry_id = $1
            ORDER BY r.sort_order, g.id"#,
        id
    )
    .fetch_all(conn)
    .await?;
    Ok(lignes
        .into_iter()
        .map(|l| AdminKnowledgeRef {
            id: l.id,
            label: l.label,
            status: KnowledgeStatus::depuis(&l.status),
        })
        .collect())
}

pub async fn remplacer_liees(conn: &mut PgConnection, id: Uuid, liees: &[Uuid]) -> Result<()> {
    sqlx::query!(
        "DELETE FROM negotiation.glossary_related WHERE entry_id = $1",
        id
    )
    .execute(&mut *conn)
    .await?;
    sqlx::query!(
        "INSERT INTO negotiation.glossary_related (entry_id, related_id, sort_order)
         SELECT $1, l.id, (l.rang - 1)::smallint
           FROM unnest($2::uuid[]) WITH ORDINALITY AS l(id, rang)
         ON CONFLICT DO NOTHING",
        id,
        liees
    )
    .execute(conn)
    .await?;
    Ok(())
}

pub struct Nouveau<'a> {
    pub family_id: Uuid,
    pub term: &'a str,
    pub acronym: Option<&'a str>,
    pub variants: &'a [String],
    pub translation: &'a Value,
    pub definition: &'a Value,
    pub heard_in_room: Option<&'a str>,
    pub auteur: Uuid,
}

/// Le `slug` vide est posé par `tg_glossary_slug` depuis le terme.
pub async fn creer(conn: &mut PgConnection, n: &Nouveau<'_>) -> Result<Uuid> {
    Ok(sqlx::query_scalar!(
        "INSERT INTO negotiation.glossary_entries
             (slug, family_term_id, term, acronym, variants, translation, definition,
              heard_in_room, created_by)
         VALUES ('', $1, $2, $3, $4, $5::jsonb::platform.i18n_text,
                 $6::jsonb::platform.i18n_text, $7, $8)
         RETURNING id",
        n.family_id,
        n.term,
        n.acronym,
        n.variants,
        n.translation,
        n.definition,
        n.heard_in_room,
        n.auteur
    )
    .fetch_one(conn)
    .await?)
}

#[derive(Default)]
pub struct Modification<'a> {
    pub family_id: Option<Uuid>,
    pub term: Option<&'a str>,
    pub acronym: Option<Option<&'a str>>,
    pub variants: Option<&'a [String]>,
    pub translation: Option<&'a Value>,
    pub definition: Option<&'a Value>,
    pub heard_in_room: Option<Option<&'a str>>,
}

pub async fn modifier(conn: &mut PgConnection, id: Uuid, m: &Modification<'_>) -> Result<()> {
    sqlx::query!(
        "UPDATE negotiation.glossary_entries SET
             family_term_id = COALESCE($2, family_term_id),
             term = COALESCE($3, term),
             acronym = CASE WHEN $4 THEN $5 ELSE acronym END,
             variants = COALESCE($6, variants),
             translation = COALESCE($7::jsonb::platform.i18n_text, translation),
             definition = COALESCE($8::jsonb::platform.i18n_text, definition),
             heard_in_room = CASE WHEN $9 THEN $10 ELSE heard_in_room END
          WHERE id = $1",
        id,
        m.family_id,
        m.term,
        m.acronym.is_some(),
        m.acronym.flatten(),
        m.variants,
        m.translation,
        m.definition,
        m.heard_in_room.is_some(),
        m.heard_in_room.flatten()
    )
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn poser_le_statut(conn: &mut PgConnection, id: Uuid, statut: &str) -> Result<()> {
    sqlx::query!(
        "UPDATE negotiation.glossary_entries SET status = $2::text::negotiation.knowledge_status
          WHERE id = $1",
        id,
        statut
    )
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn supprimer(conn: &mut PgConnection, id: Uuid) -> Result<u64> {
    Ok(
        sqlx::query!("DELETE FROM negotiation.glossary_entries WHERE id = $1", id)
            .execute(conn)
            .await?
            .rows_affected(),
    )
}
