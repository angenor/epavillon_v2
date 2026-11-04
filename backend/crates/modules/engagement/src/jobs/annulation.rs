//! Le courriel à l'équipe quand une organisation annule son activité retenue
//! (04/11/2026).
//!
//! Mis en file par le consommateur des avis sur
//! `programme.proposal.cancelled_by_organization`, que seule la route de
//! l'organisation émet : une annulation par l'équipe n'écrit pas à l'équipe.
//! Destinataires : le réglage `engagement.mail.automatic_cc`. Ce travail est
//! dans `SANS_COPIE` : ses destinataires sont déjà cette liste.

use async_trait::async_trait;
use contracts::engagement::EMAIL_QUEUE;
use contracts::programme::{ProposalCancelledByOrganization, OWNER_CANCELLATION_TASK};
use kernel::config::Config;
use kernel::db::Db;
use kernel::error::{ApiError, Result};
use kernel::events::OutboxEvent;
use kernel::jobs::{self, ClaimedJob, JobHandler, NewJob};
use kernel::mail::{Extras, Mailer};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgConnection;
use std::sync::Arc;
use uuid::Uuid;

use crate::domain::annulation;
use crate::domain::courriel_direct;
use crate::domain::gabarits::images_integrees;
use crate::jobs::send_mailing::apparence;
use crate::repo::cross;

#[derive(Debug, Serialize, Deserialize)]
struct Charge {
    proposal_id: Uuid,
    transition_id: Uuid,
}

/// Une annulation, un courriel : la clé est la ligne de journal.
pub async fn poser(conn: &mut PgConnection, event: &OutboxEvent) -> Result<()> {
    let annonce: ProposalCancelledByOrganization =
        match serde_json::from_value(event.payload.clone()) {
            Ok(a) => a,
            Err(e) => {
                tracing::warn!(erreur = %e, "annulation par l'organisation illisible");
                return Ok(());
            }
        };
    let charge = serde_json::to_value(Charge {
        proposal_id: annonce.proposal_id,
        transition_id: annonce.transition_id,
    })
    .map_err(ApiError::internal)?;
    let job = NewJob {
        queue: EMAIL_QUEUE,
        ..NewJob::new(OWNER_CANCELLATION_TASK, charge)
    }
    .idempotent(format!("annulation-organisation:{}", annonce.transition_id));
    jobs::enqueue(conn, job).await?;
    Ok(())
}

pub struct SendOwnerCancellation {
    db: Db,
    mailer: Arc<dyn Mailer>,
    app_public_url: String,
}

impl SendOwnerCancellation {
    pub fn new(db: Db, config: &Config, mailer: Arc<dyn Mailer>) -> Self {
        Self {
            db,
            mailer,
            app_public_url: config.app_public_url.clone(),
        }
    }
}

#[async_trait]
impl JobHandler for SendOwnerCancellation {
    fn task(&self) -> &'static str {
        OWNER_CANCELLATION_TASK
    }

    fn queue(&self) -> &'static str {
        EMAIL_QUEUE
    }

    async fn run(&self, job: &ClaimedJob) -> Result<()> {
        let charge: Charge = serde_json::from_value(job.payload.clone())
            .map_err(|_| ApiError::internal("charge sans proposal_id ni transition_id"))?;
        let pool = self.db.pool();
        let Some(a) =
            cross::annulation_par_lorganisation(pool, charge.proposal_id, charge.transition_id)
                .await?
        else {
            return Ok(());
        };
        let equipe = crate::mail::adresses_de_lequipe(pool)
            .await
            .map_err(ApiError::internal)?;
        let habit = apparence(
            &self.db,
            a.event_id,
            courriel_direct::GABARIT,
            None,
            &self.app_public_url,
        )
        .await?;
        let Some((mail, cc)) = annulation::courriel(
            &a,
            &equipe,
            &self.app_public_url,
            &job.id.to_string(),
            habit.visuels.as_ref(),
        ) else {
            tracing::warn!(
                reference = %a.reference_code,
                "annulation par l'organisation : aucune adresse d'équipe, aucun courriel"
            );
            return Ok(());
        };
        let extras = Extras {
            cc,
            inline_images: images_integrees(&habit.gabarit, habit.visuels.as_ref()),
            ..Extras::default()
        };
        self.mailer
            .send_with_extras(&mail, &extras)
            .await
            .map_err(|_| {
                ApiError::internal("envoi de l'avis d'annulation à l'équipe refusé par le relais")
            })
    }
}
