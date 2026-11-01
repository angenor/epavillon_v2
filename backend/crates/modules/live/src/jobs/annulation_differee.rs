//! `live.cancel_session_meeting` : la réunion d'une séance annulée ne s'annule
//! chez Zoom que cinq minutes plus tard (régie du jour, 02/11). Zoom prévient
//! lui-même ses inscrits et le lien meurt avec la réunion : une annulation
//! défaite dans ce délai garde la même réunion et le même lien.

use async_trait::async_trait;
use kernel::db::Db;
use kernel::error::{ApiError, Result};
use kernel::jobs::{self, ClaimedJob, JobHandler, NewJob};
use serde_json::json;
use sqlx::postgres::PgConnection;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::repo::cross::programme;
use crate::repo::meetings::{self, FILE};

pub const CANCEL_SESSION_MEETING: &str = "live.cancel_session_meeting";
pub const DELAI_MINUTES: i64 = 5;

pub async fn poser(conn: &mut PgConnection, session_id: Uuid) -> Result<()> {
    let a = OffsetDateTime::now_utc() + Duration::minutes(DELAI_MINUTES);
    let mut job = NewJob::new(CANCEL_SESSION_MEETING, json!({ "session_id": session_id }))
        .idempotent(format!("{session_id}:{}", a.unix_timestamp()))
        .at(a);
    job.queue = FILE;
    jobs::enqueue(conn, job).await?;
    Ok(())
}

pub struct CancelSessionMeeting {
    db: Db,
}

impl CancelSessionMeeting {
    pub fn new(db: Db) -> Self {
        Self { db }
    }
}

#[async_trait]
impl JobHandler for CancelSessionMeeting {
    fn task(&self) -> &'static str {
        CANCEL_SESSION_MEETING
    }

    fn queue(&self) -> &'static str {
        FILE
    }

    async fn run(&self, job: &ClaimedJob) -> Result<()> {
        let session_id: Uuid = serde_json::from_value(job.payload["session_id"].clone())
            .map_err(ApiError::internal)?;
        let mut tx = self.db.write(&job.context()).await?;
        let annulee = programme::seance(&mut tx, session_id)
            .await?
            .is_some_and(|s| s.status == "cancelled");
        if annulee {
            meetings::annuler_reunion(&mut tx, session_id).await?;
        }
        tx.commit().await
    }
}
