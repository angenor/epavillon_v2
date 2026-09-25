//! Le parcours « Ma première COP » au back-office : groupes et étapes, publiés
//! ou non.

use kernel::error::Result;
use serde_json::Value;
use sqlx::postgres::PgConnection;
use uuid::Uuid;

use crate::domain::admin_savoir::{AdminPathwayGroup, AdminPathwayLink, AdminPathwayStep};

pub async fn groupes(conn: &mut PgConnection) -> Result<Vec<AdminPathwayGroup>> {
    let lignes = sqlx::query!(
        r#"SELECT id, label::jsonb AS "label!: Value", sort_order, is_published
             FROM negotiation.pathway_groups
            ORDER BY sort_order, id"#
    )
    .fetch_all(conn)
    .await?;
    Ok(lignes
        .into_iter()
        .map(|l| AdminPathwayGroup {
            id: l.id,
            label: l.label,
            sort_order: l.sort_order,
            is_published: l.is_published,
            steps: Vec::new(),
        })
        .collect())
}

/// Chaque étape, la désignation de sa cible résolue et ses coches comptées.
pub async fn etapes(conn: &mut PgConnection, locale: &str) -> Result<Vec<AdminPathwayStep>> {
    let lignes = sqlx::query!(
        r#"SELECT s.id, s.group_id, s.label::jsonb AS "label!: Value",
                  s.detail::jsonb AS "detail: Value", s.origin_label::jsonb AS "origin_label: Value",
                  s.link_kind, COALESCE(s.link_document_id, s.link_faq_id, s.link_glossary_id) AS link_target,
                  CASE s.link_kind WHEN 'document' THEN platform.t(d.title, $1)
                                   WHEN 'faq' THEN platform.t(f.question, $1)
                                   WHEN 'glossary' THEN g.term END AS target_label,
                  s.link_page, s.link_section, s.link_label::jsonb AS "link_label: Value",
                  s.sort_order, s.is_published,
                  (SELECT count(*) FROM negotiation.pathway_checks c WHERE c.step_id = s.id) AS "checks!"
             FROM negotiation.pathway_steps s
             LEFT JOIN negotiation.documents d ON d.id = s.link_document_id
             LEFT JOIN negotiation.faq_entries f ON f.id = s.link_faq_id
             LEFT JOIN negotiation.glossary_entries g ON g.id = s.link_glossary_id
            ORDER BY s.sort_order, s.id"#,
        locale
    )
    .fetch_all(conn)
    .await?;
    Ok(lignes
        .into_iter()
        .map(|l| AdminPathwayStep {
            link: match (l.link_kind, l.link_target) {
                (Some(kind), Some(target_id)) => Some(AdminPathwayLink {
                    kind,
                    target_id,
                    target_label: l.target_label,
                    page: l.link_page,
                    section: l.link_section,
                    label: l.link_label,
                }),
                _ => None,
            },
            id: l.id,
            group_id: l.group_id,
            label: l.label,
            detail: l.detail,
            origin_label: l.origin_label,
            sort_order: l.sort_order,
            is_published: l.is_published,
            checks: l.checks,
        })
        .collect())
}

pub async fn creer_groupe(conn: &mut PgConnection, label: &Value, publie: bool) -> Result<Uuid> {
    Ok(sqlx::query_scalar!(
        "INSERT INTO negotiation.pathway_groups (label, is_published, sort_order)
         VALUES ($1::jsonb::platform.i18n_text, $2,
                 (SELECT COALESCE(max(sort_order) + 1, 0) FROM negotiation.pathway_groups))
         RETURNING id",
        label,
        publie
    )
    .fetch_one(conn)
    .await?)
}

pub async fn modifier_groupe(
    conn: &mut PgConnection,
    id: Uuid,
    label: Option<&Value>,
    publie: Option<bool>,
) -> Result<u64> {
    Ok(sqlx::query!(
        "UPDATE negotiation.pathway_groups
            SET label = COALESCE($2::jsonb::platform.i18n_text, label),
                is_published = COALESCE($3, is_published)
          WHERE id = $1",
        id,
        label,
        publie
    )
    .execute(conn)
    .await?
    .rows_affected())
}

/// Un groupe qui porte des étapes est refusé par la clé étrangère
/// (`NEGOTIATION_PATHWAY_GROUP_NOT_EMPTY`).
pub async fn supprimer_groupe(conn: &mut PgConnection, id: Uuid) -> Result<u64> {
    Ok(
        sqlx::query!("DELETE FROM negotiation.pathway_groups WHERE id = $1", id)
            .execute(conn)
            .await?
            .rows_affected(),
    )
}

/// Le lien d'une étape, colonne par colonne. `kind` nul : sans lien.
#[derive(Default, Clone)]
pub struct Lien<'a> {
    pub kind: Option<&'a str>,
    pub document_id: Option<Uuid>,
    pub page: Option<i16>,
    pub section: Option<&'a str>,
    pub faq_id: Option<Uuid>,
    pub glossary_id: Option<Uuid>,
    pub label: Option<&'a Value>,
}

pub struct Etape<'a> {
    pub group_id: Uuid,
    pub label: &'a Value,
    pub detail: Option<&'a Value>,
    pub origin_label: Option<&'a Value>,
    pub lien: Lien<'a>,
    pub publie: bool,
}

