//! Contrat d'envoi de courriel — pas un client.
//!
//! Contrainte d'hébergement du 20/08 : l'API et le site vivent sur deux
//! serveurs, et seul celui du site a le droit d'émettre. L'API compose le
//! message et le remet au site, qui ouvre la connexion SMTP.
//!
//! Le jour où l'hébergeur de l'API autorise l'émission, l'envoi se réécrit ici
//! et **aucun module ne bouge** : c'est tout l'intérêt d'exposer un contrat.
//! Sans cette séparation, la bascule obligerait à rouvrir chaque module qui
//! envoie un courriel — c'est-à-dire, à terme, presque tous.

use async_trait::async_trait;
use serde::Serialize;
use std::sync::Arc;

use crate::config::{MailConfig, MailTransport};
use crate::crypto;

/// Code stable écrit en tête de `platform.jobs.last_error`. Il n'est
/// délibérément PAS dans l'énuméré des codes d'API : rien ne doit pouvoir le
/// rendre dans une réponse HTTP.
pub const MAIL_RELAY_UNREACHABLE: &str = "MAIL_RELAY_UNREACHABLE";

#[derive(Debug, thiserror::Error)]
pub enum MailError {
    /// Écrite dans `platform.jobs.last_error`, jamais rendue à un client.
    #[error("{MAIL_RELAY_UNREACHABLE} : relais de courriel injoignable : {0}")]
    Unreachable(String),
    #[error("{MAIL_RELAY_UNREACHABLE} : relais de courriel refusé ({statut}) : {corps}")]
    Rejected { statut: u16, corps: String },
    #[error("transport de courriel non branché : {0}")]
    NotWired(&'static str),
}

/// Le message arrive **composé** : le texte appartient au module qui déclenche
/// l'envoi, dans la langue de `people.preferred_locale`. Le site reçoit un
/// texte, pas un gabarit. En B6 la composition passe aux modèles administrables
/// de `engagement.message_templates` ; si le site composait, il faudrait alors
/// défaire son travail.
#[derive(Debug, Clone, Serialize)]
pub struct OutgoingMail {
    /// Identifiant du travail : le site retient les identifiants déjà envoyés
    /// quelques minutes, ce qui absorbe une reprise après délai d'attente.
    pub message_id: String,
    pub to: String,
    pub locale: String,
    pub subject: String,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub html: Option<String>,
}

#[async_trait]
pub trait Mailer: Send + Sync {
    async fn send(&self, mail: &OutgoingMail) -> Result<(), MailError>;
}

pub fn build(cfg: &MailConfig) -> Arc<dyn Mailer> {
    match cfg.transport {
        MailTransport::Relay => Arc::new(RelayMailer::new(
            cfg.relay_url.clone(),
            cfg.relay_token.expose().to_owned(),
        )),
        MailTransport::Smtp => Arc::new(SmtpMailer),
    }
}

pub struct RelayMailer {
    client: reqwest::Client,
    url: String,
    token: String,
}

impl RelayMailer {
    pub fn new(url: String, token: String) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .expect("client HTTP"),
            url,
            token,
        }
    }
}

#[async_trait]
impl Mailer for RelayMailer {
    async fn send(&self, mail: &OutgoingMail) -> Result<(), MailError> {
        let reponse = self
            .client
            .post(&self.url)
            .header("X-Mail-Relay-Token", &self.token)
            .json(mail)
            .send()
            .await
            .map_err(|e| MailError::Unreachable(e.to_string()))?;

        let statut = reponse.status();
        if statut.is_success() {
            return Ok(());
        }

        let corps = reponse.text().await.unwrap_or_default();
        Err(MailError::Rejected {
            statut: statut.as_u16(),
            corps,
        })
    }
}

/// Envoi direct depuis le worker : **laissé non branché**. Le serveur de l'API
/// n'a pas le droit d'émettre ; la variante existe pour que le jour de
/// l'autorisation ne coûte qu'une implémentation et une clé de configuration.
pub struct SmtpMailer;

#[async_trait]
impl Mailer for SmtpMailer {
    async fn send(&self, _mail: &OutgoingMail) -> Result<(), MailError> {
        Err(MailError::NotWired(
            "MAIL_TRANSPORT=smtp : l'hébergeur de l'API n'autorise pas encore l'émission de courriel.",
        ))
    }
}

/// Comparaison à temps constant du secret partagé, côté réception.
/// Une requête sans secret valide reçoit 404, jamais 401 : une route privée ne
/// confirme pas son existence.
pub fn relay_token_matches(recu: &str, attendu: &str) -> bool {
    crypto::constant_time_eq(recu, attendu)
}
