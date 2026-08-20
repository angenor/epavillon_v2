//! Rafraîchissement de la projection analytique.
//!
//! **En concurrence, donc hors transaction.** `REFRESH MATERIALIZED VIEW
//! CONCURRENTLY` ne peut pas s'exécuter dans un bloc de transaction : ce travail
//! ne peut donc pas accompagner l'écriture qui le rend nécessaire — et il n'a
//! pas à le faire. La liste du back-office relit sur la table vivante les quatre
//! colonnes qui bougent au geste de l'opérateur (FR-048), si bien que le retard
//! de la projection ne porte que sur des compteurs que personne ne regarde en
//! posant un sceau (research.md § R13).
//!
//! L'index unique que le rafraîchissement concurrent exige existe :
//! `ux_mv_organization_scorecard`.

use async_trait::async_trait;
use kernel::db::Db;
use kernel::error::Result;
use kernel::jobs::{self, ClaimedJob, JobHandler, NewJob};
use serde_json::json;
use sqlx::postgres::PgConnection;
use time::OffsetDateTime;

pub const REFRESH_SCORECARD: &str = "org.scorecard.refresh";

/// Met le rafraîchissement en file, coalescé sur une **fenêtre**.
///
/// La clé porte le début de la fenêtre courante : cent écritures dans les cinq
/// minutes produisent un rafraîchissement. Sur cinq mille organisations, la
/// projection se compte en secondes — l'enchaîner à chaque approbation
/// d'adhésion ferait tourner le worker en permanence pour un tableau de bord.
pub async fn planifier(conn: &mut PgConnection, fenetre: std::time::Duration) -> Result<()> {
    let secondes = fenetre.as_secs().max(1) as i64;
    let maintenant = OffsetDateTime::now_utc();
    let debut_de_fenetre = maintenant.unix_timestamp() / secondes;

    jobs::enqueue(
        conn,
        NewJob::new(REFRESH_SCORECARD, json!({}))
            .idempotent(format!("{REFRESH_SCORECARD}:{debut_de_fenetre}"))
            .at(maintenant + time::Duration::seconds(secondes)),
    )
    .await?;

    Ok(())
}

pub struct RefreshScorecard {
    db: Db,
}

impl RefreshScorecard {
    pub fn new(db: Db) -> Self {
        Self { db }
    }
}

#[async_trait]
impl JobHandler for RefreshScorecard {
    fn task(&self) -> &'static str {
        REFRESH_SCORECARD
    }

    async fn run(&self, _job: &ClaimedJob) -> Result<()> {
        // Pas de `db.write()` ici : la porte d'écriture ouvre une transaction, et
        // le rafraîchissement concurrent n'en supporte pas. Ce n'est pas une
        // entorse au principe VII — la projection n'est pas une table auditée,
        // et rien de ce qu'on y écrit ne réclame un auteur.
        sqlx::query("REFRESH MATERIALIZED VIEW CONCURRENTLY analytics.mv_organization_scorecard")
            .execute(self.db.pool())
            .await?;

        tracing::info!("fiche de performance des organisations rafraîchie");
        Ok(())
    }
}
