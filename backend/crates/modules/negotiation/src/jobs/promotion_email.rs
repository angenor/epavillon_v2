//! Le courriel d'une place obtenue depuis la liste d'attente (research R8) :
//! un travail à part, parti aussitôt, pour qu'il ne se fonde pas dans le
//! courriel d'un changement. Il relit la place et la réunion en partant.

use async_trait::async_trait;
use contracts::negotiation::JOB_MEETING_PROMOTION_EMAIL;
use kernel::db::Db;
use kernel::error::{ApiError, Result};
use kernel::jobs::{self, ClaimedJob, JobHandler, NewJob};
use kernel::mail::Mailer;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgConnection;
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::jobs::change_email::Cible;
use crate::jobs::emails::remettre;
use crate::mail::{self, Envoi};
use crate::repo::courriel as depot;
use crate::repo::notifications as reglage;

#[derive(Serialize, Deserialize)]
struct Charge {
    meeting_id: Uuid,
    person_id: Uuid,
}

/// Une clé par promotion : la même personne peut regagner une place à la même
/// réunion après l'avoir quittée, et la clé ne se libère jamais.
pub fn cle(meeting_id: Uuid, person_id: Uuid) -> String {
    let instant = OffsetDateTime::now_utc().unix_timestamp_nanos();
    format!("promotion:{meeting_id}:{person_id}:{instant}")
}

pub async fn poser(conn: &mut PgConnection, meeting_id: Uuid, person_id: Uuid) -> Result<()> {
    let charge = serde_json::to_value(Charge {
        meeting_id,
        person_id,
    })
    .map_err(|e| ApiError::internal(format!("charge de courriel : {e}")))?;
    jobs::enqueue(
        conn,
        NewJob::new(JOB_MEETING_PROMOTION_EMAIL, charge).idempotent(cle(meeting_id, person_id)),
    )
    .await?;
    Ok(())
}

pub struct MeetingPromotionEmail {
    db: Db,
    mailer: Arc<dyn Mailer>,
    app_public_url: String,
}

impl MeetingPromotionEmail {
    pub fn new(db: Db, mailer: Arc<dyn Mailer>, app_public_url: String) -> Self {
        Self {
            db,
            mailer,
            app_public_url,
        }
    }
}

#[async_trait]
impl JobHandler for MeetingPromotionEmail {
    fn task(&self) -> &'static str {
        JOB_MEETING_PROMOTION_EMAIL
    }

    async fn run(&self, job: &ClaimedJob) -> Result<()> {
        let charge: Charge = serde_json::from_value(job.payload.clone())
            .map_err(|e| ApiError::internal(format!("charge de courriel illisible : {e}")))?;
        let identifiant = job.id.to_string();
        let message = {
            let mut conn = self.db.pool().acquire().await?;
            composer(&mut conn, &charge, &identifiant, &self.app_public_url).await?
        };
        match message {
            Some(m) => remettre(self.mailer.as_ref(), &m).await,
            None => Ok(()),
        }
    }
}

/// Désinscrite entre-temps, réunion annulée, accord éteint : rien.
async fn composer(
    conn: &mut PgConnection,
    c: &Charge,
    message_id: &str,
    app_public_url: &str,
) -> Result<Option<kernel::mail::OutgoingMail>> {
    if !reglage::accord(conn, c.person_id).await?
        || !depot::inscrite(conn, c.meeting_id, c.person_id).await?
    {
        return Ok(None);
    }
    let Some(personne) = depot::destinataire(conn, c.person_id).await? else {
        return Ok(None);
    };
    let Some(r) = depot::reunion_francophone(conn, c.meeting_id).await? else {
        return Ok(None);
    };
    if matches!(r.status.as_str(), "draft" | "cancelled") {
        return Ok(None);
    }
    let envoi = Envoi {
        message_id,
        to: &personne.email,
        locale: &personne.locale,
        app_public_url,
    };
    let chemin = Cible::FrancophoneMeeting.chemin(c.meeting_id);
    Ok(Some(mail::place_obtenue(&envoi, &r, &chemin)))
}
