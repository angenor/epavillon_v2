//! **La purge** — le moment où un objet quitte réellement le stockage.
//!
//! # Le seul événement que ce module émet
//!
//! `media.schedule_asset_purge()` émet déjà `media.asset.purge_scheduled` : elle
//! annonce l'**intention**, jamais l'exécution. La disparition effective, elle,
//! n'est annoncée par personne — et sans elle, rien ne peut réagir à une perte
//! définitive. C'est ce fichier qui émet `media.asset.purged`, et il est le seul
//! du module à émettre quoi que ce soit.
//!
//! # Le travail se replanifie lui-même, sur le patron de B1
//!
//! Rien dans le noyau ne porte de récurrence, et une boucle de plus dans le
//! worker serait un second ordonnanceur à surveiller. La clé d'unicité porte
//! **le créneau visé**, calculé sur une grille ancrée à l'époque Unix : dix
//! redémarrages dans le même créneau n'en produisent pas dix purges. La chaîne
//! se réarme au démarrage du worker, qui repose le créneau courant.
//!
//! # Un objet fautif ne bloque pas le disque
//!
//! Chaque objet est purgé pour son compte, et un échec de stockage est **compté
//! et tracé** plutôt que rendu : un seul objet dont la clé résiste ne doit pas
//! empêcher les quarante autres de libérer leur place. Sa ligne garde
//! `purged_at` nul, et le passage suivant le reprend.
//!
//! # Une purge dont l'objet a déjà disparu ABOUTIT
//!
//! `delete()` est idempotente par contrat : supprimer ce qui n'existe pas est un
//! succès. L'objectif du travail est que l'objet ne soit plus là — pas qu'il ait
//! fallu l'enlever (FR-108).

use async_trait::async_trait;
use contracts::media as contrat;
use kernel::db::Db;
use kernel::error::Result;
use kernel::events::{self, DomainEvent};
use kernel::jobs::{self, ClaimedJob, JobHandler, NewJob};
use serde_json::json;
use sqlx::postgres::PgConnection;
use std::sync::Arc;
use std::time::Duration;
use time::OffsetDateTime;

use crate::repo::{assets, renditions};
use crate::storage::ObjectStore;

pub const PURGE_ASSETS: &str = "media.purge_assets";

/// Le plafond d'un passage. La file « à effacer » est minuscule par
/// construction — `ix_assets_purgeable` ne porte que les objets supprimés non
/// purgés —, mais un rattrapage après une longue panne ne doit pas tenir la
/// connexion une heure.
const PAR_PASSAGE: i64 = 200;

pub struct PurgeAssets {
    db: Db,
    storage: Arc<dyn ObjectStore>,
    intervalle: Duration,
}

impl PurgeAssets {
    pub fn new(db: Db, storage: Arc<dyn ObjectStore>, intervalle: Duration) -> Self {
        Self {
            db,
            storage,
            intervalle,
        }
    }
}

#[async_trait]
impl JobHandler for PurgeAssets {
    fn task(&self) -> &'static str {
        PURGE_ASSETS
    }

    fn queue(&self) -> &'static str {
        super::process::QUEUE
    }

    async fn run(&self, job: &ClaimedJob) -> Result<()> {
        let echus = assets::a_purger(self.db.pool(), PAR_PASSAGE).await?;

        let mut purges = 0_usize;
        let mut resistants = 0_usize;
        for objet in &echus {
            match self.purger(job, objet).await {
                Ok(()) => purges += 1,
                Err(erreur) => {
                    resistants += 1;
                    tracing::warn!(asset = %objet.id, erreur = %erreur, "purge impossible : reprise au passage suivant");
                }
            }
        }

        // La suivante naît **après** les purges, et dans sa propre transaction :
        // une purge validée sans sa suivante romprait la chaîne en silence, et
        // un objet fautif ne doit pas emporter la replanification avec lui.
        let mut tx = self.db.write(&job.context()).await?;
        planifier(
            &mut tx,
            prochaine_occurrence(OffsetDateTime::now_utc(), self.intervalle),
        )
        .await?;
        tx.commit().await?;

        if purges > 0 || resistants > 0 {
            tracing::info!(purges, resistants, "objets effacés du stockage");
        }
        Ok(())
    }
}

impl PurgeAssets {
    /// **Le stockage d'abord, la base ensuite.**
    ///
    /// L'inverse laisserait un objet daté comme purgé et pourtant présent sur le
    /// disque, que plus aucune lecture ne viserait : de l'espace perdu pour
    /// toujours. Dans cet ordre, un worker tué entre les deux rejoue simplement
    /// la suppression, qui est idempotente.
    async fn purger(&self, job: &ClaimedJob, objet: &assets::ObjetAPurger) -> Result<()> {
        let mut tx = self.db.write(&job.context()).await?;
        let declinaisons = renditions::cles_de(&mut tx, objet.id).await?;
        tx.commit().await?;

        for cle in &declinaisons {
            self.storage.delete(cle).await?;
        }
        self.storage.delete(&objet.object_key).await?;

        let mut tx = self.db.write(&job.context()).await?;
        renditions::effacer_de(&mut tx, objet.id).await?;

        // Faux : un autre passage l'a daté entre-temps. L'annonce ne part
        // qu'une fois, parce qu'un objet ne disparaît qu'une fois.
        if assets::marquer_purge(&mut tx, objet.id).await? {
            let charge = serde_json::to_value(contrat::AssetPurged {
                bucket: objet.bucket.clone(),
                object_key: objet.object_key.clone(),
                byte_size: objet.byte_size,
                rendition_bytes: objet.rendition_bytes,
                owner_organization_id: objet.owner_organization_id,
            })
            .map_err(kernel::error::ApiError::internal)?;

            events::emit(
                &mut tx,
                DomainEvent {
                    aggregate_schema: contrat::AGGREGATE_SCHEMA,
                    aggregate_type: contrat::AGGREGATE_ASSET,
                    aggregate_id: objet.id,
                    event_type: contrat::ASSET_PURGED,
                    payload: charge,
                },
            )
            .await?;
        }
        tx.commit().await?;

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
            ..NewJob::new(PURGE_ASSETS, json!({}))
        }
        .idempotent(format!("{PURGE_ASSETS}:{}", moment.unix_timestamp()))
        .at(moment),
    )
    .await?;

    Ok(pose.is_some())
}

/// Le prochain créneau de la grille, ancrée à l'époque Unix — le calcul de B3,
/// et pour la même raison : sans grille, le créneau dériverait d'un redémarrage
/// à l'autre et la clé d'unicité ne coïnciderait jamais deux fois.
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
    fn deux_demarrages_du_meme_creneau_visent_le_meme_instant() {
        let six_heures = Duration::from_secs(6 * 3600);
        assert_eq!(
            prochaine_occurrence(datetime!(2026-08-22 06:00:01 UTC), six_heures),
            prochaine_occurrence(datetime!(2026-08-22 11:59:59 UTC), six_heures)
        );
    }

    #[test]
    fn une_cadence_nulle_ne_fait_pas_tomber_le_calcul() {
        let depuis = datetime!(2026-08-22 10:00:00 UTC);
        assert!(prochaine_occurrence(depuis, Duration::ZERO) > depuis);
    }
}
