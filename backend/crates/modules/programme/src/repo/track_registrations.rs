//! `programme.track_registrations` et ses activités gardées
//! (`programme.registrations.track_registration_id`).
//!
//! La journée se lit dans `event.programme_tracks` (lecture seule) : sa nature,
//! sa publication, ses bornes, qui se lisent sur son déroulé comme sur la page
//! publique (activités publiées et moments).

use kernel::error::Result;
use serde_json::Value;
use sqlx::postgres::PgConnection;
use sqlx::PgExecutor;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::track_registration::{DayRegistration, DayRegistrationRow};

pub struct Journee {
    pub id: Uuid,
    pub event_id: Uuid,
    pub is_day: bool,
    /// Page publique ouverte, édition ni brouillon ni annulée.
    pub published: bool,
    pub slug: String,
    pub event_slug: String,
    pub title: Value,
    /// Fin du déroulé, à défaut le dernier jour annoncé.
    pub ends_at: Option<OffsetDateTime>,
}

pub async fn journee<'e>(executor: impl PgExecutor<'e>, track_id: Uuid) -> Result<Option<Journee>> {
    let ligne = sqlx::query_as!(
        Journee,
        r#"SELECT t.id, t.event_id, programme.is_day_track_kind(t.kind) AS "is_day!",
                  (t.published_at IS NOT NULL AND e.status NOT IN ('draft', 'cancelled'))
                      AS "published!",
                  t.slug::text AS "slug!", e.slug::text AS "event_slug!",
                  t.title AS "title!: Value",
                  COALESCE(
                      GREATEST(
                          (SELECT max(s.ends_at) FROM programme.session_tracks st
                             JOIN programme.sessions s ON s.id = st.session_id
                            WHERE st.track_id = t.id AND s.published_at IS NOT NULL
                              AND s.status <> 'cancelled'),
                          (SELECT max(COALESCE(m.ends_at, m.starts_at))
                             FROM event.programme_track_moments m WHERE m.track_id = t.id)),
                      ((COALESCE(t.ends_on, t.starts_on) + 1)::timestamp AT TIME ZONE e.timezone)
                  ) AS ends_at
             FROM event.programme_tracks t
             JOIN event.events e ON e.id = t.event_id
            WHERE t.id = $1"#,
        track_id
    )
    .fetch_optional(executor)
    .await?;
    Ok(ligne)
}

/// La journée dont l'activité suit le direct (`programme.day_track_of`).
pub async fn journee_de_seance<'e>(
    executor: impl PgExecutor<'e>,
    session_id: Uuid,
) -> Result<Option<Uuid>> {
    let id = sqlx::query_scalar!(
        r#"SELECT programme.day_track_of($1) AS "track_id""#,
        session_id
    )
    .fetch_one(executor)
    .await?;
    Ok(id)
}

/// Les activités qu'on peut garder : publiées, non annulées, de la journée.
pub async fn activites<'e>(executor: impl PgExecutor<'e>, track_id: Uuid) -> Result<Vec<Uuid>> {
    let ids = sqlx::query_scalar!(
        r#"SELECT s.id FROM programme.session_tracks st
             JOIN programme.sessions s ON s.id = st.session_id
            WHERE st.track_id = $1 AND s.published_at IS NOT NULL AND s.status <> 'cancelled'
            ORDER BY s.starts_at, st.sort_order"#,
        track_id
    )
    .fetch_all(executor)
    .await?;
    Ok(ids)
}

pub async fn vivante(
    conn: &mut PgConnection,
    track_id: Uuid,
    person_id: Uuid,
) -> Result<Option<Uuid>> {
    let id = sqlx::query_scalar!(
        "SELECT id FROM programme.track_registrations
          WHERE track_id = $1 AND person_id = $2 AND status <> 'cancelled'",
        track_id,
        person_id
    )
    .fetch_optional(conn)
    .await?;
    Ok(id)
}

pub struct Nouvelle<'a> {
    pub track_id: Uuid,
    pub person_id: Uuid,
    pub organization_id: Option<Uuid>,
    pub answers: &'a Value,
    pub locale: &'a str,
    pub manage_token_hash: Option<&'a [u8]>,
}

/// `None` : une inscription concurrente de la même personne vient d'être posée.
pub async fn inserer(conn: &mut PgConnection, n: &Nouvelle<'_>) -> Result<Option<Uuid>> {
    let id = sqlx::query_scalar!(
        "INSERT INTO programme.track_registrations
             (track_id, person_id, organization_id, answers, locale, manage_token_hash)
         VALUES ($1, $2, $3, $4, $5, $6)
         ON CONFLICT (track_id, person_id) WHERE status <> 'cancelled' DO NOTHING
         RETURNING id",
        n.track_id,
        n.person_id,
        n.organization_id,
        n.answers,
        n.locale,
        n.manage_token_hash
    )
    .fetch_optional(conn)
    .await?;
    Ok(id)
}

