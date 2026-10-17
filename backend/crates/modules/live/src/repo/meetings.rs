//! `live.meetings` et `live.meeting_participants`.
//!
//! Aucune lecture de `passcode_encrypted` ni de `start_url_encrypted` hors du
//! seul chemin qui les déchiffre (l'accès d'une personne inscrite).

use kernel::error::Result;
use kernel::jobs::{self, NewJob};
use serde_json::{json, Value};
use sqlx::postgres::PgConnection;
use time::OffsetDateTime;
use uuid::Uuid;

pub const FILE: &str = "live";
pub const SYNC_MEETING: &str = "live.sync_meeting";
pub const SYNC_PARTICIPANT: &str = "live.sync_meeting_participant";
pub const CANCEL_PARTICIPANT: &str = "live.cancel_meeting_participant";
pub const CANCEL_MEETING: &str = "live.cancel_meeting";
pub const PROCESS_WEBHOOK: &str = "live.process_webhook_event";

pub struct ReunionResume {
    pub id: Uuid,
    pub external_id: Option<String>,
    pub sync_status: String,
}

/// La réunion vivante d'une séance (`ux_meetings_active_per_session`).
pub async fn reunion_active(
    conn: &mut PgConnection,
    session_id: Uuid,
) -> Result<Option<ReunionResume>> {
    let ligne = sqlx::query_as!(
        ReunionResume,
        r#"SELECT id, external_id, sync_status::text AS "sync_status!"
             FROM live.meetings
            WHERE session_id = $1 AND status NOT IN ('cancelled', 'failed')"#,
        session_id
    )
    .fetch_optional(conn)
    .await?;
    Ok(ligne)
}

pub struct NouvelleReunion<'a> {
    pub session_id: Uuid,
    pub event_id: Uuid,
    pub topic: &'a str,
    pub scheduled_start_at: OffsetDateTime,
    pub duration_minutes: i32,
    pub timezone: &'a str,
    pub external_id: Option<&'a str>,
}

/// Crée la réunion en `pending` (le déclencheur dépose sa création chez Zoom),
/// ou rend celle qu'une écriture concurrente vient de poser.
pub async fn creer_reunion(conn: &mut PgConnection, r: &NouvelleReunion<'_>) -> Result<Uuid> {
    let cree = sqlx::query_scalar!(
        r#"INSERT INTO live.meetings
               (session_id, event_id, provider, external_id, topic, scheduled_start_at,
                duration_minutes, timezone, requires_registration, status, sync_status,
                created_by)
           VALUES ($1, $2, 'zoom', $3, $4, $5, $6, $7::text::platform.timezone_name, true,
                   'draft', 'pending', platform.current_actor_id())
           ON CONFLICT (session_id) WHERE session_id IS NOT NULL
                                     AND status NOT IN ('cancelled', 'failed')
           DO NOTHING
           RETURNING id"#,
        r.session_id,
        r.event_id,
        r.external_id,
        r.topic,
        r.scheduled_start_at,
        r.duration_minutes,
        r.timezone
    )
    .fetch_optional(&mut *conn)
    .await?;

    match cree {
        Some(id) => Ok(id),
        None => reunion_active(conn, r.session_id)
            .await?
            .map(|r| r.id)
            .ok_or_else(|| kernel::error::ApiError::internal("réunion ni créée ni trouvée")),
    }
}

/// Rattache une autre réunion Zoom : les inscrits devront y être enregistrés
/// de nouveau, la relance de la réunion les reprendra (`failed`).
pub async fn rattacher(conn: &mut PgConnection, meeting_id: Uuid, external_id: &str) -> Result<()> {
    sqlx::query!(
        "UPDATE live.meetings
            SET external_id = $2, external_uuid = NULL, status = 'draft',
                join_url = NULL, registration_url = NULL,
                passcode_encrypted = NULL, start_url_encrypted = NULL
          WHERE id = $1",
        meeting_id,
        external_id
    )
    .execute(&mut *conn)
    .await?;
    sqlx::query!(
        "UPDATE live.meeting_participants
            SET external_registrant_id = NULL, personal_join_url = NULL, synced_at = NULL,
                sync_status = 'failed', last_error = 'réunion Zoom changée'
          WHERE meeting_id = $1 AND cancelled_at IS NULL",
        meeting_id
    )
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn external_id_pris(
    conn: &mut PgConnection,
    external_id: &str,
    sauf: Option<Uuid>,
) -> Result<bool> {
    let pris = sqlx::query_scalar!(
        r#"SELECT EXISTS (
               SELECT 1 FROM live.meetings
                WHERE provider = 'zoom' AND external_id = $1
                  AND ($2::uuid IS NULL OR id <> $2)
           ) AS "pris!""#,
        external_id,
        sauf
    )
    .fetch_one(conn)
    .await?;
    Ok(pris)
}

