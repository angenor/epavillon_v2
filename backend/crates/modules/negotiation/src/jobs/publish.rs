//! La publication d'un signalement validé, trente secondes après la validation
//! (research R3, SC-004).
//!
//! Une seule transaction, la ligne tenue : le travail n'agit que si la
//! validation qui l'a posé est toujours là — même `decided_at`, rien de publié,
//! rien de retiré. Une annulation passée avant lui l'a effacée ; une annulation
//! qui arrive pendant qu'il tient la ligne attend, puis trouve `published_at`
//! posé et échoue. L'un ou l'autre, jamais les deux. Rejoué, il ne fait rien.

use async_trait::async_trait;
use contracts::negotiation::JOB_REPORT_PUBLISH;
use kernel::db::Db;
use kernel::error::{ApiError, Result};
use kernel::jobs::{self, ClaimedJob, JobHandler, NewJob};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgConnection;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::jobs::change_email;
use crate::notifications::emission;
use crate::repo::publication as depot;

#[derive(Serialize, Deserialize)]
struct Charge {
    report_id: Uuid,
    #[serde(with = "time::serde::rfc3339")]
    decided_at: OffsetDateTime,
}

/// Posé dans la transaction de la validation. `publier_a` vient de la base.
pub async fn poser(
    conn: &mut PgConnection,
    report_id: Uuid,
    decided_at: OffsetDateTime,
    publier_a: OffsetDateTime,
) -> Result<()> {
    let charge = serde_json::to_value(Charge {
        report_id,
        decided_at,
    })
    .map_err(|e| ApiError::internal(format!("charge de publication : {e}")))?;
    jobs::enqueue(
        conn,
        NewJob::new(JOB_REPORT_PUBLISH, charge)
            .idempotent(format!(
                "publish:{report_id}:{}",
                decided_at.unix_timestamp_nanos() / 1_000
            ))
            .at(publier_a),
    )
    .await?;
    Ok(())
}

pub struct PublishReport {
    db: Db,
}

impl PublishReport {
    pub fn new(db: Db) -> Self {
        Self { db }
    }
}

pub enum Issue {
    Publie,
    /// Annulé, retiré, déjà publié ou revalidé depuis : rien à faire.
    Rien,
}

/// Le travail lui-même, exposé pour que les tests le rejouent.
pub async fn publier(
    conn: &mut PgConnection,
    report_id: Uuid,
    decided_at: OffsetDateTime,
) -> Result<Issue> {
    let Some(t) = depot::tenir(conn, report_id).await? else {
        return Ok(Issue::Rien);
    };
    let encore_valide = t.status == "validated"
        && t.decided_at == Some(decided_at)
        && t.published_at.is_none()
        && t.withdrawn_at.is_none();
    if !encore_valide {
        return Ok(Issue::Rien);
    }

    depot::publier(conn, t.id).await?;
    let reunion = match t.meeting_id {
        None => Some(depot::creer_reunion(conn, t.id).await?),
        Some(_) => None,
    };
    if let Some(p) = emission::publication(conn, &t, reunion).await? {
        change_email::poser(conn, p.cible, p.id, &p.destinataires).await?;
    }
    emission::decision(conn, &t).await?;
    Ok(Issue::Publie)
}

#[async_trait]
impl JobHandler for PublishReport {
    fn task(&self) -> &'static str {
        JOB_REPORT_PUBLISH
    }

    async fn run(&self, job: &ClaimedJob) -> Result<()> {
        let charge: Charge = serde_json::from_value(job.payload.clone())
            .map_err(|e| ApiError::internal(format!("charge de publication illisible : {e}")))?;
        let mut tx = self.db.write(&job.context()).await?;
        let issue = publier(&mut tx, charge.report_id, charge.decided_at).await?;
        tx.commit().await?;
        if matches!(issue, Issue::Publie) {
            tracing::info!(report_id = %charge.report_id, "signalement publié");
        }
        Ok(())
    }
}
