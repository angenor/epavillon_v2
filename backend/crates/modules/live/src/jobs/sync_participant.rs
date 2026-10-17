//! `live.sync_meeting_participant` : inscrire une personne chez Zoom et garder
//! son lien personnel.
//!
//! Réunion pas encore créée : la ligne passe en `failed` et le travail réussit,
//! c'est la création de la réunion qui la relancera
//! (`live.requeue_failed_participants`). La réunion est lue `FOR SHARE` : sa
//! création ne peut pas se glisser entre la lecture et cet échec, sans quoi la
//! relance passerait avant et l'inscrit resterait en échec.

use async_trait::async_trait;
use kernel::db::Db;
use kernel::error::{ApiError, Result};
use kernel::jobs::{ClaimedJob, JobHandler};
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::repo::meetings::{self, CANCEL_PARTICIPANT, FILE, SYNC_PARTICIPANT};
use crate::zoom::{Inscrit, Zoom, ZoomError};

pub const PAS_ENCORE_CREEE: &str = "réunion pas encore créée";

pub struct SyncParticipant {
    db: Db,
    zoom: Arc<Zoom>,
}

impl SyncParticipant {
    pub fn new(db: Db, zoom: Arc<Zoom>) -> Self {
        Self { db, zoom }
    }
}

struct AInscrire {
    numero: String,
    email: String,
    first_name: Option<String>,
    last_name: Option<String>,
    display_name: String,
    deja_inscrit: Option<String>,
    essais: i16,
    essais_max: i16,
}

#[async_trait]
impl JobHandler for SyncParticipant {
    fn task(&self) -> &'static str {
        SYNC_PARTICIPANT
    }

    fn queue(&self) -> &'static str {
        FILE
    }

    async fn run(&self, job: &ClaimedJob) -> Result<()> {
        let participant_id: Uuid = serde_json::from_value(job.payload["participant_id"].clone())
            .map_err(ApiError::internal)?;

        let Some(a) = self.reserver(job, participant_id).await? else {
            return Ok(());
        };

        let (prenom, nom) = noms(&a);
        let resultat = self.inscrire(&a, &prenom, &nom).await;

        let mut tx = self.db.write(&job.context()).await?;
        match resultat {
            Ok(inscrit) => {
                // Les identifiants s'écrivent même si l'inscription a été
                // annulée pendant l'appel : c'est ce qui permet d'annuler chez Zoom.
                let annule = sqlx::query_scalar!(
                    r#"UPDATE live.meeting_participants
                          SET external_registrant_id = $2,
                              personal_join_url = $3::text::platform.url,
                              sync_status = 'synced', synced_at = now(),
                              last_error = NULL, next_retry_at = NULL
                        WHERE id = $1
                    RETURNING cancelled_at IS NOT NULL AS "annule!""#,
                    participant_id,
                    inscrit.registrant_id,
                    inscrit.join_url
                )
                .fetch_one(&mut *tx)
                .await?;
                if annule {
                    meetings::mettre_en_file(&mut tx, CANCEL_PARTICIPANT, participant_id, None)
                        .await?;
                }
                tx.commit().await
            }
            Err(e) if e.est_passagere() && a.essais < a.essais_max => {
                // Le travail réussit et en pose un autre à l'échéance : le
                // délai suit `Retry-After` quand Zoom le donne, et les
                // tentatives se comptent sur la ligne, pas sur le travail.
                let delai = delai_avant(&e, a.essais, participant_id);
                let quand = OffsetDateTime::now_utc() + time::Duration::seconds(delai);
                sqlx::query!(
                    "UPDATE live.meeting_participants
                        SET sync_status = 'failed', last_error = $2, next_retry_at = $3
                      WHERE id = $1",
                    participant_id,
                    e.to_string(),
                    quand
                )
                .execute(&mut *tx)
                .await?;
                meetings::mettre_en_file_a(
                    &mut tx,
                    SYNC_PARTICIPANT,
                    participant_id,
                    None,
                    Some(quand),
                )
                .await?;
                tx.commit().await
            }
            Err(e) => {
                sqlx::query!(
                    "UPDATE live.meeting_participants
                        SET sync_status = 'abandoned', last_error = $2, next_retry_at = NULL
                      WHERE id = $1",
                    participant_id,
                    e.to_string()
                )
                .execute(&mut *tx)
                .await?;
                tx.commit().await?;
                tracing::warn!(participant = %participant_id, erreur = %e, "inscrit abandonné");
                Ok(())
            }
        }
    }
}

