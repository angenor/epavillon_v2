//! `live.provider_webhook_events` : écrire d'abord, traiter ensuite (D4).

use kernel::error::Result;
use serde_json::Value;
use sqlx::postgres::PgConnection;
use time::OffsetDateTime;
use uuid::Uuid;

pub struct NouveauWebhook<'a> {
    pub event_type: &'a str,
    pub payload_hash: Option<Vec<u8>>,
    pub signature: Option<&'a str>,
    pub signature_verified: bool,
    pub external_id: Option<String>,
    pub payload: &'a Value,
    pub headers: Value,
    pub occurred_at: Option<OffsetDateTime>,
}

/// `None` : notification déjà reçue (même empreinte), Zoom la rejoue.
pub async fn journaliser(conn: &mut PgConnection, w: &NouveauWebhook<'_>) -> Result<Option<Uuid>> {
    let statut = if w.signature_verified {
        "received"
    } else {
        "ignored"
    };
    let id = sqlx::query_scalar!(
        "INSERT INTO live.provider_webhook_events
             (provider, event_type, payload_hash, signature, signature_verified,
              external_id, payload, headers, occurred_at, status)
         VALUES ('zoom', $1, $2, $3, $4, $5, $6, $7, $8, $9::text::live.webhook_status)
         ON CONFLICT DO NOTHING
         RETURNING id",
        w.event_type,
        w.payload_hash,
        w.signature,
        w.signature_verified,
        w.external_id,
        w.payload,
        w.headers,
        w.occurred_at,
        statut
    )
    .fetch_optional(conn)
    .await?;
    Ok(id)
}

pub struct WebhookLu {
    pub event_type: String,
    pub payload: Value,
    pub signature_verified: bool,
    pub status: String,
}

pub async fn reserver(conn: &mut PgConnection, id: Uuid) -> Result<Option<WebhookLu>> {
    let ligne = sqlx::query_as!(
        WebhookLu,
        r#"UPDATE live.provider_webhook_events
              SET status = 'processing', attempts = attempts + 1
            WHERE id = $1 AND status IN ('received', 'failed')
        RETURNING event_type, payload, signature_verified, status::text AS "status!""#,
        id
    )
    .fetch_optional(conn)
    .await?;
    Ok(ligne)
}

pub async fn clore(
    conn: &mut PgConnection,
    id: Uuid,
    statut: &str,
    meeting_id: Option<Uuid>,
    erreur: Option<&str>,
) -> Result<()> {
    sqlx::query!(
        "UPDATE live.provider_webhook_events
            SET status = $2::text::live.webhook_status, meeting_id = COALESCE($3, meeting_id),
                last_error = $4,
                processed_at = CASE WHEN $2 IN ('processed', 'ignored') THEN now() END
          WHERE id = $1",
        id,
        statut,
        meeting_id,
        erreur
    )
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn reunion_par_numero(conn: &mut PgConnection, numero: &str) -> Result<Option<Uuid>> {
    let id = sqlx::query_scalar!(
        "SELECT id FROM live.meetings WHERE provider = 'zoom' AND external_id = $1",
        numero
    )
    .fetch_optional(conn)
    .await?;
    Ok(id)
}

pub async fn demarrer(
    conn: &mut PgConnection,
    meeting_id: Uuid,
    occurrence: Option<&str>,
    debut: OffsetDateTime,
) -> Result<()> {
    sqlx::query!(
        "UPDATE live.meetings
            SET status = 'started', actual_start_at = COALESCE(actual_start_at, $2),
                actual_end_at = NULL, external_uuid = COALESCE($3, external_uuid)
          WHERE id = $1 AND status IN ('scheduled', 'started', 'ended')",
        meeting_id,
        debut,
        occurrence
    )
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn terminer(
    conn: &mut PgConnection,
    meeting_id: Uuid,
    fin: OffsetDateTime,
) -> Result<()> {
    sqlx::query!(
        "UPDATE live.meetings
            SET status = 'ended', actual_end_at = GREATEST($2, actual_start_at)
          WHERE id = $1 AND status IN ('scheduled', 'started', 'ended')",
        meeting_id,
        fin
    )
    .execute(conn)
    .await?;
    Ok(())
}

/// Rapproche par l'identifiant d'inscrit, à défaut par l'adresse. Rend
/// l'inscription plateforme, pour noter la présence.
pub async fn participant_entre(
    conn: &mut PgConnection,
    meeting_id: Uuid,
    registrant_id: Option<&str>,
    email: Option<&str>,
    instant: OffsetDateTime,
) -> Result<Option<Option<Uuid>>> {
    let ligne = sqlx::query_scalar!(
        "UPDATE live.meeting_participants
            SET joined_at = $4, left_at = NULL
          WHERE id = (SELECT id FROM live.meeting_participants
                       WHERE meeting_id = $1 AND cancelled_at IS NULL
                         AND (external_registrant_id = $2
                              OR ($2::text IS NULL AND lower(email::text) = lower($3)))
                       LIMIT 1)
        RETURNING registration_id",
        meeting_id,
        registrant_id,
        email,
        instant
    )
    .fetch_optional(conn)
    .await?;
    Ok(ligne)
}

pub async fn participant_sorti(
    conn: &mut PgConnection,
    meeting_id: Uuid,
    registrant_id: Option<&str>,
    email: Option<&str>,
    instant: OffsetDateTime,
) -> Result<()> {
    sqlx::query!(
        "UPDATE live.meeting_participants
            SET left_at = $4
          WHERE id = (SELECT id FROM live.meeting_participants
                       WHERE meeting_id = $1 AND cancelled_at IS NULL
                         AND (external_registrant_id = $2
                              OR ($2::text IS NULL AND lower(email::text) = lower($3)))
                       LIMIT 1)",
        meeting_id,
        registrant_id,
        email,
        instant
    )
    .execute(conn)
    .await?;
    Ok(())
}
