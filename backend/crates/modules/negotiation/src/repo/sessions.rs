//! La lecture des sessions officielles d'une édition, et de ce qui dit si on
//! peut les servir. La règle de coupure n'est lue que par
//! `negotiation.import_is_serving()`, jamais réécrite ici.

use kernel::error::Result;
use serde_json::Value;
use sqlx::postgres::PgConnection;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::sessions::{
    Annulation, Groupe, OfficialSession, PointDuJour, Precedent, StatutDeSession, TypeDeSession,
};

pub struct EditionImportee {
    pub event_id: Uuid,
    pub slug: String,
    pub timezone: String,
    pub city: Option<String>,
    pub official_programme_url: String,
    pub is_enabled: bool,
    pub last_success_at: Option<OffsetDateTime>,
    pub failing_since: Option<OffsetDateTime>,
    pub serving: bool,
}

/// L'édition et son import. Sans ligne d'import, l'édition ne reçoit pas les
/// sessions officielles : elle est inconnue de cette route.
pub async fn edition(conn: &mut PgConnection, slug: &str) -> Result<Option<EditionImportee>> {
    let ligne = sqlx::query!(
        r#"SELECT e.id AS "event_id!", e.slug::text AS "slug!", e.timezone::text AS "timezone!",
                  e.city, i.official_programme_url::text AS "official_programme_url!",
                  i.is_enabled, i.last_success_at, i.failing_since,
                  negotiation.import_is_serving(e.id) AS "serving!"
             FROM event.events e
             JOIN negotiation.official_imports i ON i.event_id = e.id
            WHERE e.slug::text = $1"#,
        slug
    )
    .fetch_optional(conn)
    .await?;

    Ok(ligne.map(|l| EditionImportee {
        event_id: l.event_id,
        slug: l.slug,
        timezone: l.timezone,
        city: l.city,
        official_programme_url: l.official_programme_url,
        is_enabled: l.is_enabled,
        last_success_at: l.last_success_at,
        failing_since: l.failing_since,
        serving: l.serving,
    }))
}

/// Toutes les sessions importées de l'édition.
///
/// `read_at` : tant qu'elle figure à la source (`absent_reads = 0`), la
/// dernière lecture réussie l'a vue — `last_read_at` n'est réécrite qu'avec un
/// écart, et une session relue sans changement à 14 h afficherait « lu à 8 h ».
pub async fn sessions(
    conn: &mut PgConnection,
    event_id: Uuid,
    derniere_reussite: Option<OffsetDateTime>,
) -> Result<Vec<OfficialSession>> {
    let lignes = sqlx::query!(
        r#"SELECT m.id,
                  m.title_original AS "title_en!",
                  tr.text_fr AS "title_fr?",
                  m.start_at, m.end_at, m.venue_label,
                  p.changed_at AS "changed_at?",
                  CASE WHEN p.has_start THEN (p.old_start #>> '{}')::timestamptz
                       ELSE m.start_at END AS "prev_start!",
                  CASE WHEN p.has_end THEN (p.old_end #>> '{}')::timestamptz
                       ELSE m.end_at END AS "prev_end?",
                  CASE WHEN p.has_venue THEN p.old_venue #>> '{}'
                       ELSE m.venue_label END AS "prev_venue?",
                  ty.code AS "type_code?", ty.label::jsonb AS "type_label?: Value",
                  ty.label ->> 'en' AS "type_en?",
                  g.code AS "group_code?", g.label::jsonb AS "group_label?: Value",
                  th.code AS "theme?",
                  a.code AS "point_code?", a.title AS "point_title?",
                  m.is_open_access,
                  (m.status = 'cancelled') AS "annulee!",
                  m.cancelled_at, m.cancellation_reason,
                  m.source_url::text AS source_url,
                  CASE WHEN m.absent_reads = 0 THEN coalesce($2, m.last_read_at)
                       ELSE m.last_read_at END AS "read_at!"
             FROM negotiation.meetings m
             LEFT JOIN negotiation.title_translations tr ON tr.source_text = m.title_original
             LEFT JOIN reference.taxonomy_terms ty ON ty.id = m.meeting_type_term_id
             LEFT JOIN reference.taxonomy_terms g ON g.id = m.group_term_id
             LEFT JOIN negotiation.agenda_items a ON a.id = m.agenda_item_id
             LEFT JOIN reference.taxonomy_terms th ON th.id = a.theme_term_id
             LEFT JOIN LATERAL (
                   SELECT max(c.detected_at) AS changed_at,
                          bool_or(c.field = 'start') AS has_start,
                          bool_or(c.field = 'end') AS has_end,
                          bool_or(c.field = 'venue') AS has_venue,
                          (array_agg(c.old_value ORDER BY c.detected_at DESC, c.id DESC)
                              FILTER (WHERE c.field = 'start'))[1] AS old_start,
                          (array_agg(c.old_value ORDER BY c.detected_at DESC, c.id DESC)
                              FILTER (WHERE c.field = 'end'))[1] AS old_end,
                          (array_agg(c.old_value ORDER BY c.detected_at DESC, c.id DESC)
                              FILTER (WHERE c.field = 'venue'))[1] AS old_venue
                     FROM negotiation.meeting_changes c
                    WHERE c.meeting_id = m.id AND c.field IN ('start', 'end', 'venue')
             ) p ON true
            WHERE m.event_id = $1
              AND m.kind = 'negotiation_session'
              AND m.source_key IS NOT NULL
            ORDER BY m.start_at, m.id"#,
        event_id,
        derniere_reussite
    )
    .fetch_all(conn)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| OfficialSession {
            id: l.id,
            title_en: l.title_en,
            title_fr: l.title_fr,
            start_at: l.start_at,
            end_at: l.end_at,
            venue: l.venue_label,
            previous: l.changed_at.map(|changed_at| Precedent {
                start_at: l.prev_start,
                end_at: l.prev_end,
                venue: l.prev_venue,
                changed_at,
            }),
            meeting_type: l.type_code.map(|code| TypeDeSession {
                code,
                label: l.type_label.unwrap_or(Value::Null),
                term_en: l.type_en.unwrap_or_default(),
            }),
            group: l.group_code.map(|code| Groupe {
                code,
                label: l.group_label.unwrap_or(Value::Null),
            }),
            theme: l.theme,
            agenda_item: match (l.point_code, l.point_title) {
                (Some(code), Some(title)) => Some(PointDuJour { code, title }),
                _ => None,
            },
            open_access: l.is_open_access,
            status: if l.annulee {
                StatutDeSession::Cancelled
            } else {
                StatutDeSession::Scheduled
            },
            cancelled: match (l.annulee, l.cancelled_at) {
                (true, Some(at)) => Some(Annulation {
                    at,
                    reason: l.cancellation_reason.unwrap_or_default(),
                }),
                _ => None,
            },
            source_url: l.source_url,
            read_at: l.read_at,
        })
        .collect())
}