/// Repasse la réunion en `pending` : le déclencheur dépose un nouveau travail.
/// Jamais pendant qu'un travail tient la ligne : il la rendrait `pending` sous
/// ses pieds, et un second travail créerait une seconde réunion. Un travail
/// mort depuis dix minutes ne la tient plus.
pub async fn relancer_reunion(conn: &mut PgConnection, meeting_id: Uuid) -> Result<()> {
    sqlx::query!(
        "UPDATE live.meetings SET sync_status = 'pending', last_sync_error = NULL
          WHERE id = $1
            AND (sync_status <> 'in_progress' OR updated_at < now() - interval '10 minutes')",
        meeting_id
    )
    .execute(conn)
    .await?;
    Ok(())
}

pub struct ParticipantResume {
    pub id: Uuid,
    pub meeting_id: Uuid,
    pub registration_id: Option<Uuid>,
    pub cancelled_at: Option<OffsetDateTime>,
}

pub async fn participant_de_inscription(
    conn: &mut PgConnection,
    registration_id: Uuid,
) -> Result<Option<ParticipantResume>> {
    let ligne = sqlx::query_as!(
        ParticipantResume,
        "SELECT id, meeting_id, registration_id, cancelled_at
           FROM live.meeting_participants WHERE registration_id = $1",
        registration_id
    )
    .fetch_optional(conn)
    .await?;
    Ok(ligne)
}

pub async fn participant_par_email(
    conn: &mut PgConnection,
    meeting_id: Uuid,
    email: &str,
) -> Result<Option<ParticipantResume>> {
    let ligne = sqlx::query_as!(
        ParticipantResume,
        "SELECT id, meeting_id, registration_id, cancelled_at
           FROM live.meeting_participants
          WHERE meeting_id = $1 AND email = $2::text::platform.email",
        meeting_id,
        email
    )
    .fetch_optional(conn)
    .await?;
    Ok(ligne)
}

pub struct NouveauParticipant<'a> {
    pub meeting_id: Uuid,
    pub registration_id: Uuid,
    pub person_id: Uuid,
    pub display_name: &'a str,
    pub email: &'a str,
    pub first_name: &'a str,
    pub last_name: &'a str,
}

impl NouveauParticipant<'_> {
    /// Ce qui partira chez Zoom, rejouable à l'identique (`request_payload`).
    pub fn charge(&self) -> Value {
        json!({ "email": self.email, "first_name": self.first_name, "last_name": self.last_name })
    }
}

/// L'insertion déclenche la mise en file de l'enregistrement chez Zoom.
pub async fn inserer_participant(
    conn: &mut PgConnection,
    p: &NouveauParticipant<'_>,
) -> Result<Uuid> {
    let id = sqlx::query_scalar!(
        "INSERT INTO live.meeting_participants
             (meeting_id, registration_id, person_id, role, display_name, email, request_payload)
         VALUES ($1, $2, $3, 'attendee', $4, $5::text::platform.email, $6)
         RETURNING id",
        p.meeting_id,
        p.registration_id,
        p.person_id,
        p.display_name,
        p.email,
        p.charge()
    )
    .fetch_one(conn)
    .await?;
    Ok(id)
}

