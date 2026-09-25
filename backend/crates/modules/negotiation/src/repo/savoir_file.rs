//! La file des experts, part « signalements ». Aucune requête ne lit
//! `reporter_id` ni `handled_by` pour le rendre (R9).

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use time::Date;
use uuid::Uuid;

use crate::domain::admin_file::ExpertQueueCounts;
use crate::domain::admin_savoir::AdminFaqReport;

pub async fn comptes(conn: &mut PgConnection) -> Result<ExpertQueueCounts> {
    let c = sqlx::query!(
        r#"SELECT (SELECT count(*) FROM negotiation.faq_reports WHERE status = 'open') AS "reports!",
                  (SELECT count(*) FROM negotiation.expert_questions WHERE status = 'pending') AS "questions!",
                  (SELECT count(*) FROM negotiation.glossary_proposals WHERE status = 'pending') AS "proposals!""#
    )
    .fetch_one(conn)
    .await?;
    Ok(ExpertQueueCounts {
        reports: c.reports,
        questions: c.questions,
        proposals: c.proposals,
    })
}

pub struct Ouvert {
    pub entry_id: Uuid,
    pub question: String,
    pub status: String,
    pub verified_on: Option<Date>,
    pub report: AdminFaqReport,
}

/// Les signalements ouverts, du plus ancien au plus récent.
pub async fn ouverts(conn: &mut PgConnection, locale: &str) -> Result<Vec<Ouvert>> {
    let lignes = sqlx::query!(
        r#"SELECT r.id, r.entry_id, r.reasons, r.from_feedback, r.details,
                  r.status::text AS "status!", r.outcome, r.created_at, r.handled_at,
                  platform.t(f.question, $1) AS "question!", f.status::text AS "entry_status!",
                  f.verified_on
             FROM negotiation.faq_reports r
             JOIN negotiation.faq_entries f ON f.id = r.entry_id
            WHERE r.status = 'open'
            ORDER BY r.created_at, r.id"#,
        locale
    )
    .fetch_all(conn)
    .await?;
    Ok(lignes
        .into_iter()
        .map(|l| Ouvert {
            entry_id: l.entry_id,
            question: l.question,
            status: l.entry_status,
            verified_on: l.verified_on,
            report: AdminFaqReport {
                id: l.id,
                reasons: l.reasons,
                from_feedback: l.from_feedback,
                details: l.details,
                status: l.status,
                outcome: l.outcome,
                created_at: l.created_at,
                handled_at: l.handled_at,
            },
        })
        .collect())
}

pub async fn statut(conn: &mut PgConnection, id: Uuid) -> Result<Option<String>> {
    Ok(sqlx::query_scalar!(
        r#"SELECT status::text AS "status!" FROM negotiation.faq_reports WHERE id = $1 FOR UPDATE"#,
        id
    )
    .fetch_optional(conn)
    .await?)
}

pub async fn clore(
    conn: &mut PgConnection,
    id: Uuid,
    issue: &str,
    expert: Uuid,
) -> Result<AdminFaqReport> {
    Ok(sqlx::query_as!(
        AdminFaqReport,
        r#"UPDATE negotiation.faq_reports
              SET status = 'closed', outcome = $2, handled_by = $3, handled_at = now()
            WHERE id = $1
        RETURNING id, reasons, from_feedback, details, status::text AS "status!", outcome,
                  created_at, handled_at"#,
        id,
        issue,
        expert
    )
    .fetch_one(conn)
    .await?)
}
