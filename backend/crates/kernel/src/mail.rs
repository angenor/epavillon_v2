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
use lettre::message::{Mailbox, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::AsyncSmtpTransport;
use lettre::{AsyncTransport, Message, Tokio1Executor};
use serde::Serialize;
use std::sync::Arc;
use std::time::Duration;

use crate::config::{ConfigError, MailConfig, MailTransport, SmtpConfig, SmtpEncryption};
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

/// Rend une erreur de CONFIGURATION, jamais un expéditeur qui échouerait au
/// premier message : un serveur de courriel mal décrit doit empêcher le
/// démarrage, pas se découvrir une heure plus tard dans un travail différé.
pub fn build(cfg: &MailConfig) -> Result<Arc<dyn Mailer>, ConfigError> {
    match cfg.transport {
        MailTransport::Relay => Ok(Arc::new(RelayMailer::new(
            cfg.relay_url.clone(),
            cfg.relay_token.expose().to_owned(),
        ))),
        MailTransport::Smtp => {
            // Écarté par la validation de la configuration ; exprimé quand même,
            // parce qu'une garde qui vit dans un autre fichier se perd.
            let smtp = cfg.smtp.as_ref().ok_or_else(|| {
                ConfigError::Invalid(
                    "MAIL_TRANSPORT vaut smtp mais aucun serveur n'est décrit (SMTP_HOST).".into(),
                )
            })?;
            Ok(Arc::new(SmtpMailer::new(smtp)?))
        }
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

/// Envoi direct au serveur de courriel du domaine, en client authentifié.
///
/// **Le jour annoncé en B1 est arrivé, et il n'a rien coûté aux modules** :
/// c'est ici, et seulement ici, que l'implémentation change. Le constat du
/// 01/09 est qu'émettre au nom d'un domaine n'a jamais demandé de l'héberger —
/// il fallait un compte et le port 587 en sortie, tous deux disponibles. Le
/// détour par le site n'avait donc plus de raison d'être.
pub struct SmtpMailer {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from: Mailbox,
}

/// Aligné sur le délai que `RelayMailer` s'imposait. Sans lui, `lettre` attend
/// indéfiniment : un serveur qui accepte la connexion puis se tait retiendrait
/// un ouvrier du worker jusqu'au redémarrage.
const DELAI: Duration = Duration::from_secs(15);

impl SmtpMailer {
    pub fn new(cfg: &SmtpConfig) -> Result<Self, ConfigError> {
        let batisseur = match cfg.encryption {
            SmtpEncryption::StartTls => {
                AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&cfg.host)
            }
            SmtpEncryption::Implicit => AsyncSmtpTransport::<Tokio1Executor>::relay(&cfg.host),
            // `builder_dangerous` porte bien son nom : rien n'est chiffré. La
            // configuration a déjà refusé ce mode dès qu'un identifiant
            // l'accompagne, ce qui le réserve à Mailpit.
            SmtpEncryption::None => Ok(AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(
                &cfg.host,
            )),
        }
        .map_err(|e| {
            ConfigError::Invalid(format!(
                "connexion au serveur de courriel {} impossible à préparer : {e}",
                cfg.host
            ))
        })?;

        let mut batisseur = batisseur.port(cfg.port).timeout(Some(DELAI));
        if !cfg.username.is_empty() {
            batisseur = batisseur.credentials(Credentials::new(
                cfg.username.clone(),
                cfg.password.expose().to_owned(),
            ));
        }

        Ok(Self {
            transport: batisseur.build(),
            from: cfg.from.clone(),
        })
    }
}

/// Le code de réponse SMTP, ramené au champ `statut` que `MailError::Rejected`
/// portait pour HTTP. Les deux numérotations ne se confondent pas — 550 n'est
/// pas un code HTTP — mais le champ dit « ce que le serveur a répondu », et
/// c'est vrai des deux. Ajouter une variante à `MailError` obligerait les six
/// courriels de B1 et B2 à rouvrir leur `match` : exactement ce que le contrat
/// existe pour éviter.
fn code_reponse(erreur: &lettre::transport::smtp::Error) -> u16 {
    erreur
        .status()
        .and_then(|c| c.to_string().parse().ok())
        .unwrap_or(0)
}

#[async_trait]
impl Mailer for SmtpMailer {
    async fn send(&self, mail: &OutgoingMail) -> Result<(), MailError> {
        let destinataire: Mailbox = mail.to.parse().map_err(|e| MailError::Rejected {
            // 550, « boîte introuvable » : le refus est définitif, et le
            // rejouer ne changerait rien.
            statut: 550,
            corps: format!("adresse « {} » inexploitable : {e}", mail.to),
        })?;

        let entete = Message::builder()
            .from(self.from.clone())
            .to(destinataire)
            .subject(mail.subject.as_str());

        // Deux corps quand le module en compose deux : un client qui ne rend
        // pas le HTML affiche alors le texte, au lieu d'une pièce jointe.
        let message = match &mail.html {
            Some(html) => entete.multipart(MultiPart::alternative_plain_html(
                mail.text.clone(),
                html.clone(),
            )),
            None => entete.singlepart(SinglePart::plain(mail.text.clone())),
        }
        .map_err(|e| MailError::Rejected {
            statut: 0,
            corps: format!("message impossible à composer : {e}"),
        })?;

        match self.transport.send(message).await {
            Ok(_) => Ok(()),
            // **Le partage compte plus que le message** : `Unreachable` est
            // repris par le travail différé, `Rejected` ne l'est pas. Un refus
            // permanent — boîte inexistante, expéditeur non autorisé — rejoué
            // toutes les heures abîme la réputation du domaine expéditeur.
            Err(e) if e.is_permanent() => Err(MailError::Rejected {
                statut: code_reponse(&e),
                corps: e.to_string(),
            }),
            Err(e) => Err(MailError::Unreachable(e.to_string())),
        }
    }
}

/// Comparaison à temps constant du secret partagé, côté réception.
/// Une requête sans secret valide reçoit 404, jamais 401 : une route privée ne
/// confirme pas son existence.
pub fn relay_token_matches(recu: &str, attendu: &str) -> bool {
    crypto::constant_time_eq(recu, attendu)
}
