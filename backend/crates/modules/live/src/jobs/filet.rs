//! `live.requeue_participants`, toutes les quinze minutes : relance les inscrits
//! restés en échec (quota de Zoom dépassé, panne) des réunions des quarante-huit
//! prochaines heures. Un refus définitif de Zoom (`abandoned`) n'est pas relancé
//! ici ; le back-office le relance à la main.

use async_trait::async_trait;
use kernel::db::Db;
use kernel::error::Result;
use kernel::jobs::{self, ClaimedJob, JobHandler, NewJob};
use serde_json::json;
use sqlx::postgres::PgConnection;
use time::OffsetDateTime;

use crate::repo::meetings::{self, FILE};

pub const REQUEUE_PARTICIPANTS: &str = "live.requeue_participants";
const PAS_SECONDES: i64 = 15 * 60;

pub struct RequeueParticipants {
    db: Db,
}

impl RequeueParticipants {
    pub fn new(db: Db) -> Self {
        Self { db }
    }
}

#[async_trait]
impl JobHandler for RequeueParticipants {
    fn task(&self) -> &'static str {
        REQUEUE_PARTICIPANTS
    }

    fn queue(&self) -> &'static str {
        FILE
    }

    async fn run(&self, job: &ClaimedJob) -> Result<()> {
        let mut tx = self.db.write(&job.context()).await?;
        let relances = meetings::relancer_les_proches(&mut tx).await?;
        // La suivante naît dans la même transaction : la chaîne ne se rompt pas.
        planifier(&mut tx, prochaine_occurrence(OffsetDateTime::now_utc())).await?;
        tx.commit().await?;
        if relances > 0 {
            tracing::info!(relances, "inscrits Zoom en échec relancés");
        }
        Ok(())
    }
}

pub async fn planifier(conn: &mut PgConnection, moment: OffsetDateTime) -> Result<bool> {
    let mut job = NewJob::new(REQUEUE_PARTICIPANTS, json!({}))
        .idempotent(format!(
            "{REQUEUE_PARTICIPANTS}:{}",
            moment.unix_timestamp()
        ))
        .at(moment);
    job.queue = FILE;
    Ok(jobs::enqueue(conn, job).await?.is_some())
}

pub fn prochaine_occurrence(depuis: OffsetDateTime) -> OffsetDateTime {
    let suivant = (depuis.unix_timestamp().div_euclid(PAS_SECONDES) + 1) * PAS_SECONDES;
    OffsetDateTime::from_unix_timestamp(suivant).unwrap_or(depuis)
}
