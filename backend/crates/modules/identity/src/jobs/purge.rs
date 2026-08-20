//! Purge récurrente des jetons à usage unique (FR-044).
//!
//! Le travail se **replanifie lui-même** : rien dans le noyau ne porte de
//! récurrence, et une boucle de plus dans le worker serait un second ordonnanceur
//! à surveiller. La clé d'unicité porte **le jour visé** — au plus une purge par
//! jour, quel que soit le nombre de redémarrages.
//!
//! Ce que cela ne rattrape pas, et c'est écrit : une purge qui meurt après ses
//! cinq essais n'a pas replanifié la suivante, et sa clé occupe déjà l'index.
//! La chaîne se réarme au **démarrage du worker**, qui repose le créneau du jour
//! — c'est pourquoi la planification de départ est là-bas et pas ici.

use async_trait::async_trait;
use kernel::db::Db;
use kernel::error::Result;
use kernel::jobs::{self, ClaimedJob, JobHandler, NewJob};
use kernel::tokens;
use serde_json::json;
use sqlx::postgres::PgConnection;
use time::{Duration, OffsetDateTime, Time};

pub const PURGE_EXPIRED_TOKENS: &str = "identity.purge_expired_tokens";

pub struct PurgeExpiredTokens {
    db: Db,
}

impl PurgeExpiredTokens {
    pub fn new(db: Db) -> Self {
        Self { db }
    }
}

#[async_trait]
impl JobHandler for PurgeExpiredTokens {
    fn task(&self) -> &'static str {
        PURGE_EXPIRED_TOKENS
    }

    async fn run(&self, job: &ClaimedJob) -> Result<()> {
        let mut tx = self.db.write(&job.context()).await?;

        let supprimes = tokens::purge(&mut tx).await?;
        // La suivante naît dans la MÊME transaction que la purge : une purge
        // validée sans sa suivante romprait la chaîne en silence.
        planifier(&mut tx, prochaine_occurrence(OffsetDateTime::now_utc())).await?;

        tx.commit().await?;
        tracing::info!(supprimes, "jetons périmés ou consommés purgés");
        Ok(())
    }
}

/// Pose le créneau du jour visé, s'il n'existe pas déjà. Faux : il était posé.
pub async fn planifier(conn: &mut PgConnection, moment: OffsetDateTime) -> Result<bool> {
    let pose = jobs::enqueue(
        conn,
        NewJob::new(PURGE_EXPIRED_TOKENS, json!({}))
            .idempotent(format!("{PURGE_EXPIRED_TOKENS}:{}", moment.date()))
            .at(moment),
    )
    .await?;

    Ok(pose.is_some())
}

/// Minuit UTC du lendemain. L'ancrage sur l'horloge plutôt que sur « dans
/// vingt-quatre heures » évite que le créneau dérive d'un redémarrage à l'autre
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
            prochaine_occurrence(datetime!(2026-08-20 23:59:59 UTC)),
            datetime!(2026-08-21 00:00:00 UTC)
        );
        assert_eq!(
            prochaine_occurrence(datetime!(2026-08-20 00:00:00 UTC)),
            datetime!(2026-08-21 00:00:00 UTC)
        );
    }
}
