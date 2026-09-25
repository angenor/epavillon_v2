//! La FAQ au back-office : toutes les entrées, brouillons compris.

use kernel::error::Result;
use serde_json::Value;
use sqlx::postgres::PgConnection;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::domain::admin_savoir::{
    AdminFaqFeedback, AdminFaqReport, AdminFaqRow, AdminKnowledgeRef,
};
use crate::domain::savoir::KnowledgeStatus;

pub async fn rubrique_id(conn: &mut PgConnection, code: &str) -> Result<Option<Uuid>> {
    Ok(sqlx::query_scalar!(
        "SELECT id FROM reference.taxonomy_terms WHERE taxonomy_code = 'faq_section' AND code = $1",
        code
    )
    .fetch_optional(conn)
    .await?)
}

/// `q` se cherche par trigrammes sur la question française normalisée.
pub async fn lignes(
    conn: &mut PgConnection,
    locale: &str,
    q: Option<&str>,
    rubrique: Option<&str>,
    statut: Option<&str>,
) -> Result<Vec<AdminFaqRow>> {
    let lignes = sqlx::query!(
        r#"SELECT f.id, t.code AS "section_code!", platform.t(f.question, $1) AS "question!",
                  f.status::text AS "status!", f.verified_on, f.answer IS NOT NULL AS "has_answer!",
                  f.first_published_at, f.updated_at,
                  (SELECT count(*) FROM negotiation.faq_reports r
                    WHERE r.entry_id = f.id AND r.status = 'open') AS "open_reports!"
             FROM negotiation.faq_entries f
             JOIN reference.taxonomy_terms t ON t.id = f.section_term_id
             LEFT JOIN LATERAL (SELECT platform.normalize_label($2) AS n) q ON true
            WHERE ($2::text IS NULL OR q.n IS NULL
                   OR f.question_norm LIKE '%' || q.n || '%' OR f.question_norm % q.n)
              AND ($3::text IS NULL OR t.code = $3)
              AND ($4::text IS NULL OR f.status::text = $4)
            ORDER BY CASE WHEN q.n IS NULL THEN 0 ELSE similarity(f.question_norm, q.n) END DESC,
                     t.sort_order, f.updated_at DESC, f.id"#,
        locale,
        q,
        rubrique,
        statut
    )
    .fetch_all(conn)
    .await?;
    Ok(lignes
        .into_iter()
        .map(|l| AdminFaqRow {
            id: l.id,
            section_code: l.section_code,
            question: l.question,
            status: KnowledgeStatus::depuis(&l.status),
            verified_on: l.verified_on,
            has_answer: l.has_answer,
            open_reports: l.open_reports,
            first_published_at: l.first_published_at,
            updated_at: l.updated_at,
        })
        .collect())
}

