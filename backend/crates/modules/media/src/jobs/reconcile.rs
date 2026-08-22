//! **La réconciliation des compteurs de quota.**
//!
//! Le compteur de `media.storage_quotas` est incrémental, tenu par déclencheur :
//! il doit l'être, parce qu'un quota s'oppose **au moment** du téléversement et
//! qu'une vue rafraîchie périodiquement autoriserait un dépassement entre deux
//! rafraîchissements. Un compteur incrémental dérive pourtant — une absorption
//! d'organisation, une restauration, un déclencheur désactivé le temps d'une
//! migration. `media.reconcile_storage_quotas()` le réaligne sur la
//! consommation réelle, et c'est la fonction du modèle qui le fait : la
//! réécrire ici ferait une seconde définition de la consommation.
//!
//! **Le nombre de lignes corrigées est tracé.** Zéro est l'état sain ; un chiffre
//! qui monte d'un passage à l'autre dit qu'un déclencheur ne fait plus son
//! travail, et c'est la seule façon de le voir.
//!
//! Même patron de récurrence que la purge : le travail se replanifie lui-même,
//! sur une grille ancrée à l'époque Unix, et le démarrage du worker le réarme.

use async_trait::async_trait;
use kernel::db::Db;
use kernel::error::Result;
use kernel::jobs::{self, ClaimedJob, JobHandler, NewJob};
use serde_json::json;
use sqlx::postgres::PgConnection;
use std::time::Duration;
use time::OffsetDateTime;

use crate::repo::quotas;

pub const RECONCILE_QUOTAS: &str = "media.reconcile_quotas";

pub struct ReconcileQuotas {
    db: Db,
    intervalle: Duration,
}

impl ReconcileQuotas {
    pub fn new(db: Db, intervalle: Duration) -> Self {
        Self { db, intervalle }
    }
}

#[async_trait]
impl JobHandler for ReconcileQuotas {
    fn task(&self) -> &'static str {
        RECONCILE_QUOTAS
    }

    fn queue(&self) -> &'static str {
        super::process::QUEUE
    }

    async fn run(&self, job: &ClaimedJob) -> Result<()> {
        let mut tx = self.db.write(&job.context()).await?;

        let corrigees = quotas::reconcilier(&mut tx).await?;
        // La suivante naît dans la MÊME transaction que la réconciliation : une
        // réconciliation validée sans sa suivante romprait la chaîne en silence.
        planifier(
            &mut tx,
            prochaine_occurrence(OffsetDateTime::now_utc(), self.intervalle),
        )
        .await?;

        tx.commit().await?;
        if corrigees > 0 {
            tracing::warn!(
                corrigees,
                "compteurs de quota réalignés sur la consommation réelle"
            );
        }
        Ok(())
    }
}

/// Pose le créneau visé, s'il n'existe pas déjà. Faux : il était posé.
pub async fn planifier(conn: &mut PgConnection, moment: OffsetDateTime) -> Result<bool> {
    // **La file est celle que le gestionnaire déclare.** `NewJob::new()` pose la
    // file par défaut, qu'aucun gestionnaire de ce module n'écoute : le travail
    // s'empilerait sans erreur et sans trace, exactement le défaut trouvé en
    // phase 4.
    let pose = jobs::enqueue(
        conn,
        NewJob {
            queue: super::process::QUEUE,
            ..NewJob::new(RECONCILE_QUOTAS, json!({}))
        }
        .idempotent(format!("{RECONCILE_QUOTAS}:{}", moment.unix_timestamp()))
        .at(moment),
    )
    .await?;

    Ok(pose.is_some())
}

pub fn prochaine_occurrence(depuis: OffsetDateTime, intervalle: Duration) -> OffsetDateTime {
    super::purge::prochaine_occurrence(depuis, intervalle)
}
