//! `live.sync_meeting` : créer la réunion chez Zoom, ou relire celle qu'on a
//! rattachée, puis relancer les inscrits qui l'attendaient.
//!
//! La ligne est réservée (`in_progress`) avant tout appel : deux travaux pour
//! la même réunion ne créent jamais deux réunions Zoom.

use async_trait::async_trait;
use kernel::db::Db;
use kernel::error::{ApiError, Result};
use kernel::jobs::{ClaimedJob, JobHandler};
use serde_json::{json, Value};
use std::sync::Arc;
use time::format_description::well_known::Rfc3339;
use time::{OffsetDateTime, UtcOffset};
use uuid::Uuid;

use crate::repo::meetings::{self, FILE, SYNC_MEETING};
use crate::zoom::{Zoom, ZoomError};

pub struct SyncMeeting {
    db: Db,
    zoom: Arc<Zoom>,
}

impl SyncMeeting {
    pub fn new(db: Db, zoom: Arc<Zoom>) -> Self {
        Self { db, zoom }
    }
}

/// Les champs de Zoom qui valent authentification : jamais dans `provider_payload`.
const SECRETS: [&str; 5] = [
    "password",
    "h323_password",
    "pstn_password",
    "encrypted_password",
    "start_url",
];

pub fn sans_secrets(reponse: &Value) -> Value {
    let mut copie = reponse.clone();
    if let Some(objet) = copie.as_object_mut() {
        for cle in SECRETS {
            objet.remove(cle);
        }
    }
    copie
}

/// Inscription obligatoire, approbation automatique : chaque inscrit reçoit
/// tout de suite son lien personnel, et Zoom ferme la porte à la jauge.
pub fn corps_de_creation(
    topic: &str,
    agenda: Option<&str>,
    debut: Option<OffsetDateTime>,
    duree: Option<i32>,
    fuseau: &str,
) -> Value {
    let debut = debut.and_then(|d| {
        d.to_offset(UtcOffset::UTC)
            .replace_nanosecond(0)
            .ok()?
            .format(&Rfc3339)
            .ok()
    });
    json!({
        "type": 2,
        "topic": topic,
        "agenda": agenda.map(|a| a.chars().take(2000).collect::<String>()),
        "start_time": debut,
        "duration": duree,
        "timezone": fuseau,
        "settings": {
            "approval_type": 0,
            "registration_type": 1,
            "registrants_email_notification": true,
            "registrants_confirmation_email": true,
            "join_before_host": false,
            "waiting_room": false,
            "meeting_authentication": false,
        },
    })
}

struct AReserver {
    external_id: Option<String>,
    topic: Option<String>,
    agenda: Option<String>,
    scheduled_start_at: Option<OffsetDateTime>,
    duration_minutes: Option<i32>,
    timezone: String,
}

#[async_trait]
impl JobHandler for SyncMeeting {
    fn task(&self) -> &'static str {
        SYNC_MEETING
    }

    fn queue(&self) -> &'static str {
        FILE
    }

    async fn run(&self, job: &ClaimedJob) -> Result<()> {
        let meeting_id: Uuid = serde_json::from_value(job.payload["meeting_id"].clone())
            .map_err(ApiError::internal)?;

        let mut tx = self.db.write(&job.context()).await?;
        let reservee = sqlx::query_as!(
            AReserver,
            r#"UPDATE live.meetings
                  SET sync_status = 'in_progress', sync_attempts = sync_attempts + 1
                WHERE id = $1
                  AND (sync_status IN ('pending', 'failed')
                       OR (sync_status = 'in_progress' AND updated_at < now() - interval '10 minutes'))
            RETURNING external_id, topic, agenda, scheduled_start_at, duration_minutes,
                      timezone::text AS "timezone!""#,
            meeting_id
        )
        .fetch_optional(&mut *tx)
        .await?;
        tx.commit().await?;

        // Déjà synchronisée, ou prise par un autre worker.
        let Some(r) = reservee else {
            return Ok(());
        };

        match self.appeler_zoom(&r).await {
            Ok(reunion) => self.enregistrer(job, meeting_id, &reunion).await,
            Err(e) => {
                let dernier = job.attempts >= job.max_attempts || !e.est_passagere();
                let statut = if dernier { "abandoned" } else { "failed" };
                let mut tx = self.db.write(&job.context()).await?;
                sqlx::query!(
                    "UPDATE live.meetings
                        SET sync_status = $2::text::live.sync_status, last_sync_error = $3
                      WHERE id = $1",
                    meeting_id,
                    statut,
                    e.to_string()
                )
                .execute(&mut *tx)
                .await?;
                tx.commit().await?;
                if e.est_passagere() {
                    Err(e.into())
                } else {
                    tracing::warn!(reunion = %meeting_id, erreur = %e, "réunion refusée par Zoom");
                    Ok(())
                }
            }
        }
    }
}

