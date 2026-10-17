//! `live.cancel_meeting` : la séance est annulée, la réunion Zoom aussi. Zoom
//! prévient lui-même les inscrits par courriel.

use async_trait::async_trait;
use kernel::db::Db;
use kernel::error::{ApiError, Result};
use kernel::jobs::{ClaimedJob, JobHandler};
use std::sync::Arc;
use uuid::Uuid;

use crate::repo::meetings::{CANCEL_MEETING, FILE};
use crate::zoom::{Zoom, ZoomError};

pub struct CancelMeeting {
    db: Db,
    zoom: Arc<Zoom>,
}

impl CancelMeeting {
    pub fn new(db: Db, zoom: Arc<Zoom>) -> Self {
        Self { db, zoom }
    }
}

#[async_trait]
impl JobHandler for CancelMeeting {
    fn task(&self) -> &'static str {
        CANCEL_MEETING
    }

    fn queue(&self) -> &'static str {
        FILE
    }

    async fn run(&self, job: &ClaimedJob) -> Result<()> {
        let meeting_id: Uuid = serde_json::from_value(job.payload["meeting_id"].clone())
            .map_err(ApiError::internal)?;
        let numero = sqlx::query_scalar!(
            "SELECT external_id FROM live.meetings WHERE id = $1 AND status = 'cancelled'",
            meeting_id
        )
        .fetch_optional(self.db.pool())
        .await?
        .flatten();
        let Some(numero) = numero else {
            return Ok(());
        };
        match self.zoom.client.supprimer_reunion(&numero).await {
            Ok(()) => Ok(()),
            Err(e) if e.est_passagere() => Err(e.into()),
            Err(ZoomError::Refusee { statut: 404, .. }) => Ok(()),
            Err(e) => {
                tracing::warn!(reunion = %meeting_id, erreur = %e, "annulation refusée par Zoom");
                Ok(())
            }
        }
    }
}