/// Aligne les activités gardées sur `session_ids` : les autres sont annulées,
/// celles-ci créées, ou reprises si la personne y était déjà inscrite à part.
pub async fn garder(
    conn: &mut PgConnection,
    track_registration_id: Uuid,
    session_ids: &[Uuid],
) -> Result<()> {
    sqlx::query!(
        "UPDATE programme.registrations
            SET status = 'cancelled', waitlist_position = NULL, cancelled_at = now(),
                cancelled_reason = 'Activité retirée par la personne inscrite à la journée.'
          WHERE track_registration_id = $1 AND status <> 'cancelled'
            AND session_id <> ALL ($2)",
        track_registration_id,
        session_ids
    )
    .execute(&mut *conn)
    .await?;

    sqlx::query!(
        "INSERT INTO programme.registrations
                (session_id, person_id, organization_id, answers, locale, source,
                 track_registration_id)
         SELECT g.session_id, tr.person_id, tr.organization_id, tr.answers, tr.locale,
                tr.source, tr.id
           FROM unnest($2::uuid[]) AS g(session_id)
           JOIN programme.track_registrations tr ON tr.id = $1
         ON CONFLICT (session_id, person_id) WHERE status <> 'cancelled'
         DO UPDATE SET track_registration_id = EXCLUDED.track_registration_id,
                       waitlist_position = NULL,
                       status = CASE WHEN programme.registrations.status = 'waitlisted'
                                     THEN 'registered'::programme.registration_status
                                     ELSE programme.registrations.status END
                 WHERE programme.registrations.track_registration_id
                       IS DISTINCT FROM EXCLUDED.track_registration_id",
        track_registration_id,
        session_ids
    )
    .execute(conn)
    .await?;
    Ok(())
}

/// Annule l'inscription et ses activités gardées. Faux : déjà annulée.
pub async fn annuler(conn: &mut PgConnection, track_registration_id: Uuid) -> Result<bool> {
    let annulee = sqlx::query!(
        "UPDATE programme.track_registrations
            SET status = 'cancelled', cancelled_at = now()
          WHERE id = $1 AND status <> 'cancelled'",
        track_registration_id
    )
    .execute(&mut *conn)
    .await?
    .rows_affected()
        == 1;
    if annulee {
        garder(conn, track_registration_id, &[]).await?;
    }
    Ok(annulee)
}

pub async fn poser_empreinte(
    conn: &mut PgConnection,
    track_registration_id: Uuid,
    empreinte: &[u8],
) -> Result<()> {
    sqlx::query!(
        "UPDATE programme.track_registrations SET manage_token_hash = $2 WHERE id = $1",
        track_registration_id,
        empreinte
    )
    .execute(conn)
    .await?;
    Ok(())
}

pub struct Proprietaire {
    pub id: Uuid,
    pub track_id: Uuid,
    pub person_id: Uuid,
    pub status: String,
}

pub async fn proprietaire<'e>(
    executor: impl PgExecutor<'e>,
    track_registration_id: Uuid,
) -> Result<Option<Proprietaire>> {
    let ligne = sqlx::query_as!(
        Proprietaire,
        r#"SELECT id, track_id, person_id, status::text AS "status!"
             FROM programme.track_registrations WHERE id = $1"#,
        track_registration_id
    )
    .fetch_optional(executor)
    .await?;
    Ok(ligne)
}

pub async fn par_empreinte<'e>(
    executor: impl PgExecutor<'e>,
    empreinte: &[u8],
) -> Result<Option<Proprietaire>> {
    let ligne = sqlx::query_as!(
        Proprietaire,
        r#"SELECT id, track_id, person_id, status::text AS "status!"
             FROM programme.track_registrations WHERE manage_token_hash = $1"#,
        empreinte
    )
    .fetch_optional(executor)
    .await?;
    Ok(ligne)
}

pub async fn etat<'e>(
    executor: impl PgExecutor<'e>,
    track_registration_id: Uuid,
) -> Result<Option<DayRegistration>> {
    let ligne = sqlx::query_as!(
        DayRegistration,
        r#"SELECT tr.id, tr.track_id, tr.status::text AS "status!",
                  COALESCE(ARRAY(
                      SELECT r.session_id FROM programme.registrations r
                        JOIN programme.sessions s ON s.id = r.session_id
                       WHERE r.track_registration_id = tr.id AND r.status <> 'cancelled'
                       ORDER BY s.starts_at), '{}') AS "session_ids!",
                  (SELECT count(*) FROM programme.session_tracks st
                     JOIN programme.sessions s ON s.id = st.session_id
                    WHERE st.track_id = tr.track_id AND s.published_at IS NOT NULL
                      AND s.status <> 'cancelled') AS "activity_count!",
                  tr.created_at, tr.joined_at
             FROM programme.track_registrations tr
            WHERE tr.id = $1"#,
        track_registration_id
    )
    .fetch_optional(executor)
    .await?;
    Ok(ligne)
}

/// Le prénom de la personne, pour la page de gestion d'un invité.
pub async fn prenom<'e>(executor: impl PgExecutor<'e>, person_id: Uuid) -> Result<String> {
    let prenom = sqlx::query_scalar!(
        "SELECT first_name FROM identity.people WHERE id = $1",
        person_id
    )
    .fetch_optional(executor)
    .await?;
    Ok(prenom.unwrap_or_default())
}

pub async fn liste_nominative<'e>(
    executor: impl PgExecutor<'e>,
    track_id: Uuid,
) -> Result<Vec<DayRegistrationRow>> {
    let lignes = sqlx::query_as!(
        DayRegistrationRow,
        r#"SELECT tr.id, tr.status::text AS "status!", p.id AS person_id,
                  p.first_name, p.last_name, p.primary_email::text AS "email!",
                  o.legal_name AS "organization_name?",
                  EXISTS (SELECT 1 FROM identity.accounts a WHERE a.person_id = p.id)
                      AS "has_account!",
                  (SELECT count(*) FROM programme.registrations r
                    WHERE r.track_registration_id = tr.id AND r.status <> 'cancelled')
                      AS "kept_count!",
                  tr.created_at, tr.joined_at
             FROM programme.track_registrations tr
             JOIN identity.people p ON p.id = tr.person_id
             LEFT JOIN org.organizations o ON o.id = tr.organization_id
            WHERE tr.track_id = $1
            ORDER BY tr.status = 'cancelled', p.last_name, p.first_name"#,
        track_id
    )
    .fetch_all(executor)
    .await?;
    Ok(lignes)
}
