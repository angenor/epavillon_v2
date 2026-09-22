//! Purge récurrente des essais de code — **quatre-vingt-dix jours**.
//!
//! Assez pour constater une série d'échecs et comprendre ce qui s'est passé
//! pendant une COP ; trop court pour que la table devienne un journal de
//! fréquentation de l'application. C'est ce que `COMMENT ON TABLE
//! invitation_code_attempts` annonce, et ce fichier est ce qui le rend vrai.
//!
//! Le travail se **replanifie lui-même**, comme la purge des jetons : rien dans
//! le noyau ne porte de récurrence, et une boucle de plus dans le worker serait
//! un second ordonnanceur à surveiller. La clé d'unicité porte le jour visé.
//!
//! Ce que cela ne rattrape pas, et c'est écrit : une purge morte après ses cinq
//! essais n'a pas posé la suivante, et sa clé occupe déjà l'index. La chaîne se
//! réarme au **démarrage du worker**, qui repose le créneau du jour — c'est
//! pourquoi la planification de départ est là-bas et pas ici.

use async_trait::async_trait;
use kernel::db::Db;
use kernel::error::Result;
use kernel::jobs::{self, ClaimedJob, JobHandler, NewJob};
use serde_json::json;
use sqlx::postgres::PgConnection;
use time::{Duration, OffsetDateTime, Time};

pub const PURGE_ATTEMPTS: &str = "negotiation.purge_invitation_attempts";

/// La durée de conservation, en jours.
const RETENTION_JOURS: i64 = 90;

pub struct PurgeInvitationAttempts {
    db: Db,
}

impl PurgeInvitationAttempts {
    pub fn new(db: Db) -> Self {
        Self { db }
    }
}

#[async_trait]
impl JobHandler for PurgeInvitationAttempts {
    fn task(&self) -> &'static str {
        PURGE_ATTEMPTS
    }

    async fn run(&self, job: &ClaimedJob) -> Result<()> {
        let mut tx = self.db.write(&job.context()).await?;

        let supprimes = sqlx::query!(
            "DELETE FROM negotiation.invitation_code_attempts
              WHERE attempted_at < now() - make_interval(days => $1)",
            RETENTION_JOURS as i32
        )
        .execute(&mut *tx)
        .await?
        .rows_affected();

        // La suivante naît dans la MÊME transaction que la purge : une purge
        // validée sans sa suivante romprait la chaîne en silence.
        planifier(&mut tx, prochaine_occurrence(OffsetDateTime::now_utc())).await?;

        tx.commit().await?;
        tracing::info!(supprimes, "essais de code d'invitation purgés");
        Ok(())
    }
}

/// Pose le créneau du jour visé, s'il n'existe pas déjà. Faux : il était posé.
pub async fn planifier(conn: &mut PgConnection, moment: OffsetDateTime) -> Result<bool> {
    let pose = jobs::enqueue(
        conn,
        NewJob::new(PURGE_ATTEMPTS, json!({}))
            .idempotent(format!("{PURGE_ATTEMPTS}:{}", moment.date()))
            .at(moment),
    )
    .await?;

    Ok(pose.is_some())
}

/// Minuit UTC du lendemain — le même ancrage que la purge des jetons : « dans
/// vingt-quatre heures » ferait dériver le créneau d'un redémarrage à l'autre
/// jusqu'à tomber en pleine journée.
pub fn prochaine_occurrence(depuis: OffsetDateTime) -> OffsetDateTime {
    (depuis.date() + Duration::days(1))
        .with_time(Time::MIDNIGHT)
        .assume_utc()
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    #[test]
    fn la_prochaine_occurrence_est_minuit_du_lendemain() {
        assert_eq!(
            prochaine_occurrence(datetime!(2026-11-08 23:59:59 UTC)),
            datetime!(2026-11-09 00:00:00 UTC)
        );
    }
}
