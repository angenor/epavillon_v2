//! Le courriel d'un changement (research R9) : **un travail par destinataire et
//! par tranche de dix minutes**, posé dans la transaction qui émet l'avis.
//!
//! Contrairement aux courriels de 0b, il relit la base en partant : plusieurs
//! changements d'une même tranche donnent un seul courriel, qui dit l'état
//! final (FR-029). L'accord éteint ne coupe que le courriel ; l'avis dans
//! l'application est déjà parti.

use async_trait::async_trait;
use contracts::negotiation::JOB_SESSION_CHANGE_EMAIL;
use kernel::db::Db;
use kernel::error::{ApiError, Result};
use kernel::jobs::{self, ClaimedJob, JobHandler, NewJob};
use kernel::mail::Mailer;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgConnection;
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::jobs::emails::remettre;
use crate::mail::{self, Envoi};
use crate::repo::courriel::{self as depot, TRANCHE_SECONDES};
use crate::repo::notifications as reglage;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Cible {
    Meeting,
    NetworkMeeting,
}

impl Cible {
    pub fn chemin(self, id: Uuid) -> String {
        match self {
            Self::Meeting => format!("/guide-nego/negociations/{id}"),
            Self::NetworkMeeting => format!("/guide-nego/negociations/reseau/{id}"),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct Charge {
    cible: Cible,
    id: Uuid,
    person_id: Uuid,
    tranche: i64,
}

pub fn cle(id: Uuid, tranche: i64, person_id: Uuid) -> String {
    format!("email:{id}:{tranche}:{person_id}")
}

/// Un travail par destinataire ; un second changement de la même tranche
/// retombe sur la même clé et ne pose rien.
pub async fn poser(
    conn: &mut PgConnection,
    cible: Cible,
    id: Uuid,
    destinataires: &[Uuid],
) -> Result<()> {
    if destinataires.is_empty() {
        return Ok(());
    }
    let tranche = depot::tranche(conn).await?;
    let fin = OffsetDateTime::from_unix_timestamp((tranche + 1) * TRANCHE_SECONDES)
        .map_err(|e| ApiError::internal(format!("tranche hors bornes : {e}")))?;
    for &person_id in destinataires {
        let charge = serde_json::to_value(Charge {
            cible,
            id,
            person_id,
            tranche,
        })
        .map_err(|e| ApiError::internal(format!("charge de courriel : {e}")))?;
        jobs::enqueue(
            conn,
            NewJob::new(JOB_SESSION_CHANGE_EMAIL, charge)
                .idempotent(cle(id, tranche, person_id))
                .at(fin),
        )
        .await?;
    }
    Ok(())
}

pub struct SessionChangeEmail {
    db: Db,
    mailer: Arc<dyn Mailer>,
    app_public_url: String,
}

impl SessionChangeEmail {
    pub fn new(db: Db, mailer: Arc<dyn Mailer>, app_public_url: String) -> Self {
        Self {
            db,
            mailer,
            app_public_url,
        }
    }
}

#[async_trait]
impl JobHandler for SessionChangeEmail {
    fn task(&self) -> &'static str {
        JOB_SESSION_CHANGE_EMAIL
    }

    async fn run(&self, job: &ClaimedJob) -> Result<()> {
        let charge: Charge = serde_json::from_value(job.payload.clone())
            .map_err(|e| ApiError::internal(format!("charge de courriel illisible : {e}")))?;
        let identifiant = job.id.to_string();
        let message = {
            let mut conn = self.db.pool().acquire().await?;
            composer(&mut conn, &charge, &identifiant, &self.app_public_url).await?
        };
        match message {
            Some(m) => remettre(self.mailer.as_ref(), &m).await,
            None => Ok(()),
        }
    }
}

async fn composer(
    conn: &mut PgConnection,
    c: &Charge,
    message_id: &str,
    app_public_url: &str,
) -> Result<Option<kernel::mail::OutgoingMail>> {
    if !reglage::accord(conn, c.person_id).await? {
        return Ok(None);
    }
    let Some(personne) = depot::destinataire(conn, c.person_id).await? else {
        return Ok(None);
    };
    let envoi = Envoi {
        message_id,
        to: &personne.email,
        locale: &personne.locale,
        app_public_url,
    };
    let chemin = c.cible.chemin(c.id);
    match c.cible {
        Cible::Meeting => {
            if !depot::suit_la_session(conn, c.id, c.person_id).await? {
                return Ok(None);
            }
            // Une minute de marge : l'import date ses changements à l'horloge
            // du worker, la tranche vient de celle de la base.
            let marge = time::Duration::seconds(60);
            let debut = OffsetDateTime::from_unix_timestamp(c.tranche * TRANCHE_SECONDES)
                .map_err(|e| ApiError::internal(format!("tranche hors bornes : {e}")))?
                - marge;
            let fin = debut + time::Duration::seconds(TRANCHE_SECONDES) + marge * 2;
            let Some(etat) = depot::etat_session(conn, c.id, debut, fin).await? else {
                return Ok(None);
            };
            Ok(mail::etat_de_session(&etat)
                .map(|e| mail::changement_de_session(&envoi, &etat, e, &chemin)))
        }
        Cible::NetworkMeeting => {
            if !depot::suit_la_reunion(conn, c.id, c.person_id).await? {
                return Ok(None);
            }
            Ok(depot::etat_reunion(conn, c.id)
                .await?
                .map(|r| mail::reunion_non_annoncee(&envoi, &r, &chemin)))
        }
    }
}
