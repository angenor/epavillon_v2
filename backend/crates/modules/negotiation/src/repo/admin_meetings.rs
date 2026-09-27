//! Les réunions de la Francophonie au back-office : saisies à la main, jamais
//! importées. Chaque requête s'y borne (`kind` des deux natures, `source_key`
//! nul) : une session officielle ne s'édite pas d'ici.

use kernel::error::Result;
use serde_json::Value;
use sqlx::postgres::PgConnection;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::admin_meetings::{AdminFrancophoneMeeting, AdminMeetingRegistrant};
use crate::domain::meetings::NatureDeReunion;
use crate::repo::meetings::refus;

/// Ce que le corps pose, nettoyé ; le reste vient du serveur.
pub struct Saisie {
    pub nature: Option<Uuid>,
    pub kind: &'static str,
    pub title: Value,
    pub description: Option<Value>,
    pub start_at: OffsetDateTime,
    pub end_at: OffsetDateTime,
    pub format: &'static str,
    pub venue: Option<String>,
    pub external_url: Option<String>,
    pub capacity: Option<i32>,
    pub waitlist_enabled: bool,
    pub requires_registration: bool,
    pub registration_opens_at: Option<OffsetDateTime>,
    pub registration_closes_at: Option<OffsetDateTime>,
    pub open_access: bool,
    pub access_audience: Option<Value>,
    pub is_ifdd_organized: bool,
    pub organizer_org_id: Option<Uuid>,
}

/// Ce que le serveur pose à la création (R9 bis).
pub struct Origine<'a> {
    pub space_id: Uuid,
    pub event_id: Uuid,
    pub slug: &'a str,
    pub timezone: &'a str,
    pub created_by: Uuid,
}

/// Une réunion (`id`), ou toutes celles d'une édition (`event_id`), par début.
pub async fn lire(
    conn: &mut PgConnection,
    event_id: Option<Uuid>,
    id: Option<Uuid>,
) -> Result<Vec<AdminFrancophoneMeeting>> {
    let lignes = sqlx::query!(
        r#"SELECT m.id, m.slug::text AS "slug!", e.slug::text AS "edition!",
                  m.timezone::text AS "timezone!",
                  t.code AS "type_code?", t.label::jsonb AS "type_label?: Value",
                  m.title::jsonb AS "title!: Value", m.description::jsonb AS "description?: Value",
                  m.start_at, m.end_at AS "end_at!", m.format::text AS "format!", m.venue_label,
                  m.external_url::text AS "external_url?", m.capacity, m.waitlist_enabled,
                  m.requires_registration, m.registration_opens_at, m.registration_closes_at,
                  coalesce(m.is_open_access, true) AS "open_access!",
                  m.access_audience::jsonb AS "access_audience?: Value",
                  m.is_ifdd_organized, m.organizer_org_id, o.legal_name AS "organizer_name?",
                  m.status::text AS "status!", m.cancellation_reason, m.pavilion_session_id,
                  m.registered_count,
                  (SELECT count(*) FROM negotiation.meeting_registrations r
                    WHERE r.meeting_id = m.id AND r.status = 'waitlisted') AS "waitlisted_count!",
                  m.updated_at
             FROM negotiation.meetings m
             JOIN event.events e ON e.id = m.event_id
             LEFT JOIN reference.taxonomy_terms t ON t.id = m.francophone_type_term_id
             LEFT JOIN org.organizations o ON o.id = org.resolve_organization(m.organizer_org_id)
            WHERE m.kind IN ('preparatory_workshop', 'francophone_consultation')
              AND m.source_key IS NULL
              AND ($1::uuid IS NULL OR m.event_id = $1)
              AND ($2::uuid IS NULL OR m.id = $2)
            ORDER BY m.start_at, m.id"#,
        event_id,
        id
    )
    .fetch_all(conn)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| AdminFrancophoneMeeting {
            id: l.id,
            slug: l.slug,
            edition: l.edition,
            timezone: l.timezone,
            meeting_type: l
                .type_code
                .zip(l.type_label)
                .map(|(code, label)| NatureDeReunion { code, label }),
            title: l.title,
            description: l.description,
            start_at: l.start_at,
            end_at: l.end_at,
            format: l.format,
            venue: l.venue_label,
            external_url: l.external_url,
            capacity: l.capacity,
            waitlist_enabled: l.waitlist_enabled,
            requires_registration: l.requires_registration,
            registration_opens_at: l.registration_opens_at,
            registration_closes_at: l.registration_closes_at,
            open_access: l.open_access,
            access_audience: l.access_audience,
            organizer: if l.is_ifdd_organized {
                "IFDD".to_owned()
            } else {
                l.organizer_name.unwrap_or_default()
            },
            is_ifdd_organized: l.is_ifdd_organized,
            organizer_org_id: l.organizer_org_id,
            status: l.status,
            cancellation_reason: l.cancellation_reason,
            pavilion_session_id: l.pavilion_session_id,
            registered_count: l.registered_count,
            waitlisted_count: l.waitlisted_count,
            updated_at: l.updated_at,
        })
        .collect())
}

