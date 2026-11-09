//! Les intervenants en ligne (08/11).
//!
//! `live.speaker_places` : les places Zoom des intervenants d'une activité et
//! le courriel de leur lien, demandés par le module Programmation (inscription,
//! renvoi par la régie) ou par le balayage ci-dessous.
//!
//! `live.speaker_links_sweep`, toutes les quinze minutes, seulement si le
//! réglage `live.speaker_links.auto` est vrai : le lien d'inscription au
//! porteur à J-2, le lien Zoom de chaque intervenant à J-1. Les clés
//! `<séance>:auto` font de chaque envoi d'office un envoi unique.

use async_trait::async_trait;
use contracts::engagement::{SpeakerSignupLinkEmail, EMAIL_QUEUE, SEND_SPEAKER_SIGNUP_LINK};
use contracts::live::{SpeakerPlaces, SPEAKER_PLACES};
use kernel::db::Db;
use kernel::error::{ApiError, Result};
use kernel::jobs::{self, ClaimedJob, JobHandler, NewJob};
use serde_json::json;
use sqlx::postgres::PgConnection;
use time::{Duration, OffsetDateTime};

use crate::repo::cross::intervenants as lecture;
use crate::repo::meetings::FILE;
use crate::service::intervenants;

pub const SPEAKER_LINKS_SWEEP: &str = "live.speaker_links_sweep";
const PAS_SECONDES: i64 = 15 * 60;

pub struct SpeakerPlacesJob {
    db: Db,
}

impl SpeakerPlacesJob {
    pub fn new(db: Db) -> Self {
        Self { db }
    }
}

#[async_trait]
impl JobHandler for SpeakerPlacesJob {
    fn task(&self) -> &'static str {
        SPEAKER_PLACES
    }

    fn queue(&self) -> &'static str {
        FILE
    }

    async fn run(&self, job: &ClaimedJob) -> Result<()> {
        let charge: SpeakerPlaces =
            serde_json::from_value(job.payload.clone()).map_err(ApiError::internal)?;
        let mut tx = self.db.write(&job.context()).await?;
        let demandes = intervenants::donner_les_places(
            &mut tx,
            charge.session_id,
            charge.speaker_ids.as_deref(),
            charge.auto,
        )
        .await?;
        tx.commit().await?;
        tracing::info!(session = %charge.session_id, demandes, "liens Zoom d'intervenants demandés");
        Ok(())
    }
}

pub struct SpeakerLinksSweep {
    db: Db,
}

impl SpeakerLinksSweep {
    pub fn new(db: Db) -> Self {
        Self { db }
    }
}

#[async_trait]
impl JobHandler for SpeakerLinksSweep {
    fn task(&self) -> &'static str {
        SPEAKER_LINKS_SWEEP
    }

    fn queue(&self) -> &'static str {
        FILE
    }

    async fn run(&self, job: &ClaimedJob) -> Result<()> {
        let mut tx = self.db.write(&job.context()).await?;
        let maintenant = OffsetDateTime::now_utc();
        if lecture::envoi_doffice(&mut tx).await? {
            balayer(&mut tx, maintenant).await?;
        }
        planifier(&mut tx, prochaine_occurrence(maintenant)).await?;
        tx.commit().await
    }
}

async fn balayer(conn: &mut PgConnection, maintenant: OffsetDateTime) -> Result<()> {
    for s in lecture::seances_avant(conn, maintenant + Duration::hours(48)).await? {
        if !s.has_owner {
            continue;
        }
        let charge = SpeakerSignupLinkEmail {
            session_id: s.id,
            preview_to: None,
        };
        let job = NewJob {
            queue: EMAIL_QUEUE,
            ..NewJob::new(
                SEND_SPEAKER_SIGNUP_LINK,
                serde_json::to_value(&charge).map_err(ApiError::internal)?,
            )
        }
        .idempotent(format!("{}:auto", s.id));
        jobs::enqueue(conn, job).await?;
    }
    for s in lecture::seances_avant(conn, maintenant + Duration::hours(24)).await? {
        let charge = SpeakerPlaces {
            session_id: s.id,
            speaker_ids: None,
            auto: true,
        };
        let job = NewJob {
            queue: FILE,
            ..NewJob::new(
                SPEAKER_PLACES,
                serde_json::to_value(&charge).map_err(ApiError::internal)?,
            )
        }
        .idempotent(format!("{}:auto", s.id));
        jobs::enqueue(conn, job).await?;
    }
    Ok(())
}

pub async fn planifier(conn: &mut PgConnection, moment: OffsetDateTime) -> Result<bool> {
    let mut job = NewJob::new(SPEAKER_LINKS_SWEEP, json!({}))
        .idempotent(format!("{SPEAKER_LINKS_SWEEP}:{}", moment.unix_timestamp()))
        .at(moment);
    job.queue = FILE;
    Ok(jobs::enqueue(conn, job).await?.is_some())
}

pub fn prochaine_occurrence(depuis: OffsetDateTime) -> OffsetDateTime {
    let suivant = (depuis.unix_timestamp().div_euclid(PAS_SECONDES) + 1) * PAS_SECONDES;
    OffsetDateTime::from_unix_timestamp(suivant).unwrap_or(depuis)
}