/// Reprend une ligne existante (réinscription, autre réunion) et la remet en
/// file : le déclencheur ne réagit qu'à l'insertion.
pub async fn reprendre_participant(
    conn: &mut PgConnection,
    participant_id: Uuid,
    p: &NouveauParticipant<'_>,
) -> Result<()> {
    sqlx::query!(
        "UPDATE live.meeting_participants
            SET external_registrant_id = CASE WHEN meeting_id = $2 THEN external_registrant_id END,
                personal_join_url      = CASE WHEN meeting_id = $2 THEN personal_join_url END,
                meeting_id = $2, registration_id = $3, person_id = $4, display_name = $5,
                email = $6::text::platform.email, request_payload = $7,
                cancelled_at = NULL, sync_status = 'pending', sync_attempts = 0,
                last_error = NULL, next_retry_at = now(), synced_at = NULL
          WHERE id = $1",
        participant_id,
        p.meeting_id,
        p.registration_id,
        p.person_id,
        p.display_name,
        p.email,
        p.charge()
    )
    .execute(&mut *conn)
    .await?;
    mettre_en_file(conn, SYNC_PARTICIPANT, participant_id, Some(p.meeting_id)).await
}

/// Pose `cancelled_at` et dépose l'annulation chez Zoom. Faux : déjà annulée.
pub async fn annuler_participant(conn: &mut PgConnection, participant_id: Uuid) -> Result<bool> {
    let annule = sqlx::query!(
        "UPDATE live.meeting_participants
            SET cancelled_at = now(), next_retry_at = NULL
          WHERE id = $1 AND cancelled_at IS NULL",
        participant_id
    )
    .execute(&mut *conn)
    .await?
    .rows_affected()
        == 1;
    if annule {
        mettre_en_file(conn, CANCEL_PARTICIPANT, participant_id, None).await?;
    }
    Ok(annule)
}

/// Clé horodatée : `platform.jobs` garde la clé d'un travail réussi, une clé
/// fixe interdirait toute seconde synchronisation du même participant.
pub async fn mettre_en_file(
    conn: &mut PgConnection,
    tache: &'static str,
    participant_id: Uuid,
    meeting_id: Option<Uuid>,
) -> Result<()> {
    mettre_en_file_a(conn, tache, participant_id, meeting_id, None).await
}

pub async fn mettre_en_file_a(
    conn: &mut PgConnection,
    tache: &'static str,
    participant_id: Uuid,
    meeting_id: Option<Uuid>,
    quand: Option<OffsetDateTime>,
) -> Result<()> {
    let mut job = NewJob::new(
        tache,
        json!({ "participant_id": participant_id, "meeting_id": meeting_id }),
    )
    .idempotent(format!(
        "{participant_id}:{}",
        OffsetDateTime::now_utc().unix_timestamp_nanos()
    ));
    if let Some(quand) = quand {
        job = job.at(quand);
    }
    job.queue = FILE;
    jobs::enqueue(conn, job).await?;
    Ok(())
}

pub async fn requeue_failed(conn: &mut PgConnection, meeting_id: Uuid) -> Result<i32> {
    let n = sqlx::query_scalar!(
        r#"SELECT live.requeue_failed_participants($1, 5000, true) AS "n!""#,
        meeting_id
    )
    .fetch_one(conn)
    .await?;
    Ok(n)
}

/// L'écran du back-office : la réunion et ses compteurs d'inscrits.
pub async fn tableau(
    conn: &mut PgConnection,
    session_id: Uuid,
) -> Result<Option<crate::domain::meeting::AdminSessionMeeting>> {
    let ligne = sqlx::query_as!(
        crate::domain::meeting::AdminSessionMeeting,
        r#"SELECT m.id, m.session_id AS "session_id!", m.provider::text AS "provider!",
                  m.external_id, m.status::text AS "status!",
                  m.sync_status::text AS "sync_status!", m.last_sync_error,
                  m.join_url::text AS join_url,
                  count(p.id) FILTER (WHERE p.cancelled_at IS NULL) AS "participants_total!",
                  count(p.id) FILTER (WHERE p.cancelled_at IS NULL AND p.sync_status = 'synced')
                      AS "participants_synced!",
                  count(p.id) FILTER (WHERE p.cancelled_at IS NULL
                                        AND p.sync_status IN ('failed', 'abandoned'))
                      AS "participants_failed!"
             FROM live.meetings m
             LEFT JOIN live.meeting_participants p ON p.meeting_id = m.id
            WHERE m.session_id = $1 AND m.status NOT IN ('cancelled', 'failed')
            GROUP BY m.id"#,
        session_id
    )
    .fetch_optional(conn)
    .await?;
    Ok(ligne)
}

pub struct ReunionPublique {
    pub provider: String,
    pub status: String,
    pub actual_end_at: Option<OffsetDateTime>,
}