impl SyncMeeting {
    async fn appeler_zoom(&self, r: &AReserver) -> std::result::Result<Value, ZoomError> {
        let client = &self.zoom.client;
        match &r.external_id {
            Some(numero) => {
                let reunion = client.lire_reunion(numero).await?;
                if reunion["settings"]["approval_type"].as_i64() == Some(0) {
                    return Ok(reunion);
                }
                // Réunion créée à la main sans inscription, ou à approbation
                // manuelle : sans approbation automatique, pas de lien personnel.
                client
                    .modifier_reunion(numero, &json!({ "settings": { "approval_type": 0 } }))
                    .await?;
                client.lire_reunion(numero).await
            }
            None => {
                let topic = r.topic.as_deref().unwrap_or("Pavillon de la Francophonie");
                client
                    .creer_reunion(&corps_de_creation(
                        topic,
                        r.agenda.as_deref(),
                        r.scheduled_start_at,
                        r.duration_minutes,
                        &r.timezone,
                    ))
                    .await
            }
        }
    }

    async fn enregistrer(&self, job: &ClaimedJob, meeting_id: Uuid, reunion: &Value) -> Result<()> {
        let numero = match &reunion["id"] {
            Value::Number(n) => n.to_string(),
            Value::String(s) => s.clone(),
            _ => return Err(ApiError::internal("réponse de Zoom sans numéro de réunion")),
        };
        let coffre = &self.zoom.coffre;
        let passcode = reunion["password"]
            .as_str()
            .filter(|p| !p.is_empty())
            .map(|p| coffre.chiffrer(p))
            .transpose()?;
        let lien_hote = reunion["start_url"]
            .as_str()
            .filter(|p| !p.is_empty())
            .map(|p| coffre.chiffrer(p))
            .transpose()?;
        let url = |cle: &str| {
            reunion[cle]
                .as_str()
                .filter(|u| u.starts_with("https://") || u.starts_with("http://"))
                .map(str::to_owned)
        };

        let mut tx = self.db.write(&job.context()).await?;
        sqlx::query!(
            "UPDATE live.meetings
                SET external_id = $2, external_uuid = $3, provider_account = $4,
                    provider_payload = $5, join_url = $6::text::platform.url,
                    registration_url = $7::text::platform.url,
                    passcode_encrypted = $8, start_url_encrypted = $9,
                    host_email = $10::text::platform.email,
                    status = CASE WHEN status = 'draft' THEN 'scheduled' ELSE status END,
                    sync_status = 'synced', last_synced_at = now(), last_sync_error = NULL
              WHERE id = $1",
            meeting_id,
            numero,
            reunion["uuid"].as_str(),
            reunion["host_id"].as_str(),
            sans_secrets(reunion),
            url("join_url"),
            url("registration_url"),
            passcode,
            lien_hote,
            reunion["host_email"].as_str().filter(|e| e.contains('@')),
        )
        .execute(&mut *tx)
        .await?;
        meetings::requeue_failed(&mut tx, meeting_id).await?;
        tx.commit().await
    }
}
