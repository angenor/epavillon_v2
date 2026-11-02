//! Partitions et rétention des journaux de `platform` : `audit_log` et
//! `ops_log`.
//!
//! Jusqu'au 02/11/2026, aucun travail ne préparait les partitions de l'audit :
//! seul l'amorçage du modèle en créait deux, et tout tombait ensuite dans la
//! partition par défaut. Celles des modules (`engagement.email_messages`…)
//! restent à leur module. Même patron de récurrence que les autres chaînes : le
//! travail pose sa suivante, la clé d'unicité porte le jour visé.
//!
//! L'audit n'est jamais purgé ici : sa rétention n'est pas décidée.

use async_trait::async_trait;
use kernel::db::Db;
use kernel::error::Result;
use kernel::jobs::{self, ClaimedJob, JobHandler, NewJob};
use serde_json::json;
use sqlx::postgres::PgConnection;
use std::sync::Arc;
use std::time::Duration;
use time::OffsetDateTime;

pub const MAINTAIN_JOURNALS: &str = "platform.maintain_journals";
const JOUR: Duration = Duration::from_secs(86_400);
const MOIS_DAVANCE: i32 = 3;
const RETENTION_OPS_MOIS: i32 = 3;

pub struct MaintainJournals {
    db: Db,
}

pub fn job_handlers(db: Db) -> Vec<Arc<dyn JobHandler>> {
    vec![Arc::new(MaintainJournals { db })]
}

#[async_trait]
impl JobHandler for MaintainJournals {
    fn task(&self) -> &'static str {
        MAINTAIN_JOURNALS
    }

    async fn run(&self, job: &ClaimedJob) -> Result<()> {
        let mut tx = self.db.write(&job.context()).await?;

        // Le mois de départ vient de la base, jamais de l'horloge du processus.
        let mut preparees = Vec::new();
        for table in ["audit_log", "ops_log"] {
            for decalage in 0..=MOIS_DAVANCE {
                let nom = sqlx::query_scalar!(
                    r#"SELECT platform.ensure_month_partition(
                           'platform', $1,
                           (date_trunc('month', now()) + make_interval(months => $2::int))::date
                       ) AS "nom!""#,
                    table,
                    decalage
                )
                .fetch_one(&mut *tx)
                .await?;
                preparees.push(nom);
            }
        }

        let supprimees = sqlx::query_scalar!(
            r#"SELECT platform.purge_ops_log($1) AS "supprimees!""#,
            RETENTION_OPS_MOIS
        )
        .fetch_one(&mut *tx)
        .await?;

        planifier(&mut tx, prochaine_occurrence(OffsetDateTime::now_utc())).await?;
        tx.commit().await?;

        if !supprimees.is_empty() {
            tracing::info!(partitions = ?supprimees, "journal d'exploitation : partitions purgées");
        }
        tracing::debug!(partitions = ?preparees, "partitions des journaux prêtes");
        Ok(())
    }
}

/// Pose le créneau visé, s'il n'existe pas déjà. Faux : il était posé.
pub async fn planifier(conn: &mut PgConnection, moment: OffsetDateTime) -> Result<bool> {
    let pose = jobs::enqueue(
        conn,
        NewJob::new(MAINTAIN_JOURNALS, json!({}))
            .idempotent(format!("{MAINTAIN_JOURNALS}:{}", moment.unix_timestamp()))
            .at(moment),
    )
    .await?;
    Ok(pose.is_some())
}

/// Au démarrage : un passage tout de suite si aucun n'a eu lieu aujourd'hui
/// (la clé est celle du créneau de minuit du jour), puis le créneau suivant.
/// Sans cela, les partitions de l'audit attendraient le prochain minuit.
pub async fn armer(conn: &mut PgConnection, maintenant: OffsetDateTime) -> Result<bool> {
    let pas = JOUR.as_secs() as i64;
    let jour = maintenant.unix_timestamp().div_euclid(pas) * pas;
    let immediat = jobs::enqueue(
        &mut *conn,
        NewJob::new(MAINTAIN_JOURNALS, json!({}))
            .idempotent(format!("{MAINTAIN_JOURNALS}:{jour}"))
            .at(maintenant),
    )
    .await?
    .is_some();
    let suivant = planifier(conn, prochaine_occurrence(maintenant)).await?;
    Ok(immediat || suivant)
}

/// Minuit UTC suivant.
pub fn prochaine_occurrence(depuis: OffsetDateTime) -> OffsetDateTime {
    let pas = JOUR.as_secs() as i64;
    let suivant = (depuis.unix_timestamp().div_euclid(pas) + 1) * pas;
    OffsetDateTime::from_unix_timestamp(suivant).unwrap_or(depuis)
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    #[test]
    fn la_prochaine_occurrence_est_minuit_suivant() {
        assert_eq!(
            prochaine_occurrence(datetime!(2026-11-02 13:05:00 UTC)),
            datetime!(2026-11-03 00:00:00 UTC)
        );
    }
}
