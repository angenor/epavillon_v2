//! Les signalements d'une autrice. Le doublon en attente et le rejeu d'une même
//! référence sont tenus par la base ; le code traduit ses refus (principe VIII).

use kernel::error::{ApiError, ErrorCode, Result};
use sqlx::postgres::PgConnection;
use uuid::Uuid;

use crate::domain::reports::{
    statut_vu, MyReport, MyReportStatus, NouveauSignalement, ReportReason, ReportedSession,
};

const EN_ATTENTE: &str = "ux_session_reports_pending";

/// Une session importée de cette édition.
pub async fn session_de_ledition(
    conn: &mut PgConnection,
    event_id: Uuid,
    meeting_id: Uuid,
) -> Result<bool> {
    let existe = sqlx::query_scalar!(
        r#"SELECT EXISTS (
               SELECT 1 FROM negotiation.meetings
                WHERE id = $1 AND event_id = $2
                  AND kind = 'negotiation_session' AND source_key IS NOT NULL
           ) AS "existe!""#,
        meeting_id,
        event_id
    )
    .fetch_one(conn)
    .await?;
    Ok(existe)
}

pub async fn theme(conn: &mut PgConnection, code: &str) -> Result<Option<Uuid>> {
    Ok(sqlx::query_scalar!(
        "SELECT id FROM reference.taxonomy_terms
          WHERE taxonomy_code = 'negotiation_theme' AND code = $1 AND is_active",
        code
    )
    .fetch_optional(conn)
    .await?)
}

/// Écrit le signalement. `None` : la référence existait déjà (rejeu concurrent).
pub async fn inserer(
    conn: &mut PgConnection,
    author_id: Uuid,
    client_ref: Uuid,
    event_id: Uuid,
    s: &NouveauSignalement,
    theme_term_id: Option<Uuid>,
) -> Result<Option<Uuid>> {
    let insere = sqlx::query_scalar!(
        "INSERT INTO negotiation.session_reports
             (author_id, client_ref, event_id, meeting_id, reason, proposed_start,
              proposed_venue, what, proposed_day, theme_term_id, detail)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
         ON CONFLICT ON CONSTRAINT ux_session_reports_client_ref DO NOTHING
         RETURNING id",
        author_id,
        client_ref,
        event_id,
        s.session_id,
        s.reason.as_db(),
        s.proposed_start,
        s.proposed_venue,
        s.what,
        s.proposed_day,
        theme_term_id,
        s.detail
    )
    .fetch_optional(conn)
    .await;

    match insere {
        Ok(id) => Ok(id),
        Err(e) if kernel::pg_error::constraint(&e) == Some(EN_ATTENTE) => {
            Err(ApiError::new(ErrorCode::NegotiationReportDuplicate).field("session_id"))
        }
        Err(e) => Err(e.into()),
    }
}

pub struct Filtre {
    pub client_ref: Option<Uuid>,
    pub event_id: Option<Uuid>,
}

/// Les signalements de l'autrice, plus récent d'abord.
pub async fn lister(
    conn: &mut PgConnection,
    author_id: Uuid,
    filtre: Filtre,
) -> Result<Vec<MyReport>> {
    let lignes = sqlx::query!(
        r#"SELECT r.id, r.client_ref, r.reason, r.what, r.proposed_start, r.proposed_venue,
                  r.proposed_day, r.detail, r.status::text AS "status!",
                  (r.published_at IS NOT NULL) AS "publie!",
                  r.submitted_at, r.decided_at, r.reject_reason, r.reject_detail,
                  m.id AS "session_id?", m.title_original AS "title_en?",
                  tr.text_fr AS "title_fr?", m.start_at AS "start_at?", m.venue_label,
                  th.code AS "theme?", nm.id AS "network_meeting_id?"
             FROM negotiation.session_reports r
             LEFT JOIN negotiation.meetings m ON m.id = r.meeting_id
             LEFT JOIN negotiation.title_translations tr ON tr.source_text = m.title_original
             LEFT JOIN reference.taxonomy_terms th ON th.id = r.theme_term_id
             LEFT JOIN negotiation.network_meetings nm
                    ON nm.id = r.network_meeting_id AND nm.withdrawn_at IS NULL
                   AND r.published_at IS NOT NULL
            WHERE r.author_id = $1
              AND ($2::uuid IS NULL OR r.client_ref = $2)
              AND ($3::uuid IS NULL OR r.event_id = $3)
            ORDER BY r.submitted_at DESC, r.id DESC"#,
        author_id,
        filtre.client_ref,
        filtre.event_id
    )
    .fetch_all(conn)
    .await?;

    Ok(lignes
        .into_iter()
        .filter_map(|l| {
            let status = statut_vu(&l.status, l.publie);
            let session = match (l.session_id, l.title_en, l.start_at) {
                (Some(id), Some(title_en), Some(start_at)) => Some(ReportedSession {
                    id,
                    title_en,
                    title_fr: l.title_fr,
                    start_at,
                    venue: l.venue_label,
                }),
                _ => None,
            };
            Some(MyReport {
                id: l.id,
                client_ref: l.client_ref,
                reason: ReportReason::from_db(&l.reason)?,
                session,
                what: l.what,
                proposed_start: l.proposed_start,
                proposed_venue: l.proposed_venue,
                day: l.proposed_day,
                theme: l.theme,
                network_meeting_id: l.network_meeting_id,
                detail: l.detail,
                status,
                submitted_at: l.submitted_at,
                decided_at: l.decided_at.filter(|_| status != MyReportStatus::Submitted),
                reject_reason: l.reject_reason,
                reject_detail: l.reject_detail,
            })
        })
        .collect())
}

/// Les signalements qui attendent une décision, toutes éditions confondues.
pub async fn a_relire(conn: &mut PgConnection) -> Result<i64> {
    Ok(sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM negotiation.session_reports WHERE status = 'submitted'"#
    )
    .fetch_one(conn)
    .await?)
}
