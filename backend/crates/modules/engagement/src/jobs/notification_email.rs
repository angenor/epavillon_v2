//! Le courriel d'un avis déjà composé par son module (branche générique du
//! consommateur, `WithNotification.email`). Un travail par destinataire : un
//! échec d'envoi se reprend pour lui seul.
//!
//! Le canal se consulte au départ, pas à la mise en file : une personne qui
//! coupe le courriel entre-temps est entendue.

use async_trait::async_trait;
use contracts::negotiation::{Notification, NotificationText};
use kernel::config::Config;
use kernel::db::Db;
use kernel::error::{ApiError, Result};
use kernel::jobs::{self, ClaimedJob, JobHandler, NewJob};
use kernel::mail::{Mailer, OutgoingMail};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgConnection;
use std::sync::Arc;
use uuid::Uuid;

use crate::domain::reminder::NotificationChannel;
use crate::repo::{cross, delivery};

pub const NOTIFICATION_EMAIL: &str = "engagement.notification_email";

#[derive(Debug, Serialize, Deserialize)]
struct Charge {
    person_id: Uuid,
    type_code: String,
    title: NotificationText,
    body: NotificationText,
    link_path: String,
}

/// Un travail par destinataire, clé `(événement, personne)` : le rejeu d'un
/// événement ne double aucun courriel.
pub async fn poser(conn: &mut PgConnection, event_id: Uuid, n: &Notification) -> Result<()> {
    for &person_id in &n.recipients {
        let charge = serde_json::to_value(Charge {
            person_id,
            type_code: n.type_code.clone(),
            title: n.title.clone(),
            body: n.body.clone(),
            link_path: n.link_path.clone(),
        })
        .map_err(|e| ApiError::internal(format!("charge de courriel : {e}")))?;
        jobs::enqueue(
            conn,
            NewJob {
                queue: crate::jobs::send_reminder::QUEUE,
                ..NewJob::new(NOTIFICATION_EMAIL, charge)
            }
            .idempotent(format!("avis:{event_id}:{person_id}")),
        )
        .await?;
    }
    Ok(())
}

pub struct NotificationEmail {
    db: Db,
    mailer: Arc<dyn Mailer>,
    app_public_url: String,
}

impl NotificationEmail {
    pub fn new(db: Db, config: &Config, mailer: Arc<dyn Mailer>) -> Self {
        Self {
            db,
            mailer,
            app_public_url: config.app_public_url.clone(),
        }
    }
}

#[async_trait]
impl JobHandler for NotificationEmail {
    fn task(&self) -> &'static str {
        NOTIFICATION_EMAIL
    }

    fn queue(&self) -> &'static str {
        crate::jobs::send_reminder::QUEUE
    }

    async fn run(&self, job: &ClaimedJob) -> Result<()> {
        let c: Charge = serde_json::from_value(job.payload.clone())
            .map_err(|e| ApiError::internal(format!("charge de courriel illisible : {e}")))?;
        let pool = self.db.pool();

        if !delivery::canal_autorise(
            pool,
            c.person_id,
            &c.type_code,
            NotificationChannel::Email.as_str(),
        )
        .await?
        {
            return Ok(());
        }
        let Some(destinataire) = cross::personne_pour_courriel(pool, c.person_id).await? else {
            return Ok(());
        };

        let mail = composer(&c, &job.id.to_string(), &destinataire, &self.app_public_url);
        self.mailer
            .send(&mail)
            .await
            .map_err(|e| ApiError::internal(e.to_string()))
    }
}

fn composer(
    c: &Charge,
    message_id: &str,
    destinataire: &cross::DestinataireCourriel,
    app_public_url: &str,
) -> OutgoingMail {
    let anglais = destinataire.locale.starts_with("en");
    let (titre, corps) = if anglais {
        (&c.title.en, &c.body.en)
    } else {
        (&c.title.fr, &c.body.fr)
    };
    let lien = format!("{}{}", app_public_url.trim_end_matches('/'), c.link_path);
    let salut = if anglais { "Hello" } else { "Bonjour" };

    OutgoingMail {
        message_id: message_id.to_owned(),
        to: destinataire.email.clone(),
        locale: destinataire.locale.clone(),
        subject: titre.clone(),
        text: format!(
            "{salut} {},\n\n{corps}\n\n{lien}\n",
            destinataire.first_name
        ),
        html: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_langue_choisit_le_texte_et_le_lien_est_absolu() {
        let c = Charge {
            person_id: Uuid::nil(),
            type_code: "programme.comment.addressed".to_owned(),
            title: NotificationText {
                fr: "Titre".to_owned(),
                en: "Title".to_owned(),
            },
            body: NotificationText {
                fr: "Corps".to_owned(),
                en: "Body".to_owned(),
            },
            link_path: "/admin/propositions/x".to_owned(),
        };
        let d = cross::DestinataireCourriel {
            email: "a@example.org".to_owned(),
            locale: "en".to_owned(),
            first_name: "Ada".to_owned(),
        };
        let m = composer(&c, "j", &d, "https://epavillon.org/");
        assert_eq!(m.subject, "Title");
        assert!(m.text.starts_with("Hello Ada,"));
        assert!(m
            .text
            .contains("https://epavillon.org/admin/propositions/x"));
    }
}
