//! **La préparation des partitions mensuelles du journal d'expédition**
//! (écart n° 137).
//!
//! Le modèle amorce **trois mois** de partitions et annonce, en commentaire, un
//! worker de maintenance qui n'existait pas. Passé ce trimestre, tout tombe dans
//! la partition « fourre-tout » : aucune écriture n'échoue — c'est bien pour cela
//! qu'elle existe —, mais la **purge par bascule de partition**, seule raison du
//! partitionnement, cesse de fonctionner. Une table de plusieurs millions de
//! lignes se purgerait alors par `DELETE`, ce que le partitionnement était censé
//! éviter.
//!
//! **Rien n'est réécrit ici** : `platform.ensure_month_partition()` est
//! idempotente, sait détacher la partition par défaut, y déplacer les lignes du
//! mois visé et la rattacher. Ce travail ne fait que l'appeler, mois par mois.
//!
//! Même patron de récurrence que les trois chaînes de B1, B2 et B3 : le travail
//! pose sa propre occurrence suivante, la clé d'unicité porte **le jour visé**,
//! et le démarrage du worker réarme la chaîne.

use async_trait::async_trait;
use kernel::db::Db;
use kernel::error::Result;
use kernel::jobs::{self, ClaimedJob, JobHandler, NewJob};
use serde_json::json;
use sqlx::postgres::PgConnection;
use std::time::Duration;
use time::OffsetDateTime;

pub const ENSURE_PARTITIONS: &str = "engagement.ensure_partitions";

/// Combien de mois à venir sont préparés d'avance, en plus du mois courant.
///
/// Trois, comme l'amorçage du modèle : une panne du worker d'un trimestre
/// entier se rattrape sans qu'une seule ligne soit tombée dans la partition par
/// défaut.
const MOIS_DAVANCE: i64 = 3;

pub struct EnsurePartitions {
    db: Db,
    intervalle: Duration,
}

impl EnsurePartitions {
    pub fn new(db: Db, intervalle: Duration) -> Self {
        Self { db, intervalle }
    }
}

#[async_trait]
impl JobHandler for EnsurePartitions {
    fn task(&self) -> &'static str {
        ENSURE_PARTITIONS
    }

    async fn run(&self, job: &ClaimedJob) -> Result<()> {
        let mut tx = self.db.write(&job.context()).await?;

        // Le mois de départ vient de la BASE, jamais de l'horloge du processus :
        // les deux peuvent différer de quelques secondes, et un travail lancé le
        // 1er à minuit préparerait alors le mois précédent.
        let mut preparees = Vec::new();
        for decalage in 0..=MOIS_DAVANCE {
            let nom = sqlx::query_scalar!(
                r#"SELECT platform.ensure_month_partition(
                       'engagement', 'email_messages',
                       (date_trunc('month', now()) + make_interval(months => $1::int))::date
                   ) AS "nom!""#,
                decalage as i32
            )
            .fetch_one(&mut *tx)
            .await?;
            preparees.push(nom);
        }

        // La suivante naît dans la MÊME transaction : une préparation validée
        // sans sa suivante romprait la chaîne en silence.
        planifier(
            &mut tx,
            prochaine_occurrence(OffsetDateTime::now_utc(), self.intervalle),
        )
        .await?;

        tx.commit().await?;
        tracing::debug!(partitions = ?preparees, "partitions du journal d'expédition prêtes");
        Ok(())
    }
}

/// Pose le créneau visé, s'il n'existe pas déjà. Faux : il était posé.
pub async fn planifier(conn: &mut PgConnection, moment: OffsetDateTime) -> Result<bool> {
    let pose = jobs::enqueue(
        conn,
        NewJob::new(ENSURE_PARTITIONS, json!({}))
            .idempotent(format!("{ENSURE_PARTITIONS}:{}", moment.unix_timestamp()))
            .at(moment),
    )
    .await?;

    Ok(pose.is_some())
}

/// Le prochain créneau de la grille, ancrée à l'époque Unix. À la cadence
/// journalière par défaut, le créneau **est** le jour — et dix redémarrages dans
/// la journée n'en produisent pas dix.
pub fn prochaine_occurrence(depuis: OffsetDateTime, intervalle: Duration) -> OffsetDateTime {
    let pas = intervalle.as_secs().max(1) as i64;
    let suivant = (depuis.unix_timestamp().div_euclid(pas) + 1) * pas;
    OffsetDateTime::from_unix_timestamp(suivant).unwrap_or(depuis)
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    #[test]
    fn la_prochaine_occurrence_est_minuit_du_lendemain() {
        assert_eq!(
            prochaine_occurrence(
                datetime!(2026-08-22 23:59:59 UTC),
                Duration::from_secs(86_400)
            ),
            datetime!(2026-08-23 00:00:00 UTC)
        );
    }

    #[test]
    fn deux_demarrages_du_meme_jour_visent_le_meme_instant() {
        let jour = Duration::from_secs(86_400);
        assert_eq!(
            prochaine_occurrence(datetime!(2026-08-22 00:00:01 UTC), jour),
            prochaine_occurrence(datetime!(2026-08-22 23:59:59 UTC), jour)
        );
    }
}
