//! Lecture du schéma `event` — **en lecture seule**.
//!
//! L'édition mesurée, son appel, et **l'échéance qui fait foi**.

use kernel::error::Result;
use serde_json::Value;
use sqlx::postgres::PgConnection;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::dashboard::{CallForProposals, EventEdition};

pub async fn edition(conn: &mut PgConnection, event_id: Uuid) -> Result<Option<EventEdition>> {
    let ligne = sqlx::query_as!(
        EventEdition,
        r#"SELECT e.id, e.series_id, e.edition_label, e.edition_year,
                  e.title AS "title!: Value", e.acronym, e.slug,
                  e.description AS "description!: Value",
                  e.status::text AS "status!",
                  e.participation_mode::text AS "participation_mode!",
                  e.timezone::text AS "timezone!",
                  e.starts_at, e.ends_at,
                  e.country_id, e.city, e.address,
                  e.latitude::float8 AS "latitude?", e.longitude::float8 AS "longitude?",
                  e.has_pavilion, e.programme_published_at,
                  e.highlights AS "highlights?: Value",
                  e.created_by, e.created_at, e.updated_at
             FROM event.events e
            WHERE e.id = $1"#,
        event_id
    )
    .fetch_optional(conn)
    .await?;

    Ok(ligne)
}

/// **Zéro ou un appel par édition, jamais deux** (règle métier n° 5). La
/// requête n'en rend donc qu'un, et le `LIMIT` le dit plutôt que de le supposer.
pub async fn appel(conn: &mut PgConnection, event_id: Uuid) -> Result<Option<CallForProposals>> {
    let ligne = sqlx::query_as!(
        CallForProposals,
        r#"SELECT c.id, c.event_id, c.code,
                  c.title AS "title!: Value", c.description AS "description?: Value",
                  c.status::text AS "status!",
                  c.opens_at, c.closes_at, c.extended_until, c.results_expected_at,
                  c.max_proposals_per_organization, c.requires_verified_organization,
                  c.min_speakers, c.max_speakers,
                  c.default_duration_minutes, c.min_duration_minutes, c.max_duration_minutes,
                  c.daily_start_time::text AS "daily_start_time!",
                  c.daily_end_time::text   AS "daily_end_time!",
                  c.allowed_formats::text[] AS "allowed_formats!",
                  c.required_reviews, c.blind_review,
                  c.guidelines_url::text AS "guidelines_url?",
                  c.created_by, c.created_at, c.updated_at
             FROM event.calls_for_proposals c
            WHERE c.event_id = $1
            ORDER BY c.opens_at
            LIMIT 1"#,
        event_id
    )
    .fetch_optional(conn)
    .await?;

    Ok(ligne)
}

/// **L'échéance qui fait foi** — `event.effective_deadline()`, la fonction du
/// modèle : elle est appelée, jamais recalculée. Recopier
/// `COALESCE(extended_until, closes_at)` ferait une seconde définition, et la
/// première évolution du SQL les ferait diverger.
pub async fn echeance(conn: &mut PgConnection, call_id: Uuid) -> Result<Option<OffsetDateTime>> {
    let instant = sqlx::query_scalar!(
        r#"SELECT event.effective_deadline($1) AS "echeance?""#,
        call_id
    )
    .fetch_one(conn)
    .await?;

    Ok(instant)
}
