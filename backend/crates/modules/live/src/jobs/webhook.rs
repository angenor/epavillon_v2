//! `live.process_webhook_event` : appliquer une notification Zoom journalisée.
//! Une notification non vérifiée n'est jamais appliquée (080_live.sql, D4).

use async_trait::async_trait;
use kernel::db::Db;
use kernel::error::{ApiError, Result};
use kernel::jobs::{ClaimedJob, JobHandler};
use serde_json::Value;
use sqlx::postgres::PgConnection;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::repo::cross::programme;
use crate::repo::meetings::{FILE, PROCESS_WEBHOOK};
use crate::repo::webhooks;

pub struct ProcessWebhook {
    db: Db,
}

impl ProcessWebhook {
    pub fn new(db: Db) -> Self {
        Self { db }
    }
}

#[async_trait]
impl JobHandler for ProcessWebhook {
    fn task(&self) -> &'static str {
        PROCESS_WEBHOOK
    }

    fn queue(&self) -> &'static str {
        FILE
    }

    async fn run(&self, job: &ClaimedJob) -> Result<()> {
        let id: Uuid = serde_json::from_value(job.payload["webhook_event_id"].clone())
            .map_err(ApiError::internal)?;
        let mut tx = self.db.write(&job.context()).await?;
        let issue = traiter(&mut tx, id).await;
        match issue {
            Ok(()) => tx.commit().await,
            Err(e) => {
                tx.rollback().await?;
                let mut tx = self.db.write(&job.context()).await?;
                webhooks::clore(&mut tx, id, "failed", None, Some(&e.to_string())).await?;
                tx.commit().await?;
                Err(e)
            }
        }
    }
}

/// Applique la notification et clôt sa ligne, dans la transaction de l'appelant.
pub async fn traiter(conn: &mut PgConnection, id: Uuid) -> Result<()> {
    let Some(w) = webhooks::reserver(conn, id).await? else {
        return Ok(());
    };
    if !w.signature_verified {
        return webhooks::clore(conn, id, "ignored", None, None).await;
    }

    let objet = &w.payload["payload"]["object"];
    let numero = match &objet["id"] {
        Value::Number(n) => Some(n.to_string()),
        Value::String(s) => Some(s.clone()),
        _ => None,
    };
    let reunion = match &numero {
        Some(n) => webhooks::reunion_par_numero(conn, n).await?,
        None => None,
    };
    let Some(meeting_id) = reunion else {
        return webhooks::clore(conn, id, "ignored", None, None).await;
    };

    let participant = &objet["participant"];
    let applique = match w.event_type.as_str() {
        "meeting.started" => {
            let debut = instant(&objet["start_time"]);
            webhooks::demarrer(conn, meeting_id, objet["uuid"].as_str(), debut).await?;
            true
        }
        "meeting.ended" => {
            webhooks::terminer(conn, meeting_id, instant(&objet["end_time"])).await?;
            true
        }
        "meeting.participant_joined" => {
            let inscription = webhooks::participant_entre(
                conn,
                meeting_id,
                participant["registrant_id"]
                    .as_str()
                    .filter(|r| !r.is_empty()),
                participant["email"].as_str().filter(|e| !e.is_empty()),
                instant(&participant["join_time"]),
            )
            .await?;
            // Entrée libre : aucune inscription, `joined_at` suffit.
            if let Some(Some(registration_id)) = inscription {
                programme::noter_la_presence(conn, registration_id).await?;
            }
            true
        }
        "meeting.participant_left" => {
            webhooks::participant_sorti(
                conn,
                meeting_id,
                participant["registrant_id"]
                    .as_str()
                    .filter(|r| !r.is_empty()),
                participant["email"].as_str().filter(|e| !e.is_empty()),
                instant(&participant["leave_time"]),
            )
            .await?;
            true
        }
        _ => false,
    };

    let statut = if applique { "processed" } else { "ignored" };
    webhooks::clore(conn, id, statut, Some(meeting_id), None).await
}

fn instant(valeur: &Value) -> OffsetDateTime {
    valeur
        .as_str()
        .and_then(|t| OffsetDateTime::parse(t, &Rfc3339).ok())
        .unwrap_or_else(OffsetDateTime::now_utc)
}
