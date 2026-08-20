//! Le balayage de détection : **par tranches, avec curseur, et se
//! replanifiant**.
//!
//! Cinq mille appels d'une recherche à quelques dizaines de millisecondes font
//! une à trois minutes : c'est acceptable la nuit, mais pas dans une seule
//! transaction, et pas dans un travail qu'un redémarrage ferait reprendre de
//! zéro. Chaque tranche pose donc la suivante avec son curseur, et la dernière
//! planifie le passage du lendemain (research.md § R11).
//!
//! **La clé d'unicité porte le jour et le curseur**, ce qui rend le rejeu
//! inoffensif — le motif de la purge récurrente de B1, réemployé tel quel.
//!
//! Le démarrage du worker ne fait que **réarmer** la chaîne, au cas où sa
//! dernière occurrence serait morte avant d'avoir posé la suivante.

use async_trait::async_trait;
use kernel::config::Config;
use kernel::db::Db;
use kernel::error::{ApiError, Result};
use kernel::jobs::{self, ClaimedJob, JobHandler, NewJob};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::postgres::PgConnection;
use time::{Duration, OffsetDateTime, Time};
use uuid::Uuid;

use crate::domain::ids::OrganizationId;
use crate::repo::duplicates;
use crate::repo::search::{self, SearchInput};

pub const SCAN_DUPLICATES: &str = "org.duplicates.scan";

/// Ce que la tranche porte : le jour du passage, et où elle reprend.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Charge {
    /// Le jour du passage, en toutes lettres — il entre dans la clé d'unicité.
    jour: String,
    /// Dernière fiche examinée. `None` : la première tranche du jour.
    #[serde(default)]
    apres: Option<Uuid>,
}

/// Pose la première tranche d'un passage, si elle n'existe pas déjà.
///
/// Faux : elle était posée. C'est ce que le démarrage du worker appelle — dix
/// redémarrages dans la journée ne produisent pas dix balayages.
pub async fn planifier(conn: &mut PgConnection, moment: OffsetDateTime) -> Result<bool> {
    let jour = moment.date().to_string();

    let pose = jobs::enqueue(
        conn,
        NewJob::new(SCAN_DUPLICATES, json!({ "jour": jour, "apres": null }))
            .idempotent(cle(&jour, None))
            .at(moment),
    )
    .await?;

    Ok(pose.is_some())
}

/// Minuit UTC du lendemain. L'ancrage sur l'horloge plutôt que sur « dans
/// vingt-quatre heures » évite que le créneau dérive d'un redémarrage à l'autre
/// jusqu'à tomber en pleine journée — la leçon de la purge de B1.
pub fn prochaine_occurrence(depuis: OffsetDateTime) -> OffsetDateTime {
    (depuis.date() + Duration::days(1))
        .with_time(Time::MIDNIGHT)
        .assume_utc()
}

/// La clé d'unicité d'une tranche : le jour **et** le curseur.
///
/// Sans le curseur, la deuxième tranche du jour se heurterait à la clé de la
/// première et le balayage s'arrêterait après deux cents fiches, sans rien dire.
fn cle(jour: &str, apres: Option<Uuid>) -> String {
    match apres {
        Some(id) => format!("{SCAN_DUPLICATES}:{jour}:{id}"),
        None => format!("{SCAN_DUPLICATES}:{jour}:debut"),
    }
}

pub struct ScanDuplicates {
    db: Db,
    seuil: f64,
    tranche: i64,
}

impl ScanDuplicates {
    pub fn new(db: Db, config: &Config) -> Self {
        Self {
            db,
            seuil: config.org.duplicate_score_threshold as f64,
            tranche: config.org.duplicate_scan_batch as i64,
        }
    }
}

#[async_trait]
impl JobHandler for ScanDuplicates {
    fn task(&self) -> &'static str {
        SCAN_DUPLICATES
    }

    async fn run(&self, job: &ClaimedJob) -> Result<()> {
        let charge: Charge = serde_json::from_value(job.payload.clone())
            .map_err(|e| ApiError::internal(format!("charge utile illisible : {e}")))?;

        let fiches =
            duplicates::tranche_a_balayer(self.db.pool(), charge.apres, self.tranche).await?;

        if fiches.is_empty() {
            // Le passage est fini : on planifie celui du lendemain.
            let mut tx = self.db.write(&job.context()).await?;
            planifier(&mut tx, prochaine_occurrence(OffsetDateTime::now_utc())).await?;
            tx.commit().await?;

            tracing::info!(jour = %charge.jour, "balayage des doublons terminé");
            return Ok(());
        }

        let mut consignees = 0_u32;
        let dernier = fiches.last().map(|f| f.id);

        for fiche in &fiches {
            // **La lecture non filtrée**, celle du back-office : le domaine
            // partagé doit faire entrer la fiche, c'est le signal le plus fiable
            // du modèle.
            let voisines = search::brute(
                self.db.pool(),
                SearchInput {
                    name: &fiche.legal_name,
                    country_id: fiche.country_id,
                    email: fiche.contact_email.as_deref(),
                    website: fiche.website.as_deref(),
                    limit: 10,
                },
            )
            .await?;

            let mut tx = self.db.write(&job.context()).await?;
            for voisine in voisines {
                let autre = voisine.organization_id.as_uuid();
                if autre == fiche.id || voisine.score < self.seuil {
                    continue;
                }

                if duplicates::consigner(
                    &mut tx,
                    OrganizationId(fiche.id),
                    voisine.organization_id,
                    voisine.score,
                    &voisine.match_reasons,
                )
                .await?
                {
                    consignees += 1;
                }
            }
            tx.commit().await?;
        }

        // **La tranche suivante naît dans la même transaction que rien** : elle
        // est posée après le travail, et sa clé d'unicité rend un rejeu
        // inoffensif. Si le worker meurt ici, le démarrage réarme la chaîne au
        // début du jour suivant — le pire cas est un passage manqué, jamais un
        // doublon consigné deux fois.
        let mut tx = self.db.write(&job.context()).await?;
        jobs::enqueue(
            &mut tx,
            NewJob::new(
                SCAN_DUPLICATES,
                json!({ "jour": charge.jour, "apres": dernier }),
            )
            .idempotent(cle(&charge.jour, dernier)),
        )
        .await?;
        tx.commit().await?;

        tracing::info!(
            jour = %charge.jour,
            examinees = fiches.len(),
            consignees,
            "tranche de balayage traitée"
        );
        Ok(())
    }
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
    }

    /// Deux tranches du même jour ont **deux clés différentes**. Sans le
    /// curseur, la seconde se heurterait à la clé de la première et le balayage
    /// s'arrêterait après une tranche, sans rien dire.
    #[test]
    fn le_curseur_entre_dans_la_cle_dunicite() {
        let jour = "2026-08-20";
        let premiere = cle(jour, None);
        let seconde = cle(jour, Some(Uuid::now_v7()));

        assert_ne!(premiere, seconde);
        assert!(premiere.contains(jour) && seconde.contains(jour));
    }
}