pub async fn reunion_publique(
    conn: &mut PgConnection,
    session_id: Uuid,
) -> Result<Option<ReunionPublique>> {
    let ligne = sqlx::query_as!(
        ReunionPublique,
        r#"SELECT provider::text AS "provider!", status::text AS "status!", actual_end_at
             FROM live.meetings
            WHERE session_id = $1 AND status IN ('scheduled', 'started', 'ended')"#,
        session_id
    )
    .fetch_optional(conn)
    .await?;
    Ok(ligne)
}

/// La place d'une inscription, prête chez Zoom. Seul chemin qui lit le code
/// d'accès chiffré. Ne dépend pas de `sync_status` : une resynchronisation
/// pendant l'activité ne doit fermer la porte à personne.
pub struct Place {
    pub external_id: Option<String>,
    pub passcode_encrypted: Option<Vec<u8>>,
    pub display_name: String,
    pub email: String,
    pub personal_join_url: Option<String>,
}

pub async fn place(conn: &mut PgConnection, registration_id: Uuid) -> Result<Option<Place>> {
    let ligne = sqlx::query_as!(
        Place,
        r#"SELECT m.external_id, m.passcode_encrypted,
                  p.display_name, p.email::text AS "email!",
                  p.personal_join_url::text AS personal_join_url
             FROM live.meeting_participants p
             JOIN live.meetings m ON m.id = p.meeting_id
            WHERE p.registration_id = $1
              AND p.cancelled_at IS NULL
              AND p.external_registrant_id IS NOT NULL
              AND p.personal_join_url IS NOT NULL
              AND m.external_id IS NOT NULL
              AND m.passcode_encrypted IS NOT NULL
              AND m.status IN ('scheduled', 'started')"#,
        registration_id
    )
    .fetch_optional(conn)
    .await?;
    Ok(ligne)
}

/// Filet de sécurité : relance les inscrits en échec des réunions qui
/// commencent dans les quarante-huit heures (ou sont en cours).
pub async fn relancer_les_proches(conn: &mut PgConnection) -> Result<i64> {
    let n = sqlx::query_scalar!(
        r#"SELECT COALESCE(sum(live.requeue_failed_participants(m.id, 5000, false)), 0)::bigint AS "n!"
             FROM live.meetings m
            WHERE m.sync_status = 'synced'
              AND m.status IN ('scheduled', 'started')
              AND m.scheduled_start_at < now() + interval '48 hours'
              AND m.scheduled_start_at
                  + make_interval(mins => COALESCE(m.duration_minutes, 0)) > now()"#
    )
    .fetch_one(conn)
    .await?;
    Ok(n)
}

/// Séance annulée : la réunion l'est aussi, et son annulation part chez Zoom.
pub async fn annuler_reunion(conn: &mut PgConnection, session_id: Uuid) -> Result<()> {
    let annulee = sqlx::query!(
        "UPDATE live.meetings
            SET status = 'cancelled', sync_status = 'not_required'
          WHERE session_id = $1 AND status NOT IN ('cancelled', 'failed')
        RETURNING id, external_id",
        session_id
    )
    .fetch_optional(&mut *conn)
    .await?;
    if let Some(m) = annulee.filter(|m| m.external_id.is_some()) {
        let mut job = NewJob::new(CANCEL_MEETING, json!({ "meeting_id": m.id }))
            .idempotent(format!("{}:annulation", m.id));
        job.queue = FILE;
        jobs::enqueue(conn, job).await?;
    }
    Ok(())
}

/// Séance déplacée : la réunion prend le nouvel horaire, et la relance le
/// reporte chez Zoom.
pub async fn recaler_reunion(
    conn: &mut PgConnection,
    session_id: Uuid,
    debut: OffsetDateTime,
    minutes: i32,
) -> Result<()> {
    let id = sqlx::query_scalar!(
        "UPDATE live.meetings
            SET scheduled_start_at = $2, duration_minutes = $3
          WHERE session_id = $1 AND status NOT IN ('cancelled', 'failed')
        RETURNING id",
        session_id,
        debut,
        minutes
    )
    .fetch_optional(&mut *conn)
    .await?;
    if let Some(id) = id {
        relancer_reunion(conn, id).await?;
    }
    Ok(())
}
