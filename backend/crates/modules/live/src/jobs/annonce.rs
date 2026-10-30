//! `live.announce_meeting_start` : une réunion démarrée en répétition, plus
//! d'un quart d'heure avant sa séance, est annoncée à début − 15 min si elle
//! tourne encore. Le travail est déposé par la base, qui décide aussi
//! (`live.announce_meeting_start()`, `080_live.sql` § 2 bis).

use async_trait::async_trait;
use kernel::db::Db;
use kernel::error::{ApiError, Result};
use kernel::jobs::{ClaimedJob, JobHandler};
use sqlx::postgres::PgConnection;
use uuid::Uuid;

use crate::repo::meetings::FILE;

pub const ANNOUNCE_MEETING_START: &str = "live.announce_meeting_start";

pub struct AnnounceMeetingStart {
    db: Db,
}

impl AnnounceMeetingStart {
    pub fn new(db: Db) -> Self {
        Self { db }
    }
}

#[async_trait]
impl JobHandler for AnnounceMeetingStart {
    fn task(&self) -> &'static str {
        ANNOUNCE_MEETING_START
    }

    fn queue(&self) -> &'static str {
        FILE
    }

    async fn run(&self, job: &ClaimedJob) -> Result<()> {
        let meeting_id: Uuid = serde_json::from_value(job.payload["meeting_id"].clone())
            .map_err(ApiError::internal)?;
        let mut tx = self.db.write(&job.context()).await?;
        annoncer(&mut tx, meeting_id, job.payload["occurrence"].as_str()).await?;
        tx.commit().await
    }
}

/// Vrai si le direct est annoncé ; faux si la réunion s'est arrêtée, a été
/// relancée, ou si la séance a été déplacée plus tard (nouvelle échéance).
pub async fn annoncer(
    conn: &mut PgConnection,
    meeting_id: Uuid,
    occurrence: Option<&str>,
) -> Result<bool> {
    let annonce = sqlx::query_scalar!(
        r#"SELECT live.announce_meeting_start($1, $2) AS "annonce!""#,
        meeting_id,
        occurrence
    )
    .fetch_one(conn)
    .await?;
    Ok(annonce)
}
