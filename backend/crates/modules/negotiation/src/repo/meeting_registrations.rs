//! Les inscriptions aux réunions de la Francophonie. Le statut écrit n'est
//! qu'une intention : `tg_validate_meeting_registration` décide inscrite ou en
//! attente, sous le verrou de la réunion, ou refuse.

use kernel::error::{ApiError, Result};
use sqlx::postgres::PgConnection;
use uuid::Uuid;

use crate::domain::meeting_registrations::{
    EtatDInscription, MeetingRegistration, MeetingRegistrationState, VideoLink,
};
use crate::repo::meetings::refus;

pub struct ReunionVerrouillee {
    pub programmee: bool,
    pub commencee: bool,
}

/// La réunion, verrouillée **avant** toute ligne d'inscription : la promotion
/// prend le même ordre, et deux désinscriptions simultanées ne s'interbloquent
/// pas.
pub async fn verrouiller(
    conn: &mut PgConnection,
    meeting_id: Uuid,
) -> Result<Option<ReunionVerrouillee>> {
    let ligne = sqlx::query!(
        r#"SELECT (status = 'scheduled') AS "programmee!", (now() >= start_at) AS "commencee!"
             FROM negotiation.meetings
            WHERE id = $1
              AND kind IN ('preparatory_workshop', 'francophone_consultation')
              AND source_key IS NULL
              FOR UPDATE"#,
        meeting_id
    )
    .fetch_optional(conn)
    .await?;
    Ok(ligne.map(|l| ReunionVerrouillee {
        programmee: l.programmee,
        commencee: l.commencee,
    }))
}

pub struct Ligne {
    pub status: EtatDInscription,
    pub waitlist_position: Option<i32>,
    pub client_ref: Option<Uuid>,
}

impl Ligne {
    pub fn etat(&self) -> MeetingRegistrationState {
        MeetingRegistrationState {
            status: self.status,
            waitlist_position: self.waitlist_position,
        }
    }
}

pub async fn ligne(
    conn: &mut PgConnection,
    meeting_id: Uuid,
    person_id: Uuid,
) -> Result<Option<Ligne>> {
    let ligne = sqlx::query!(
        r#"SELECT status::text AS "status!", waitlist_position, client_ref
             FROM negotiation.meeting_registrations
            WHERE meeting_id = $1 AND person_id = $2"#,
        meeting_id,
        person_id
    )
    .fetch_optional(conn)
    .await?;
    Ok(ligne.map(|l| Ligne {
        status: EtatDInscription::from_db(&l.status),
        waitlist_position: l.waitlist_position,
        client_ref: l.client_ref,
    }))
}

fn refus_dinscription(erreur: sqlx::Error) -> ApiError {
    if kernel::pg_error::constraint(&erreur) == Some("ux_meeting_registrations_client_ref") {
        return ApiError::validation(
            "Cette référence a déjà servi à un autre geste.",
            "client_ref",
        );
    }
    refus(erreur)
}

pub async fn inserer(
    conn: &mut PgConnection,
    meeting_id: Uuid,
    person_id: Uuid,
    client_ref: Uuid,
) -> Result<MeetingRegistrationState> {
    let l = sqlx::query!(
        r#"INSERT INTO negotiation.meeting_registrations (meeting_id, person_id, client_ref)
           VALUES ($1, $2, $3)
           RETURNING status::text AS "status!", waitlist_position"#,
        meeting_id,
        person_id,
        client_ref
    )
    .fetch_one(conn)
    .await
    .map_err(refus_dinscription)?;
    Ok(MeetingRegistrationState {
        status: EtatDInscription::from_db(&l.status),
        waitlist_position: l.waitlist_position,
    })
}

