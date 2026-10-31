//! `live.incident_window` : un message d'incident apparaît ou expire à l'heure
//! dite sans qu'aucune ligne ne change. La base dépose ce travail à chaque borne
//! future de la fenêtre (`live.tg_incidents_signal()`, `080_live.sql` § 6) ; il
//! rejoue le signal des pages publiques, qui relisent leurs bandeaux.
//!
//! File `live_incidents`, servie avec ou sans Zoom : la file « live » ne l'est
//! qu'avec Zoom configuré, et un travail sans gestionnaire y échouerait.

use async_trait::async_trait;
use kernel::db::Db;
use kernel::error::{ApiError, Result};
use kernel::jobs::{ClaimedJob, JobHandler};
use uuid::Uuid;

pub const INCIDENT_WINDOW: &str = "live.incident_window";
pub const FILE_INCIDENTS: &str = "live_incidents";

pub struct IncidentWindow {
    db: Db,
}

impl IncidentWindow {
    pub fn new(db: Db) -> Self {
        Self { db }
    }
}

#[async_trait]
impl JobHandler for IncidentWindow {
    fn task(&self) -> &'static str {
        INCIDENT_WINDOW
    }

    fn queue(&self) -> &'static str {
        FILE_INCIDENTS
    }

    async fn run(&self, job: &ClaimedJob) -> Result<()> {
        let incident_id: Uuid = serde_json::from_value(job.payload["incident_id"].clone())
            .map_err(ApiError::internal)?;
        let mut tx = self.db.write(&job.context()).await?;
        sqlx::query!(
            r#"SELECT 1 AS "un!" FROM live.signal_incident($1)"#,
            incident_id
        )
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await
    }
}