pub async fn creer_etape(conn: &mut PgConnection, e: &Etape<'_>) -> Result<Uuid> {
    Ok(sqlx::query_scalar!(
        "INSERT INTO negotiation.pathway_steps
             (group_id, label, detail, origin_label, link_kind, link_document_id, link_page,
              link_section, link_faq_id, link_glossary_id, link_label, is_published, sort_order)
         VALUES ($1, $2::jsonb::platform.i18n_text, $3::jsonb::platform.i18n_text,
                 $4::jsonb::platform.i18n_text, $5, $6, $7, $8, $9, $10,
                 $11::jsonb::platform.i18n_text, $12,
                 (SELECT COALESCE(max(sort_order) + 1, 0) FROM negotiation.pathway_steps
                   WHERE group_id = $1))
         RETURNING id",
        e.group_id,
        e.label,
        e.detail,
        e.origin_label,
        e.lien.kind,
        e.lien.document_id,
        e.lien.page,
        e.lien.section,
        e.lien.faq_id,
        e.lien.glossary_id,
        e.lien.label,
        e.publie
    )
    .fetch_one(conn)
    .await?)
}

/// Le groupe de l'étape, ligne verrouillée.
pub async fn groupe_de(conn: &mut PgConnection, id: Uuid) -> Result<Option<Uuid>> {
    Ok(sqlx::query_scalar!(
        "SELECT group_id FROM negotiation.pathway_steps WHERE id = $1 FOR UPDATE",
        id
    )
    .fetch_optional(conn)
    .await?)
}

#[derive(Default)]
pub struct ModificationEtape<'a> {
    /// Un autre groupe : l'étape passe à sa fin.
    pub group_id: Option<Uuid>,
    pub label: Option<&'a Value>,
    pub detail: Option<Option<&'a Value>>,
    pub origin_label: Option<Option<&'a Value>>,
    pub lien: Option<Lien<'a>>,
    pub publie: Option<bool>,
}

pub async fn modifier_etape(
    conn: &mut PgConnection,
    id: Uuid,
    m: &ModificationEtape<'_>,
) -> Result<()> {
    let l = m.lien.clone().unwrap_or_default();
    sqlx::query!(
        "UPDATE negotiation.pathway_steps SET
             sort_order = CASE WHEN $2::uuid IS NOT NULL AND $2 <> group_id
                               THEN (SELECT COALESCE(max(sort_order) + 1, 0)
                                       FROM negotiation.pathway_steps WHERE group_id = $2)
                               ELSE sort_order END,
             group_id = COALESCE($2, group_id),
             label = COALESCE($3::jsonb::platform.i18n_text, label),
             detail = CASE WHEN $4 THEN $5::jsonb::platform.i18n_text ELSE detail END,
             origin_label = CASE WHEN $6 THEN $7::jsonb::platform.i18n_text ELSE origin_label END,
             link_kind = CASE WHEN $8 THEN $9 ELSE link_kind END,
             link_document_id = CASE WHEN $8 THEN $10 ELSE link_document_id END,
             link_page = CASE WHEN $8 THEN $11 ELSE link_page END,
             link_section = CASE WHEN $8 THEN $12 ELSE link_section END,
             link_faq_id = CASE WHEN $8 THEN $13 ELSE link_faq_id END,
             link_glossary_id = CASE WHEN $8 THEN $14 ELSE link_glossary_id END,
             link_label = CASE WHEN $8 THEN $15::jsonb::platform.i18n_text ELSE link_label END,
             is_published = COALESCE($16, is_published)
          WHERE id = $1",
        id,
        m.group_id,
        m.label,
        m.detail.is_some(),
        m.detail.flatten(),
        m.origin_label.is_some(),
        m.origin_label.flatten(),
        m.lien.is_some(),
        l.kind,
        l.document_id,
        l.page,
        l.section,
        l.faq_id,
        l.glossary_id,
        l.label,
        m.publie
    )
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn est_cochee(conn: &mut PgConnection, id: Uuid) -> Result<bool> {
    Ok(sqlx::query_scalar!(
        r#"SELECT EXISTS (SELECT 1 FROM negotiation.pathway_checks WHERE step_id = $1) AS "e!""#,
        id
    )
    .fetch_one(conn)
    .await?)
}

pub async fn supprimer_etape(conn: &mut PgConnection, id: Uuid) -> Result<u64> {
    Ok(
        sqlx::query!("DELETE FROM negotiation.pathway_steps WHERE id = $1", id)
            .execute(conn)
            .await?
            .rows_affected(),
    )
}

pub async fn ranger_le_groupe(conn: &mut PgConnection, id: Uuid, rang: i16) -> Result<u64> {
    Ok(sqlx::query!(
        "UPDATE negotiation.pathway_groups SET sort_order = $2 WHERE id = $1",
        id,
        rang
    )
    .execute(conn)
    .await?
    .rows_affected())
}

pub async fn ranger_l_etape(
    conn: &mut PgConnection,
    id: Uuid,
    groupe: Uuid,
    rang: i16,
) -> Result<u64> {
    Ok(sqlx::query!(
        "UPDATE negotiation.pathway_steps SET group_id = $2, sort_order = $3 WHERE id = $1",
        id,
        groupe,
        rang
    )
    .execute(conn)
    .await?
    .rows_affected())
}
