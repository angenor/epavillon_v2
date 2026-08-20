//! Travaux d'envoi de courriel.
//!
//! **Ces travaux portent un secret** : la charge utile transporte le jeton en
//! clair, que la base ne connaît que haché. Deux conséquences, et elles sont
//! toutes les deux dans le code :
//!
//! - la réussite **vide la charge utile** (`kernel::jobs::succeed`) : le travail
//!   garde sa trace — un courriel est parti, quand, après combien d'essais —
//!   sans garder son contenu ;
//!   et l'échec définitif la vide aussi, par `carries_secret()`, sinon un
//!   travail mort conserverait son jeton indéfiniment ;
//! - le message d'erreur écrit dans `platform.jobs.last_error` **ne recopie ni
//!   l'adresse du destinataire ni le corps de la réponse du relais**. Un journal
//!   d'exploitation se lit par beaucoup de monde.
//!
//! Le gestionnaire ne touche pas la base : tout ce qu'il lui faut est dans la
//! charge utile. C'est ce qui lui permet de rester une fonction pure de bout en
//! bout, et à l'envoi de ne jamais dépendre d'une lecture qui pourrait avoir
//! changé depuis la mise en file.

use async_trait::async_trait;
use kernel::error::{ApiError, Result};
use kernel::jobs::{ClaimedJob, JobHandler};
use kernel::mail::{Mailer, OutgoingMail};
use serde::Deserialize;
use std::sync::Arc;

use crate::mail::{self, MailContext};

pub const SEND_VERIFICATION_EMAIL: &str = "identity.send_verification_email";
pub const SEND_EXISTING_ACCOUNT_NOTICE: &str = "identity.send_existing_account_notice";
pub const SEND_PASSWORD_RESET_EMAIL: &str = "identity.send_password_reset_email";

/// Ce que la mise en file dépose. `token` n'est présent que pour les envois qui
/// portent un lien : un rappel de compte existant n'en a pas, et ne doit pas en
/// avoir.
#[derive(Debug, Deserialize)]
struct Charge {
    to: String,
    locale: String,
    first_name: String,
    #[serde(default)]
    token: Option<String>,
}

pub struct SendVerificationEmail {
    mailer: Arc<dyn Mailer>,
    app_public_url: String,
}

pub struct SendExistingAccountNotice {
    mailer: Arc<dyn Mailer>,
    app_public_url: String,
}

pub struct SendPasswordResetEmail {
    mailer: Arc<dyn Mailer>,
    app_public_url: String,
}

impl SendVerificationEmail {
    pub fn new(mailer: Arc<dyn Mailer>, app_public_url: String) -> Self {
        Self {
            mailer,
            app_public_url,
        }
    }
}

impl SendExistingAccountNotice {
    pub fn new(mailer: Arc<dyn Mailer>, app_public_url: String) -> Self {
        Self {
            mailer,
            app_public_url,
        }
    }
}

impl SendPasswordResetEmail {
    pub fn new(mailer: Arc<dyn Mailer>, app_public_url: String) -> Self {
        Self {
            mailer,
            app_public_url,
        }
    }
}

#[async_trait]
impl JobHandler for SendVerificationEmail {
    fn task(&self) -> &'static str {
        SEND_VERIFICATION_EMAIL
    }

    fn carries_secret(&self) -> bool {
        true
    }

    async fn run(&self, job: &ClaimedJob) -> Result<()> {
        let charge = lire(job)?;
        let jeton = charge
            .token
            .as_deref()
            .ok_or_else(|| ApiError::internal("travail d'envoi sans jeton"))?;

        let identifiant = job.id.to_string();
        let message = mail::verification_email(
            &contexte(&identifiant, &charge, &self.app_public_url),
            jeton,
        );
        remettre(self.mailer.as_ref(), &message).await
    }
}

#[async_trait]
impl JobHandler for SendExistingAccountNotice {
    fn task(&self) -> &'static str {
        SEND_EXISTING_ACCOUNT_NOTICE
    }

    async fn run(&self, job: &ClaimedJob) -> Result<()> {
        let charge = lire(job)?;
        let identifiant = job.id.to_string();
        let message =
            mail::existing_account_notice(&contexte(&identifiant, &charge, &self.app_public_url));
        remettre(self.mailer.as_ref(), &message).await
    }
}

#[async_trait]
impl JobHandler for SendPasswordResetEmail {
    fn task(&self) -> &'static str {
        SEND_PASSWORD_RESET_EMAIL
    }

    /// Le lien de réinitialisation ouvre un compte à qui le tient : un travail
    /// mort qui le garderait en clair vaudrait un mot de passe en clair.
    fn carries_secret(&self) -> bool {
        true
    }

    async fn run(&self, job: &ClaimedJob) -> Result<()> {
        let charge = lire(job)?;
        let jeton = charge
            .token
            .as_deref()
            .ok_or_else(|| ApiError::internal("travail d'envoi sans jeton"))?;

        let identifiant = job.id.to_string();
        let message = mail::password_reset_email(
            &contexte(&identifiant, &charge, &self.app_public_url),
            jeton,
        );
        remettre(self.mailer.as_ref(), &message).await
    }
}

fn lire(job: &ClaimedJob) -> Result<Charge> {
    serde_json::from_value(job.payload.clone())
        .map_err(|e| ApiError::internal(format!("charge utile illisible : {e}")))
}

/// L'identifiant du travail sert d'identifiant de message : le site retient
/// quelques minutes ceux qu'il a déjà envoyés, ce qui absorbe une reprise
/// d'essai après délai d'attente dépassé — le courriel est parti, la réponse
/// s'est perdue.
fn contexte<'a>(
    message_id: &'a str,
    charge: &'a Charge,
    app_public_url: &'a str,
) -> MailContext<'a> {
    MailContext {
        message_id,
        to: &charge.to,
        locale: &charge.locale,
        first_name: &charge.first_name,
        app_public_url,
    }
}

/// L'erreur rendue est **relue par l'exploitation** : elle porte le code stable
/// et le statut, jamais l'adresse ni le corps renvoyé par le relais.
async fn remettre(mailer: &dyn Mailer, message: &OutgoingMail) -> Result<()> {
    use kernel::mail::MailError;

    mailer.send(message).await.map_err(|e| match e {
        MailError::Unreachable(_) => ApiError::internal(format!(
            "{} : relais injoignable",
            kernel::mail::MAIL_RELAY_UNREACHABLE
        )),
        MailError::Rejected { statut, .. } => ApiError::internal(format!(
            "{} : relais refusé ({statut})",
            kernel::mail::MAIL_RELAY_UNREACHABLE
        )),
        MailError::NotWired(raison) => ApiError::internal(raison),
    })
}
