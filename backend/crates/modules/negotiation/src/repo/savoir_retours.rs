//! Retours « Oui / Non » et signalements « Dépassé ou faux » : l'auteur est
//! gardé pour l'unicité et le plafond, jamais rendu à un expert (R9).

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use uuid::Uuid;

use crate::domain::savoir_retours::{FaqFeedback, FaqReportReceipt};

pub async fn servie(conn: &mut PgConnection, entry_id: Uuid) -> Result<bool> {
    Ok(sqlx::query_scalar!(
        r#"SELECT EXISTS (SELECT 1 FROM negotiation.faq_entries
                           WHERE id = $1 AND status <> 'draft') AS "existe!""#,
        entry_id
    )
    .fetch_one(conn)
    .await?)
}

/// Une voix par personne et par entrée : la dernière écrase la précédente.
pub async fn voter(
    conn: &mut PgConnection,
    entry_id: Uuid,
    personne: Uuid,
    helpful: bool,
    motif: Option<&str>,
) -> Result<FaqFeedback> {
    Ok(sqlx::query_as!(
        FaqFeedback,
        "INSERT INTO negotiation.faq_feedback (entry_id, person_id, helpful, missing_reason)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (entry_id, person_id)
         DO UPDATE SET helpful = EXCLUDED.helpful, missing_reason = EXCLUDED.missing_reason
         RETURNING entry_id, helpful, missing_reason, updated_at",
        entry_id,
        personne,
        helpful,
        motif
    )
    .fetch_one(conn)
    .await?)
}

/// « Dépassée ou fausse » depuis le retour : un signalement sans motif, une
/// fois par personne et par entrée, même clos depuis.
pub async fn signaler_depuis_le_retour(
    conn: &mut PgConnection,
    entry_id: Uuid,
    personne: Uuid,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO negotiation.faq_reports (entry_id, reporter_id, client_ref, from_feedback)
         VALUES ($1, $2, platform.uuid_v7(), true)
         ON CONFLICT (entry_id, reporter_id) WHERE from_feedback DO NOTHING",
        entry_id,
        personne
    )
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn mes_voix(conn: &mut PgConnection, personne: Uuid) -> Result<Vec<FaqFeedback>> {
    Ok(sqlx::query_as!(
        FaqFeedback,
        "SELECT f.entry_id, f.helpful, f.missing_reason, f.updated_at
           FROM negotiation.faq_feedback f
           JOIN negotiation.faq_entries e ON e.id = f.entry_id
          WHERE f.person_id = $1 AND e.status <> 'draft'
          ORDER BY f.entry_id",
        personne
    )
    .fetch_all(conn)
    .await?)
}

/// Sérialise les envois d'une même personne : le plafond et le rejeu se lisent
/// sans course.
pub async fn verrouiller(conn: &mut PgConnection, personne: Uuid) -> Result<()> {
    sqlx::query!(
        "SELECT pg_advisory_xact_lock(hashtextextended('faq_reports:' || $1, 0))",
        personne.to_string()
    )
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn recu(
    conn: &mut PgConnection,
    personne: Uuid,
    client_ref: Uuid,
) -> Result<Option<FaqReportReceipt>> {
    Ok(sqlx::query_as!(
        FaqReportReceipt,
        "SELECT id, entry_id, client_ref, created_at FROM negotiation.faq_reports
          WHERE reporter_id = $1 AND client_ref = $2",
        personne,
        client_ref
    )
    .fetch_optional(conn)
    .await?)
}

/// Les signalements « Dépassé ou faux » du jour de Paris ; ceux venus d'un
/// retour ne comptent pas, ils sont bornés à un par entrée.
pub async fn envoyes_aujourdhui(conn: &mut PgConnection, personne: Uuid) -> Result<i64> {
    Ok(sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM negotiation.faq_reports
            WHERE reporter_id = $1 AND NOT from_feedback
              AND created_at >= (date_trunc('day', now() AT TIME ZONE 'Europe/Paris')
                                 AT TIME ZONE 'Europe/Paris')"#,
        personne
    )
    .fetch_one(conn)
    .await?)
}

pub async fn signaler(
    conn: &mut PgConnection,
    entry_id: Uuid,
    personne: Uuid,
    client_ref: Uuid,
    motifs: &[String],
    details: Option<&str>,
) -> Result<FaqReportReceipt> {
    Ok(sqlx::query_as!(
        FaqReportReceipt,
        "INSERT INTO negotiation.faq_reports (entry_id, reporter_id, client_ref, reasons, details)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING id, entry_id, client_ref, created_at",
        entry_id,
        personne,
        client_ref,
        motifs,
        details
    )
    .fetch_one(conn)
    .await?)
}
