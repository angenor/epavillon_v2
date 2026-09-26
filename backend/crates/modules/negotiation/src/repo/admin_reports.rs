//! La file et les décisions. Chaque décision est un `UPDATE` gardé par l'état
//! attendu : deux gestes simultanés, ou l'annulation contre la publication, se
//! départagent par le verrou de ligne, jamais par une lecture préalable (R3).

use kernel::error::{ApiError, ErrorCode, Result};
use serde_json::Value;
use sqlx::postgres::PgConnection;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::admin_reports::{Refus, ReportAuthor, ReportQueueItem, SourceNow};
use crate::domain::reports::{MyReport, MyReportStatus, ReportReason, ReportedSession};

const EN_ATTENTE: &str = "ux_session_reports_pending";

pub struct Lu {
    pub item: ReportQueueItem,
    pub du_jour: bool,
}

fn statut(valeur: &str) -> MyReportStatus {
    match valeur {
        "validated" => MyReportStatus::Validated,
        "rejected" => MyReportStatus::Rejected,
        _ => MyReportStatus::Submitted,
    }
}

/// Sans `report_id` : ce qui attend, et ce qui a été tranché aujourd'hui dans
/// le fuseau de la COP. Avec : ce signalement seul, quel que soit son état.
pub async fn lister(
    conn: &mut PgConnection,
    event_id: Option<Uuid>,
    report_id: Option<Uuid>,
    locale: &str,
) -> Result<Vec<Lu>> {
    let lignes = sqlx::query!(
        r#"SELECT r.id, r.client_ref, r.reason, r.what, r.proposed_start, r.proposed_venue,
                  r.proposed_day, r.detail, r.status::text AS "status!",
                  r.submitted_at, r.decided_at, r.reject_reason, r.reject_detail, r.published_at,
                  r.withdrawn_at,
                  m.id AS "session_id?", m.title_original AS "title_en?",
                  tr.text_fr AS "title_fr?", m.start_at AS "start_at?", m.end_at AS session_end,
                  m.venue_label, m.status::text AS "session_status?",
                  th.code AS "theme?", nm.id AS "network_meeting_id?",
                  CASE WHEN m.absent_reads = 0 THEN coalesce(i.last_success_at, m.last_read_at)
                       ELSE m.last_read_at END AS source_read_at,
                  a.display_name AS "author_name!",
                  platform.t(pays.name, $3) AS country,
                  d.display_name AS "decided_by?",
                  coalesce((r.decided_at AT TIME ZONE e.timezone::text)::date
                           = (now() AT TIME ZONE e.timezone::text)::date, false) AS "du_jour!"
             FROM negotiation.session_reports r
             JOIN event.events e ON e.id = r.event_id
             JOIN identity.people a ON a.id = r.author_id
             LEFT JOIN reference.countries pays ON pays.id = a.country_id
             LEFT JOIN identity.people d ON d.id = r.decided_by
             LEFT JOIN negotiation.meetings m ON m.id = r.meeting_id
             LEFT JOIN negotiation.official_imports i ON i.event_id = r.event_id
             LEFT JOIN negotiation.title_translations tr ON tr.source_text = m.title_original
             LEFT JOIN reference.taxonomy_terms th ON th.id = r.theme_term_id
             LEFT JOIN negotiation.network_meetings nm
                    ON nm.id = r.network_meeting_id AND nm.withdrawn_at IS NULL
            WHERE ($1::uuid IS NULL OR r.event_id = $1)
              AND ($2::uuid IS NULL OR r.id = $2)
              AND ($2::uuid IS NOT NULL OR r.status = 'submitted'
                   OR (r.decided_at AT TIME ZONE e.timezone::text)::date
                      = (now() AT TIME ZONE e.timezone::text)::date)
            ORDER BY r.submitted_at, r.id"#,
        event_id,
        report_id,
        locale
    )
    .fetch_all(conn)
    .await?;

    Ok(lignes
        .into_iter()
        .filter_map(|l| {
            let (session, source_now) = match (l.session_id, l.title_en, l.start_at) {
                (Some(id), Some(title_en), Some(start_at)) => (
                    Some(ReportedSession {
                        id,
                        title_en,
                        title_fr: l.title_fr,
                        start_at,
                        venue: l.venue_label.clone(),
                    }),
                    Some(SourceNow {
                        status: l.session_status.unwrap_or_default(),
                        start_at,
                        end_at: l.session_end,
                        venue: l.venue_label,
                        read_at: l.source_read_at,
                    }),
                ),
                _ => (None, None),
            };
            Some(Lu {
                du_jour: l.du_jour,
                item: ReportQueueItem {
                    report: MyReport {
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
                        status: statut(&l.status),
                        submitted_at: l.submitted_at,
                        decided_at: l.decided_at,
                        reject_reason: l.reject_reason,
                        reject_detail: l.reject_detail,
                    },
                    author: ReportAuthor {
                        name: l.author_name,
                        country: l.country,
                    },
                    source_now,
                    decided_by: l.decided_by,
                    published_at: l.published_at,
                    withdrawn_at: l.withdrawn_at,
                },
            })
        })
        .collect())
}