pub async fn nature(conn: &mut PgConnection, code: &str) -> Result<Option<Uuid>> {
    let id = sqlx::query_scalar!(
        "SELECT id FROM reference.taxonomy_terms
          WHERE taxonomy_code = 'francophone_meeting_type' AND code = $1 AND is_active",
        code
    )
    .fetch_optional(conn)
    .await?;
    Ok(id)
}

pub async fn inserer(conn: &mut PgConnection, o: &Origine<'_>, s: &Saisie) -> Result<Uuid> {
    let id = sqlx::query_scalar!(
        r#"INSERT INTO negotiation.meetings
               (space_id, event_id, slug, timezone, created_by, status,
                kind, francophone_type_term_id, title, description, start_at, end_at, format,
                venue_label, external_url, capacity, waitlist_enabled, requires_registration,
                registration_opens_at, registration_closes_at, is_open_access, access_audience,
                is_ifdd_organized, organizer_org_id)
           VALUES ($1, $2, $3::text::platform.slug, $4::text::platform.timezone_name, $5, 'draft',
                   $6::text::negotiation.meeting_kind, $7, $8::jsonb::platform.i18n_text,
                   $9::jsonb::platform.i18n_text, $10, $11, $12::text::negotiation.meeting_format,
                   $13, $14::text::platform.url, $15, $16, $17, $18, $19, $20,
                   $21::jsonb::platform.i18n_text, $22, $23)
           RETURNING id"#,
        o.space_id,
        o.event_id,
        o.slug,
        o.timezone,
        o.created_by,
        s.kind,
        s.nature,
        s.title,
        s.description,
        s.start_at,
        s.end_at,
        s.format,
        s.venue,
        s.external_url,
        s.capacity,
        s.waitlist_enabled,
        s.requires_registration,
        s.registration_opens_at,
        s.registration_closes_at,
        s.open_access,
        s.access_audience,
        s.is_ifdd_organized,
        s.organizer_org_id,
    )
    .fetch_one(conn)
    .await
    .map_err(refus)?;
    Ok(id)
}

/// L'état avant l'écriture, sous verrou : le service compare ce qui change.
pub struct Avant {
    pub status: String,
    pub capacity: Option<i32>,
    pub start_at: OffsetDateTime,
    pub end_at: Option<OffsetDateTime>,
    pub format: String,
    pub venue: Option<String>,
}

pub async fn verrouiller(conn: &mut PgConnection, id: Uuid) -> Result<Option<Avant>> {
    let ligne = sqlx::query_as!(
        Avant,
        r#"SELECT status::text AS "status!", capacity, start_at, end_at,
                  format::text AS "format!", venue_label AS venue
             FROM negotiation.meetings
            WHERE id = $1
              AND kind IN ('preparatory_workshop', 'francophone_consultation')
              AND source_key IS NULL
              FOR UPDATE"#,
        id
    )
    .fetch_optional(conn)
    .await?;
    Ok(ligne)
}

