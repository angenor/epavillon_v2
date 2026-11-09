//! La place Zoom d'un intervenant en ligne (08/11, `080_live.sql`) : une ligne
//! `panelist` sans inscription, reconnue par son adresse, et la demande du
//! courriel de son lien.

use contracts::engagement::{SpeakerZoomLinkEmail, EMAIL_QUEUE, SEND_SPEAKER_ZOOM_LINK};
use kernel::error::{ApiError, Result};
use kernel::jobs::{self, NewJob};
use serde_json::json;
use sqlx::postgres::PgConnection;
use time::OffsetDateTime;
use uuid::Uuid;

pub struct NouvelIntervenant<'a> {
    pub meeting_id: Uuid,
    pub person_id: Uuid,
    pub display_name: &'a str,
    pub email: &'a str,
    pub first_name: &'a str,
    pub last_name: &'a str,
}

/// L'insertion déclenche la mise en file de l'enregistrement chez Zoom.
pub async fn inserer(conn: &mut PgConnection, n: &NouvelIntervenant<'_>) -> Result<Uuid> {
    let id = sqlx::query_scalar!(
        "INSERT INTO live.meeting_participants
             (meeting_id, person_id, role, display_name, email, request_payload)
         VALUES ($1, $2, 'panelist', $3, $4::text::platform.email, $5)
         RETURNING id",
        n.meeting_id,
        n.person_id,
        n.display_name,
        n.email,
        json!({ "email": n.email, "first_name": n.first_name, "last_name": n.last_name })
    )
    .fetch_one(conn)
    .await?;
    Ok(id)
}

pub struct PlaceDIntervenant {
    pub id: Uuid,
    pub cancelled: bool,
    pub link_requested_at: Option<OffsetDateTime>,
    pub personal_join_url: Option<String>,
}

pub async fn place(
    conn: &mut PgConnection,
    meeting_id: Uuid,
    email: &str,
) -> Result<Option<PlaceDIntervenant>> {
    let ligne = sqlx::query_as!(
        PlaceDIntervenant,
        r#"SELECT id, cancelled_at IS NOT NULL AS "cancelled!", link_requested_at,
                  CASE WHEN sync_status = 'synced' THEN personal_join_url::text END AS personal_join_url
             FROM live.meeting_participants
            WHERE meeting_id = $1 AND email = $2::text::platform.email"#,
        meeting_id,
        email
    )
    .fetch_optional(conn)
    .await?;
    Ok(ligne)
}

/// Le courriel du lien, dû à cette place pour cette activité. Il part tout de
/// suite si le lien existe déjà, sinon à la fin de la synchronisation, sous la
/// même clé : un seul envoi par demande.
pub async fn demander_le_lien(
    conn: &mut PgConnection,
    participant_id: Uuid,
    session_id: Uuid,
) -> Result<()> {
    let ligne = sqlx::query!(
        r#"UPDATE live.meeting_participants
              SET link_requested_at = clock_timestamp(), speaker_session_id = $2
            WHERE id = $1
        RETURNING link_requested_at AS "demande!", sync_status = 'synced' AS "pret!""#,
        participant_id,
        session_id
    )
    .fetch_one(&mut *conn)
    .await?;
    if ligne.pret {
        poser_le_courriel(conn, participant_id, session_id, ligne.demande).await?;
    }
    Ok(())
}

pub async fn poser_le_courriel(
    conn: &mut PgConnection,
    participant_id: Uuid,
    session_id: Uuid,
    demande: OffsetDateTime,
) -> Result<()> {
    let charge = SpeakerZoomLinkEmail {
        participant_id: Some(participant_id),
        session_id,
        preview_to: None,
    };
    let job = NewJob {
        queue: EMAIL_QUEUE,
        ..NewJob::new(
            SEND_SPEAKER_ZOOM_LINK,
            serde_json::to_value(&charge).map_err(ApiError::internal)?,
        )
    }
    .idempotent(format!(
        "{participant_id}:{}",
        demande.unix_timestamp_nanos() / 1_000
    ));
    jobs::enqueue(conn, job).await?;
    Ok(())
}
