//! La lecture des réunions de la Francophonie d'une édition, et la traduction
//! des refus que la base porte sur elles.

use kernel::error::{ApiError, ErrorCode, Result};
use serde_json::Value;
use sqlx::postgres::PgConnection;
use uuid::Uuid;

use crate::domain::meetings::{FrancophoneMeeting, NatureDeReunion, StatutDeReunion};

pub struct Edition {
    pub event_id: Uuid,
    pub slug: String,
    pub timezone: String,
    pub city: Option<String>,
}

pub async fn edition(conn: &mut PgConnection, slug: &str) -> Result<Option<Edition>> {
    let ligne = sqlx::query!(
        r#"SELECT e.id AS "event_id!", e.slug::text AS "slug!", e.timezone::text AS "timezone!",
                  e.city
             FROM event.events e
            WHERE e.slug::text = $1"#,
        slug
    )
    .fetch_optional(conn)
    .await?;
    Ok(ligne.map(|l| Edition {
        event_id: l.event_id,
        slug: l.slug,
        timezone: l.timezone,
        city: l.city,
    }))
}

/// Les réunions publiées de l'édition — un brouillon jamais —, par début.
pub async fn publiees(conn: &mut PgConnection, event_id: Uuid) -> Result<Vec<FrancophoneMeeting>> {
    let lignes = sqlx::query!(
        r#"SELECT m.id, t.code AS type_code, t.label::jsonb AS "type_label!: Value",
                  m.title::jsonb AS "title!: Value", m.description::jsonb AS "description?: Value",
                  m.start_at, m.end_at AS "end_at!", m.format::text AS "format!", m.venue_label,
                  (m.external_url IS NOT NULL) AS "has_video!",
                  m.is_ifdd_organized, o.legal_name AS "organizer_name?",
                  coalesce(m.is_open_access, true) AS "open_access!",
                  m.access_audience::jsonb AS "access_audience?: Value",
                  m.requires_registration, m.capacity, m.registered_count, m.waitlist_enabled,
                  m.registration_opens_at, m.registration_closes_at,
                  (m.status = 'cancelled') AS "annulee!",
                  m.cancellation_reason, m.pavilion_session_id
             FROM negotiation.meetings m
             JOIN reference.taxonomy_terms t ON t.id = m.francophone_type_term_id
             LEFT JOIN org.organizations o ON o.id = org.resolve_organization(m.organizer_org_id)
            WHERE m.event_id = $1
              AND m.kind IN ('preparatory_workshop', 'francophone_consultation')
              AND m.source_key IS NULL
              AND m.status <> 'draft'
            ORDER BY m.start_at, m.id"#,
        event_id
    )
    .fetch_all(conn)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| FrancophoneMeeting {
            id: l.id,
            meeting_type: NatureDeReunion {
                code: l.type_code,
                label: l.type_label,
            },
            title: l.title,
            description: l.description,
            start_at: l.start_at,
            end_at: l.end_at,
            format: l.format,
            venue: l.venue_label,
            has_video: l.has_video,
            organizer: if l.is_ifdd_organized {
                "IFDD".to_owned()
            } else {
                l.organizer_name.unwrap_or_default()
            },
            open_access: l.open_access,
            access_audience: l.access_audience,
            requires_registration: l.requires_registration,
            capacity: l.capacity,
            registered_count: l.registered_count,
            waitlist_enabled: l.waitlist_enabled,
            registration_opens_at: l.registration_opens_at,
            registration_closes_at: l.registration_closes_at,
            status: if l.annulee {
                StatutDeReunion::Cancelled
            } else {
                StatutDeReunion::Scheduled
            },
            cancellation_reason: l.cancellation_reason,
            pavilion_session_id: l.pavilion_session_id,
        })
        .collect())
}

/// `(SQLSTATE, contrainte)` → code stable. Le message de la base commence par
/// son code : il ne s'affiche jamais tel quel.
pub fn refus(erreur: sqlx::Error) -> ApiError {
    let etat = kernel::pg_error::sqlstate(&erreur);
    let code = match (etat.as_deref(), kernel::pg_error::constraint(&erreur)) {
        (Some("23001"), Some("meeting_unavailable")) => ErrorCode::NegotiationMeetingUnavailable,
        (Some("23001"), Some("meeting_closed")) => ErrorCode::NegotiationMeetingClosed,
        (Some("23001"), Some("meeting_full")) => ErrorCode::NegotiationMeetingFull,
        (Some("23514"), Some(c)) if c.starts_with("ck_meetings_") => {
            return ApiError::new(ErrorCode::NegotiationMeetingInvalid).detail(c);
        }
        _ => return erreur.into(),
    };
    ApiError::new(code)
}