pub async fn modifier(conn: &mut PgConnection, id: Uuid, s: &Saisie) -> Result<()> {
    sqlx::query!(
        r#"UPDATE negotiation.meetings
              SET kind = $2::text::negotiation.meeting_kind, francophone_type_term_id = $3,
                  title = $4::jsonb::platform.i18n_text, description = $5::jsonb::platform.i18n_text,
                  start_at = $6, end_at = $7, format = $8::text::negotiation.meeting_format,
                  venue_label = $9, external_url = $10::text::platform.url, capacity = $11,
                  waitlist_enabled = $12, requires_registration = $13,
                  registration_opens_at = $14, registration_closes_at = $15,
                  is_open_access = $16, access_audience = $17::jsonb::platform.i18n_text,
                  is_ifdd_organized = $18, organizer_org_id = $19
            WHERE id = $1"#,
        id,
        s.kind,
        s.nature,
        s.title,
        s.description,
        s.start_at,
        s.end_at,
        s.format,
        s.venue,
        s.external_url,
        s.capacity,
        s.waitlist_enabled,
        s.requires_registration,
        s.registration_opens_at,
        s.registration_closes_at,
        s.open_access,
        s.access_audience,
        s.is_ifdd_organized,
        s.organizer_org_id,
    )
    .execute(conn)
    .await
    .map_err(refus)?;
    Ok(())
}

/// Brouillon → publiée. Les contraintes qui ne mordent qu'hors brouillon
/// refusent ici, traduites.
pub async fn publier(conn: &mut PgConnection, id: Uuid) -> Result<()> {
    sqlx::query!(
        "UPDATE negotiation.meetings SET status = 'scheduled' WHERE id = $1 AND status = 'draft'",
        id
    )
    .execute(conn)
    .await
    .map_err(refus)?;
    Ok(())
}

pub async fn annuler(conn: &mut PgConnection, id: Uuid, motif: &str) -> Result<()> {
    sqlx::query!(
        "UPDATE negotiation.meetings SET status = 'cancelled', cancellation_reason = $2
          WHERE id = $1 AND status <> 'cancelled'",
        id,
        motif
    )
    .execute(conn)
    .await
    .map_err(refus)?;
    Ok(())
}

pub async fn lier_au_pavillon(
    conn: &mut PgConnection,
    id: Uuid,
    activite: Option<Uuid>,
) -> Result<()> {
    sqlx::query!(
        "UPDATE negotiation.meetings SET pavilion_session_id = $2 WHERE id = $1",
        id,
        activite
    )
    .execute(conn)
    .await
    .map_err(refus)?;
    Ok(())
}

/// Inscrites et liste d'attente, désinscrites exclues ; `status` dit laquelle.
pub async fn inscrites(
    conn: &mut PgConnection,
    id: Uuid,
) -> Result<Vec<(String, AdminMeetingRegistrant)>> {
    let lignes = sqlx::query!(
        r#"SELECT r.status::text AS "status!", p.id AS person_id,
                  p.display_name AS "name!", pays.name::jsonb AS "country?: Value",
                  r.registered_at, r.waitlist_position
             FROM negotiation.meeting_registrations r
             JOIN identity.people p ON p.id = r.person_id
             LEFT JOIN reference.countries pays ON pays.id = p.country_id
            WHERE r.meeting_id = $1 AND r.status <> 'cancelled'
            ORDER BY r.waitlist_position NULLS FIRST, r.registered_at, p.id"#,
        id
    )
    .fetch_all(conn)
    .await?;
    Ok(lignes
        .into_iter()
        .map(|l| {
            (
                l.status,
                AdminMeetingRegistrant {
                    person_id: l.person_id,
                    name: l.name,
                    country: l.country,
                    registered_at: l.registered_at,
                    waitlist_position: l.waitlist_position,
                },
            )
        })
        .collect())
}
