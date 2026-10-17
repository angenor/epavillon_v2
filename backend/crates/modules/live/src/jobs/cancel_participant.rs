//! `live.cancel_meeting_participant` : une inscription annulée libère sa place
//! chez Zoom. Une réinscription survenue entre-temps (`cancelled_at` remis à
//! NULL) rend le travail sans objet.

use async_trait::async_trait;
use kernel::db::Db;
use kernel::error::{ApiError, Result};
use kernel::jobs::{ClaimedJob, JobHandler};
use std::sync::Arc;
use uuid::Uuid;

use crate::repo::meetings::{CANCEL_PARTICIPANT, FILE};
use crate::zoom::Zoom;

pub struct CancelParticipant {
    db: Db,
    zoom: Arc<Zoom>,
}

impl CancelParticipant {
    pub fn new(db: Db, zoom: Arc<Zoom>) -> Self {
        Self { db, zoom }
    }
}

#[async_trait]
impl JobHandler for CancelParticipant {
    fn task(&self) -> &'static str {
        CANCEL_PARTICIPANT
    }

    fn queue(&self) -> &'static str {
        FILE
    }

    async fn run(&self, job: &ClaimedJob) -> Result<()> {
        let participant_id: Uuid = serde_json::from_value(job.payload["participant_id"].clone())
            .map_err(ApiError::internal)?;

        let ligne = sqlx::query!(
            r#"SELECT m.external_id, p.external_registrant_id, p.email::text AS "email!",
                      (p.sync_status = 'in_progress'
                       AND p.last_attempt_at > now() - interval '10 minutes') AS "en_cours!"
                 FROM live.meeting_participants p
                 JOIN live.meetings m ON m.id = p.meeting_id
                WHERE p.id = $1 AND p.cancelled_at IS NOT NULL"#,
            participant_id
        )
        .fetch_optional(self.db.pool())
        .await?;

        let Some(ligne) = ligne else {
            return Ok(());
        };
        // Une inscription chez Zoom est en vol : son identifiant n'est pas
        // encore connu. On réessaie ; elle-même reposera ce travail en finissant.
        if ligne.en_cours {
            return Err(ApiError::with_message(
                kernel::error::ErrorCode::Conflict,
                "inscription Zoom en cours, annulation différée",
            ));
        }
        let (Some(numero), Some(registrant)) = (ligne.external_id, ligne.external_registrant_id)
        else {
            return Ok(());
        };

        match self
            .zoom
            .client
            .changer_statut_inscrit(&numero, "cancel", &registrant, &ligne.email)
            .await
        {
            Ok(()) => Ok(()),
            Err(e) if e.est_passagere() => Err(e.into()),
            Err(e) => {
                tracing::warn!(participant = %participant_id, erreur = %e, "annulation refusée par Zoom");
                Ok(())
            }
        }
    }
}