/// `cancelled` → une entrée : la base refait tous ses contrôles.
pub async fn reinscrire(
    conn: &mut PgConnection,
    meeting_id: Uuid,
    person_id: Uuid,
    client_ref: Uuid,
) -> Result<MeetingRegistrationState> {
    let l = sqlx::query!(
        r#"UPDATE negotiation.meeting_registrations
              SET status = 'registered', client_ref = $3
            WHERE meeting_id = $1 AND person_id = $2
           RETURNING status::text AS "status!", waitlist_position"#,
        meeting_id,
        person_id,
        client_ref
    )
    .fetch_one(conn)
    .await
    .map_err(refus_dinscription)?;
    Ok(MeetingRegistrationState {
        status: EtatDInscription::from_db(&l.status),
        waitlist_position: l.waitlist_position,
    })
}

pub async fn desinscrire(conn: &mut PgConnection, meeting_id: Uuid, person_id: Uuid) -> Result<()> {
    sqlx::query!(
        "UPDATE negotiation.meeting_registrations
            SET status = 'cancelled'
          WHERE meeting_id = $1 AND person_id = $2 AND status <> 'cancelled'",
        meeting_id,
        person_id
    )
    .execute(conn)
    .await
    .map_err(refus)?;
    Ok(())
}

/// Les personnes promues, à prévenir.
pub async fn promouvoir(conn: &mut PgConnection, meeting_id: Uuid) -> Result<Vec<Uuid>> {
    let promues = sqlx::query_scalar!(
        r#"SELECT p AS "p!" FROM negotiation.promote_meeting_waitlist($1) AS p"#,
        meeting_id
    )
    .fetch_all(conn)
    .await?;
    Ok(promues)
}

/// Les inscriptions actives de la personne sur l'édition, brouillons exclus.
pub async fn mes_inscriptions(
    conn: &mut PgConnection,
    person_id: Uuid,
    event_id: Uuid,
) -> Result<Vec<MeetingRegistration>> {
    let lignes = sqlx::query!(
        r#"SELECT r.meeting_id, r.status::text AS "status!", r.waitlist_position, r.client_ref,
                  r.registered_at
             FROM negotiation.meeting_registrations r
             JOIN negotiation.meetings m ON m.id = r.meeting_id
            WHERE r.person_id = $1 AND m.event_id = $2
              AND m.kind IN ('preparatory_workshop', 'francophone_consultation')
              AND m.source_key IS NULL AND m.status <> 'draft'
              AND r.status <> 'cancelled'
            ORDER BY m.start_at, m.id"#,
        person_id,
        event_id
    )
    .fetch_all(conn)
    .await?;
    Ok(lignes
        .into_iter()
        .map(|l| MeetingRegistration {
            meeting_id: l.meeting_id,
            status: EtatDInscription::from_db(&l.status),
            waitlist_position: l.waitlist_position,
            client_ref: l.client_ref,
            registered_at: l.registered_at,
        })
        .collect())
}

/// Le lien de chaque réunion où la personne est inscrite, et de chaque réunion
/// sans inscription (research R6). Jamais pour la liste d'attente. L'accès
/// négociateur se vérifie avant.
pub async fn liens(
    conn: &mut PgConnection,
    person_id: Uuid,
    event_id: Uuid,
) -> Result<Vec<VideoLink>> {
    let lignes = sqlx::query!(
        r#"SELECT m.id, m.external_url::text AS "url!"
             FROM negotiation.meetings m
             LEFT JOIN negotiation.meeting_registrations r
                    ON r.meeting_id = m.id AND r.person_id = $1
            WHERE m.event_id = $2
              AND m.kind IN ('preparatory_workshop', 'francophone_consultation')
              AND m.source_key IS NULL AND m.status NOT IN ('draft', 'cancelled')
              AND m.external_url IS NOT NULL
              AND (NOT m.requires_registration OR r.status = 'registered')
            ORDER BY m.start_at, m.id"#,
        person_id,
        event_id
    )
    .fetch_all(conn)
    .await?;
    Ok(lignes
        .into_iter()
        .map(|l| VideoLink {
            meeting_id: l.id,
            url: l.url,
        })
        .collect())
}
