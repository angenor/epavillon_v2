//! La traduction des titres de l'édition (research R11), posée par l'import.
//!
//! Un échec finit le travail sans écrire le lot et **sans nouvel essai** : le
//! travail réussit côté file, et la lecture suivante le reposera. Les lots déjà
//! traduits restent écrits.

use async_trait::async_trait;
use kernel::db::Db;
use kernel::error::{ApiError, Result};
use kernel::jobs::{self, ClaimedJob, JobHandler, NewJob};
use serde::Deserialize;
use serde_json::json;
use sqlx::postgres::PgConnection;
use std::sync::Arc;
use uuid::Uuid;

use crate::import::traduction::{EchecTraduction, Traducteur, TAILLE_DU_LOT};
use crate::repo::{import as depot, traductions};

pub const TRANSLATE_SESSION_TITLES: &str = "negotiation.translate_session_titles";

#[derive(Deserialize)]
struct Charge {
    event_id: Uuid,
}

pub struct TranslateSessionTitles {
    db: Db,
    /// Aucun sans clé : l'import ne pose alors pas ce travail.
    traducteur: Option<Arc<dyn Traducteur>>,
}

impl TranslateSessionTitles {
    pub fn new(db: Db, traducteur: Option<Arc<dyn Traducteur>>) -> Self {
        Self { db, traducteur }
    }
}

#[async_trait]
impl JobHandler for TranslateSessionTitles {
    fn task(&self) -> &'static str {
        TRANSLATE_SESSION_TITLES
    }

    async fn run(&self, job: &ClaimedJob) -> Result<()> {
        let charge: Charge = serde_json::from_value(job.payload.clone()).map_err(|e| {
            ApiError::internal(format!("charge du travail de traduction illisible : {e}"))
        })?;
        let Some(traducteur) = &self.traducteur else {
            tracing::info!("aucune clé de traduction : les titres restent en anglais");
            return Ok(());
        };

        let (modele, titres) = {
            let mut conn = self.db.pool().acquire().await?;
            let modele = traductions::modele(&mut conn).await?;
            let titres = depot::titres_sans_traduction(&mut conn, charge.event_id).await?;
            (modele, titres)
        };
        let Some(modele) = modele else {
            tracing::warn!(
                reglage = traductions::CLE_MODELE,
                "modèle de rédaction absent : rien n'est traduit"
            );
            return Ok(());
        };

        let mut ecrites = 0;
        for lot in titres.chunks(TAILLE_DU_LOT) {
            let traduits = traducteur
                .traduire(&modele, lot)
                .await
                .and_then(|t| aligne(lot.len(), t));
            let traduits = match traduits {
                Ok(t) => t,
                Err(echec) => {
                    tracing::warn!(event_id = %charge.event_id, erreur = %echec, "traduction manquée");
                    break;
                }
            };
            let mut tx = self.db.write(&job.context()).await?;
            ecrites += traductions::ecrire(&mut tx, lot, &traduits, &modele).await?;
            tx.commit().await?;
        }
        tracing::info!(event_id = %charge.event_id, titres = titres.len(), ecrites, "titres traduits");
        Ok(())
    }
}

/// Le traducteur promet l'alignement ; le travail ne le croit pas sur parole.
fn aligne(attendus: usize, t: Vec<String>) -> Result<Vec<String>, EchecTraduction> {
    if t.len() == attendus {
        Ok(t)
    } else {
        Err(EchecTraduction::MalAligne {
            attendus,
            rendus: t.len(),
        })
    }
}

/// Posée par une lecture qui laisse des titres sans traduction, sauf si un
/// travail de l'édition attend déjà : deux travaux simultanés demanderaient
/// deux fois les mêmes titres (SC-010). La clé porte le créneau de la lecture.
pub async fn poser(conn: &mut PgConnection, event_id: Uuid, creneau: i64) -> Result<bool> {
    let prefixe = format!("traduction:{event_id}:");
    let en_file = sqlx::query_scalar!(
        r#"SELECT EXISTS (SELECT 1 FROM platform.jobs
                           WHERE task = $1 AND status IN ('queued', 'running')
                             AND starts_with(idempotency_key, $2)) AS "existe!""#,
        TRANSLATE_SESSION_TITLES,
        prefixe
    )
    .fetch_one(&mut *conn)
    .await?;
    if en_file {
        return Ok(false);
    }
    let pose = jobs::enqueue(
        conn,
        NewJob::new(TRANSLATE_SESSION_TITLES, json!({ "event_id": event_id }))
            .idempotent(format!("{prefixe}{creneau}")),
    )
    .await?;
    Ok(pose.is_some())
}