/// Le statut du signalement, ligne tenue jusqu'à la fin de la transaction.
pub async fn verrouiller(conn: &mut PgConnection, id: Uuid) -> Result<Option<String>> {
    Ok(sqlx::query_scalar!(
        r#"SELECT status::text AS "status!" FROM negotiation.session_reports
            WHERE id = $1 FOR UPDATE"#,
        id
    )
    .fetch_optional(conn)
    .await?)
}

pub struct Validation {
    pub decided_at: OffsetDateTime,
    /// Calculée par la base : une seule horloge décide de la publication.
    pub publier_a: OffsetDateTime,
}

pub async fn valider(
    conn: &mut PgConnection,
    id: Uuid,
    decideur: Uuid,
    source: Option<Value>,
) -> Result<Option<Validation>> {
    let ligne = sqlx::query!(
        r#"UPDATE negotiation.session_reports
              SET status = 'validated', decided_by = $2, decided_at = now(), source_snapshot = $3
            WHERE id = $1 AND status = 'submitted'
        RETURNING decided_at AS "decided_at!", now() + interval '30 seconds' AS "publier_a!""#,
        id,
        decideur,
        source
    )
    .fetch_optional(conn)
    .await?;
    Ok(ligne.map(|l| Validation {
        decided_at: l.decided_at,
        publier_a: l.publier_a,
    }))
}

/// Faux : déjà publié, retiré, ou pas validé. Un nouveau signalement en
/// attente de la même autrice sur la même session rend l'annulation impossible.
pub async fn annuler(conn: &mut PgConnection, id: Uuid) -> Result<bool> {
    let fait = sqlx::query!(
        "UPDATE negotiation.session_reports
            SET status = 'submitted', decided_by = NULL, decided_at = NULL, source_snapshot = NULL
          WHERE id = $1 AND status = 'validated' AND published_at IS NULL AND withdrawn_at IS NULL",
        id
    )
    .execute(conn)
    .await;
    match fait {
        Ok(r) => Ok(r.rows_affected() == 1),
        Err(e) if kernel::pg_error::constraint(&e) == Some(EN_ATTENTE) => {
            Err(ApiError::new(ErrorCode::NegotiationReportUndoExpired))
        }
        Err(e) => Err(e.into()),
    }
}

pub async fn refuser(
    conn: &mut PgConnection,
    id: Uuid,
    decideur: Uuid,
    refus: &Refus,
    source: Option<Value>,
) -> Result<bool> {
    let fait = sqlx::query!(
        "UPDATE negotiation.session_reports
            SET status = 'rejected', decided_by = $2, decided_at = now(), source_snapshot = $5,
                reject_reason = $3, reject_detail = $4
          WHERE id = $1 AND status = 'submitted'",
        id,
        decideur,
        refus.reason,
        refus.detail,
        source
    )
    .execute(conn)
    .await?;
    Ok(fait.rows_affected() == 1)
}

pub enum Retrait {
    Fait,
    DejaRetire,
    PasAffiche,
}

/// Retire un encart publié, et la réunion non annoncée qui en est née.
pub async fn retirer(conn: &mut PgConnection, id: Uuid) -> Result<Retrait> {
    let ligne = sqlx::query!(
        r#"SELECT published_at, withdrawn_at, network_meeting_id
             FROM negotiation.session_reports WHERE id = $1"#,
        id
    )
    .fetch_one(&mut *conn)
    .await?;
    if ligne.withdrawn_at.is_some() {
        return Ok(Retrait::DejaRetire);
    }
    if ligne.published_at.is_none() {
        return Ok(Retrait::PasAffiche);
    }
    sqlx::query!(
        "UPDATE negotiation.session_reports SET withdrawn_at = now(), withdrawal = 'admin'
          WHERE id = $1",
        id
    )
    .execute(&mut *conn)
    .await?;
    if let Some(reunion) = ligne.network_meeting_id {
        sqlx::query!(
            "UPDATE negotiation.network_meetings SET withdrawn_at = now()
              WHERE id = $1 AND withdrawn_at IS NULL",
            reunion
        )
        .execute(&mut *conn)
        .await?;
    }
    Ok(Retrait::Fait)
}