pub struct Fiche {
    pub id: Uuid,
    pub section_code: String,
    pub question: Value,
    pub answer: Option<Value>,
    pub status: String,
    pub verified_on: Option<Date>,
    pub verified_by_name: Option<String>,
    pub editorial_rank: Option<i16>,
    pub origin_question_id: Option<Uuid>,
    pub first_published_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

pub async fn fiche(conn: &mut PgConnection, id: Uuid) -> Result<Option<Fiche>> {
    Ok(sqlx::query_as!(
        Fiche,
        r#"SELECT f.id, t.code AS "section_code!", f.question::jsonb AS "question!: Value",
                  f.answer::jsonb AS "answer: Value", f.status::text AS "status!", f.verified_on,
                  CASE WHEN p.id IS NULL THEN NULL
                       ELSE concat_ws(' ', p.first_name, p.last_name) END AS verified_by_name,
                  f.editorial_rank, f.origin_question_id, f.first_published_at,
                  f.created_at, f.updated_at
             FROM negotiation.faq_entries f
             JOIN reference.taxonomy_terms t ON t.id = f.section_term_id
             LEFT JOIN identity.people p ON p.id = f.verified_by
            WHERE f.id = $1"#,
        id
    )
    .fetch_optional(conn)
    .await?)
}

/// Le statut, ligne verrouillée : la transition se décide sur l'état lu.
pub async fn statut_verrouille(conn: &mut PgConnection, id: Uuid) -> Result<Option<String>> {
    Ok(sqlx::query_scalar!(
        r#"SELECT status::text AS "s!" FROM negotiation.faq_entries WHERE id = $1 FOR UPDATE"#,
        id
    )
    .fetch_optional(conn)
    .await?)
}

pub async fn liees(
    conn: &mut PgConnection,
    locale: &str,
    id: Uuid,
) -> Result<Vec<AdminKnowledgeRef>> {
    let lignes = sqlx::query!(
        r#"SELECT f.id, platform.t(f.question, $2) AS "label!", f.status::text AS "status!"
             FROM negotiation.faq_related r
             JOIN negotiation.faq_entries f ON f.id = r.related_id
            WHERE r.entry_id = $1
            ORDER BY r.sort_order, f.id"#,
        id,
        locale
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
        "DELETE FROM negotiation.faq_related WHERE entry_id = $1",
        id
    )
    .execute(&mut *conn)
    .await?;
    sqlx::query!(
        "INSERT INTO negotiation.faq_related (entry_id, related_id, sort_order)
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

/// Les voix comptées, jamais leurs auteurs.
pub async fn retours(conn: &mut PgConnection, id: Uuid) -> Result<AdminFaqFeedback> {
    let l = sqlx::query!(
        r#"SELECT count(*) FILTER (WHERE helpful) AS "helpful!",
                  count(*) FILTER (WHERE NOT helpful) AS "not_helpful!",
                  count(*) FILTER (WHERE missing_reason = 'too_vague') AS "too_vague!",
                  count(*) FILTER (WHERE missing_reason = 'off_topic') AS "off_topic!",
                  count(*) FILTER (WHERE missing_reason = 'outdated') AS "outdated!"
             FROM negotiation.faq_feedback
            WHERE entry_id = $1"#,
        id
    )
    .fetch_one(conn)
    .await?;
    Ok(AdminFaqFeedback {
        helpful: l.helpful,
        not_helpful: l.not_helpful,
        too_vague: l.too_vague,
        off_topic: l.off_topic,
        outdated: l.outdated,
    })
}

/// Sans `reporter_id` ni `handled_by` (R9).
pub async fn signalements(conn: &mut PgConnection, id: Uuid) -> Result<Vec<AdminFaqReport>> {
    Ok(sqlx::query_as!(
        AdminFaqReport,
        r#"SELECT id, reasons, from_feedback, details, status::text AS "status!", outcome,
                  created_at, handled_at
             FROM negotiation.faq_reports
            WHERE entry_id = $1
            ORDER BY status, created_at DESC, id"#,
        id
    )
    .fetch_all(conn)
    .await?)
}

pub struct Nouvelle<'a> {
    pub section_id: Uuid,
    pub question: &'a Value,
    pub answer: Option<&'a Value>,
    pub editorial_rank: Option<i16>,
    pub auteur: Uuid,
}

pub async fn creer(conn: &mut PgConnection, n: &Nouvelle<'_>) -> Result<Uuid> {
    Ok(sqlx::query_scalar!(
        "INSERT INTO negotiation.faq_entries
             (section_term_id, question, answer, editorial_rank, created_by)
         VALUES ($1, $2::jsonb::platform.i18n_text, $3::jsonb::platform.i18n_text, $4, $5)
         RETURNING id",
        n.section_id,
        n.question,
        n.answer,
        n.editorial_rank,
        n.auteur
    )
    .fetch_one(conn)
    .await?)
}

/// `Some(x)` : le champ prend `x` ; `None` : inchangé ; `Some(None)` vide.
#[derive(Default)]
pub struct Modification<'a> {
    pub section_id: Option<Uuid>,
    pub question: Option<&'a Value>,
    pub answer: Option<Option<&'a Value>>,
    pub editorial_rank: Option<Option<i16>>,
}

pub async fn modifier(conn: &mut PgConnection, id: Uuid, m: &Modification<'_>) -> Result<()> {
    sqlx::query!(
        "UPDATE negotiation.faq_entries SET
             section_term_id = COALESCE($2, section_term_id),
             question = COALESCE($3::jsonb::platform.i18n_text, question),
             answer = CASE WHEN $4 THEN $5::jsonb::platform.i18n_text ELSE answer END,
             editorial_rank = CASE WHEN $6 THEN $7 ELSE editorial_rank END
          WHERE id = $1",
        id,
        m.section_id,
        m.question,
        m.answer.is_some(),
        m.answer.flatten(),
        m.editorial_rank.is_some(),
        m.editorial_rank.flatten()
    )
    .execute(conn)
    .await?;
    Ok(())
}

/// « À revoir » revient publiée : la vérification est ce qu'elle attendait.
pub async fn verifier(conn: &mut PgConnection, id: Uuid, le: Date, expert: Uuid) -> Result<()> {
    sqlx::query!(
        "UPDATE negotiation.faq_entries
            SET verified_on = $2, verified_by = $3,
                status = CASE WHEN status = 'to_review' THEN 'published' ELSE status END
          WHERE id = $1",
        id,
        le,
        expert
    )
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn poser_le_statut(conn: &mut PgConnection, id: Uuid, statut: &str) -> Result<()> {
    sqlx::query!(
        "UPDATE negotiation.faq_entries SET status = $2::text::negotiation.knowledge_status
          WHERE id = $1",
        id,
        statut
    )
    .execute(conn)
    .await?;
    Ok(())
}

/// Une entrée publiée une fois est refusée par la base (`ck_faq_entries_undeletable`).
pub async fn supprimer(conn: &mut PgConnection, id: Uuid) -> Result<u64> {
    Ok(
        sqlx::query!("DELETE FROM negotiation.faq_entries WHERE id = $1", id)
            .execute(conn)
            .await?
            .rows_affected(),
    )
}

/// Aujourd'hui, heure de Paris : le jour que l'expert a sous les yeux.
pub async fn aujourdhui(conn: &mut PgConnection) -> Result<Date> {
    Ok(
        sqlx::query_scalar!(r#"SELECT (now() AT TIME ZONE 'Europe/Paris')::date AS "d!""#)
            .fetch_one(conn)
            .await?,
    )
}
