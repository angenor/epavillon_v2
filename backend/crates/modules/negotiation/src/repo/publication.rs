//! Ce que la publication et le refus lisent et écrivent : le signalement tenu,
//! la réunion non annoncée qui en naît, et qui prévenir.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

/// Le signalement, ligne tenue, avec ce que l'avis doit dire — heures déjà
/// dans le fuseau de la COP.
pub struct Tenu {
    pub id: Uuid,
    pub author_id: Uuid,
    pub reason: String,
    pub meeting_id: Option<Uuid>,
    pub status: String,
    pub decided_at: Option<OffsetDateTime>,
    pub published_at: Option<OffsetDateTime>,
    pub withdrawn_at: Option<OffsetDateTime>,
    pub what: Option<String>,
    pub proposed_venue: Option<String>,
    pub proposed_day: Option<Date>,
    pub reject_reason: Option<String>,
    pub heure: Option<String>,
    pub ville: Option<String>,
    pub jour: Date,
    pub title_en: Option<String>,
    pub title_fr: Option<String>,
}

pub async fn tenir(conn: &mut PgConnection, id: Uuid) -> Result<Option<Tenu>> {
    let ligne = sqlx::query!(
        r#"SELECT r.id, r.author_id, r.reason, r.meeting_id, r.status::text AS "status!",
                  r.decided_at, r.published_at, r.withdrawn_at, r.what, r.proposed_venue,
                  r.proposed_day, r.reject_reason,
                  to_char(r.proposed_start AT TIME ZONE e.timezone::text, 'HH24:MI') AS heure,
                  e.city, (now() AT TIME ZONE e.timezone::text)::date AS "jour!",
                  m.title_original AS "title_en?", tr.text_fr AS "title_fr?"
             FROM negotiation.session_reports r
             JOIN event.events e ON e.id = r.event_id
             LEFT JOIN negotiation.meetings m ON m.id = r.meeting_id
             LEFT JOIN negotiation.title_translations tr ON tr.source_text = m.title_original
            WHERE r.id = $1
              FOR UPDATE OF r"#,
        id
    )
    .fetch_optional(conn)
    .await?;
    Ok(ligne.map(|l| Tenu {
        id: l.id,
        author_id: l.author_id,
        reason: l.reason,
        meeting_id: l.meeting_id,
        status: l.status,
        decided_at: l.decided_at,
        published_at: l.published_at,
        withdrawn_at: l.withdrawn_at,
        what: l.what,
        proposed_venue: l.proposed_venue,
        proposed_day: l.proposed_day,
        reject_reason: l.reject_reason,
        heure: l.heure,
        ville: l.city,
        jour: l.jour,
        title_en: l.title_en,
        title_fr: l.title_fr,
    }))
}

pub async fn publier(conn: &mut PgConnection, id: Uuid) -> Result<()> {
    sqlx::query!(
        "UPDATE negotiation.session_reports SET published_at = now() WHERE id = $1",
        id
    )
    .execute(conn)
    .await?;
    Ok(())
}

/// La réunion non annoncée, recopiée du signalement, et liée à lui.
pub async fn creer_reunion(conn: &mut PgConnection, report_id: Uuid) -> Result<Uuid> {
    let id = sqlx::query_scalar!(
        r#"INSERT INTO negotiation.network_meetings
               (event_id, report_id, title, venue, start_at, day, theme_term_id, validated_at)
           SELECT event_id, id, what, proposed_venue, proposed_start, proposed_day,
                  theme_term_id, decided_at
             FROM negotiation.session_reports WHERE id = $1
        RETURNING id"#,
        report_id
    )
    .fetch_one(&mut *conn)
    .await?;
    sqlx::query!(
        "UPDATE negotiation.session_reports SET network_meeting_id = $2 WHERE id = $1",
        report_id,
        id
    )
    .execute(conn)
    .await?;
    Ok(id)
}

pub async fn destinataires_session(conn: &mut PgConnection, meeting_id: Uuid) -> Result<Vec<Uuid>> {
    Ok(sqlx::query_scalar!(
        r#"SELECT p AS "p!" FROM negotiation.change_recipients($1) p ORDER BY 1"#,
        meeting_id
    )
    .fetch_all(conn)
    .await?)
}

pub async fn destinataires_reunion(
    conn: &mut PgConnection,
    network_meeting_id: Uuid,
) -> Result<Vec<Uuid>> {
    Ok(sqlx::query_scalar!(
        r#"SELECT p AS "p!" FROM negotiation.network_recipients($1) p ORDER BY 1"#,
        network_meeting_id
    )
    .fetch_all(conn)
    .await?)
}