/// `Retry-After`, sinon trente secondes doublées à chaque essai (une heure au
/// plus), plus jusqu'à une minute propre à chaque inscrit : trois cents
/// inscrits refusés ensemble ne reviennent pas ensemble.
fn delai_avant(e: &ZoomError, essais: i16, participant_id: Uuid) -> i64 {
    let base = match e.reessayer_dans() {
        Some(s) => s as i64,
        None => (30i64 << (essais.clamp(1, 8) - 1)).min(3600),
    };
    base + i64::from(participant_id.as_bytes()[15] % 60)
}

impl SyncParticipant {
    async fn reserver(&self, job: &ClaimedJob, participant_id: Uuid) -> Result<Option<AInscrire>> {
        let mut tx = self.db.write(&job.context()).await?;
        let reunion = sqlx::query!(
            r#"SELECT m.external_id, m.status::text AS "status!"
                 FROM live.meeting_participants p
                 JOIN live.meetings m ON m.id = p.meeting_id
                WHERE p.id = $1
                  FOR SHARE OF m"#,
            participant_id
        )
        .fetch_optional(&mut *tx)
        .await?;

        let Some(reunion) = reunion else {
            return Ok(None);
        };
        let numero = match reunion.external_id {
            Some(n) if matches!(reunion.status.as_str(), "scheduled" | "started" | "ended") => n,
            _ => {
                sqlx::query!(
                    "UPDATE live.meeting_participants
                        SET sync_status = 'failed', last_error = $2, last_attempt_at = now()
                      WHERE id = $1 AND sync_status IN ('pending', 'failed')
                        AND cancelled_at IS NULL",
                    participant_id,
                    PAS_ENCORE_CREEE
                )
                .execute(&mut *tx)
                .await?;
                tx.commit().await?;
                return Ok(None);
            }
        };

        let ligne = sqlx::query!(
            r#"UPDATE live.meeting_participants
                  SET sync_status = 'in_progress', sync_attempts = sync_attempts + 1,
                      last_attempt_at = now()
                WHERE id = $1 AND cancelled_at IS NULL
                  AND (sync_status IN ('pending', 'failed')
                       OR (sync_status = 'in_progress'
                           AND last_attempt_at < now() - interval '10 minutes'))
            RETURNING email::text AS "email!", display_name,
                      request_payload->>'first_name' AS first_name,
                      request_payload->>'last_name' AS last_name,
                      external_registrant_id, sync_attempts, max_sync_attempts"#,
            participant_id
        )
        .fetch_optional(&mut *tx)
        .await?;
        tx.commit().await?;

        Ok(ligne.map(|l| AInscrire {
            numero,
            email: l.email,
            first_name: l.first_name,
            last_name: l.last_name,
            display_name: l.display_name,
            deja_inscrit: l.external_registrant_id,
            essais: l.sync_attempts,
            essais_max: l.max_sync_attempts,
        }))
    }

    async fn inscrire(
        &self,
        a: &AInscrire,
        prenom: &str,
        nom: &str,
    ) -> std::result::Result<Inscrit, ZoomError> {
        let client = &self.zoom.client;
        let inscrit = client.inscrire(&a.numero, &a.email, prenom, nom).await?;
        // Réinscription après annulation : Zoom range l'inscrit parmi les
        // refusés ; retrouvé en attente ou refusé, il est réapprouvé.
        if a.deja_inscrit.is_some() || matches!(inscrit.statut, Some("pending" | "denied")) {
            if let Err(e) = client
                .changer_statut_inscrit(&a.numero, "approve", &inscrit.registrant_id, &a.email)
                .await
            {
                if e.est_passagere() {
                    return Err(e);
                }
                tracing::warn!(erreur = %e, "réapprobation de l'inscrit refusée par Zoom");
            }
        }
        Ok(inscrit)
    }
}

/// Zoom exige un prénom ; le nom peut manquer.
fn noms(a: &AInscrire) -> (String, String) {
    let prenom = a
        .first_name
        .clone()
        .filter(|p| !p.trim().is_empty())
        .unwrap_or_else(|| a.display_name.clone());
    let nom = a.last_name.clone().unwrap_or_default();
    (
        prenom.chars().take(64).collect(),
        nom.chars().take(64).collect(),
    )
}
