//! Configuration typée, lue de l'environnement au démarrage et validée là.
//!
//! Une durée mal écrite fait échouer le démarrage, jamais une requête. Les
//! réglages d'exploitation vivent ici et non dans `platform.settings` : un
//! seuil relu en base à chaque connexion serait un point de panne, et une
//! donnée modifiable sans trace de déploiement (écarts n° 18 et 19).

use figment::providers::Env;
use figment::Figment;
use serde::Deserialize;
use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    // Mise en boîte : l'erreur de figment pèse 200 octets, et une
    // configuration refusée n'est renvoyée qu'une fois, au démarrage.
    #[error("configuration illisible : {0}")]
    Read(#[from] Box<figment::Error>),
    #[error("{0}")]
    Invalid(String),
}

fn invalid(message: impl Into<String>) -> ConfigError {
    ConfigError::Invalid(message.into())
}

/// Reflet plat de l'environnement. Les noms de champs sont ceux des variables,
/// en minuscules — `AUTH_LOCKOUT_DURATION` devient `auth_lockout_duration`.
#[derive(Debug, Deserialize)]
struct Raw {
    database_url: String,
    #[serde(default = "default_bind_addr")]
    api_bind_addr: String,
    #[serde(default = "vrai")]
    api_docs_enabled: bool,
    #[serde(default = "default_public_url")]
    app_public_url: String,
    #[serde(default)]
    worker_id: String,

    #[serde(default = "default_lockout_threshold")]
    auth_lockout_threshold: u16,
    #[serde(with = "humantime_serde", default = "minutes_15")]
    auth_lockout_duration: Duration,
    #[serde(with = "humantime_serde", default = "minutes_15")]
    auth_access_token_ttl: Duration,
    #[serde(with = "humantime_serde", default = "hours_12")]
    auth_session_ttl: Duration,
    #[serde(with = "humantime_serde", default = "days_30")]
    auth_session_ttl_remembered: Duration,

    #[serde(with = "humantime_serde", default = "hours_24")]
    auth_token_ttl_email_verification: Duration,
    #[serde(with = "humantime_serde", default = "hours_1")]
    auth_token_ttl_password_reset: Duration,
    #[serde(with = "humantime_serde", default = "days_7")]
    auth_token_ttl_invitation: Duration,
    #[serde(with = "humantime_serde", default = "minutes_15")]
    auth_token_ttl_magic_link: Duration,
    #[serde(with = "humantime_serde", default = "days_14")]
    auth_token_ttl_speaker_confirmation: Duration,

    #[serde(default)]
    auth_signing_key: String,
    // Le défaut FERME au lieu d'ouvrir : une clé oubliée en production poserait
    // sinon le cookie de session sans l'attribut Secure, sans que rien ne le dise.
    #[serde(default = "default_cookie_secure")]
    auth_cookie_secure: bool,
    #[serde(default)]
    auth_cookie_domain: String,

    /// Adresses ou préfixes des mandataires dont on accepte `X-Forwarded-For`.
    /// Vide : personne — le défaut ferme au lieu d'ouvrir.
    #[serde(default)]
    trusted_proxies: String,

    #[serde(default = "default_mail_transport")]
    mail_transport: String,
    #[serde(default)]
    mail_relay_url: String,
    #[serde(default)]
    mail_relay_token: String,

    #[serde(default)]
    otel_exporter_otlp_endpoint: String,
    #[serde(default = "default_service_name")]
    otel_service_name: String,
    #[serde(default = "default_rust_log")]
    rust_log: String,
}

fn default_bind_addr() -> String {
    "127.0.0.1:8080".to_owned()
}
fn default_public_url() -> String {
    "http://localhost:3000".to_owned()
}
fn default_mail_transport() -> String {
    "relay".to_owned()
}
fn default_service_name() -> String {
    "epavillon-api".to_owned()
}
fn default_rust_log() -> String {
    "info".to_owned()
}
fn default_lockout_threshold() -> u16 {
    5
}
fn default_cookie_secure() -> bool {
    true
}
fn vrai() -> bool {
    true
}
fn minutes_15() -> Duration {
    Duration::from_secs(15 * 60)
}
fn hours_1() -> Duration {
    Duration::from_secs(3600)
}
fn hours_12() -> Duration {
    Duration::from_secs(12 * 3600)
}
fn hours_24() -> Duration {
    Duration::from_secs(24 * 3600)
}
fn days_7() -> Duration {
    Duration::from_secs(7 * 86_400)
}
fn days_14() -> Duration {
    Duration::from_secs(14 * 86_400)
}
fn days_30() -> Duration {
    Duration::from_secs(30 * 86_400)
}

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: Secret,
    pub api_bind_addr: String,
    /// La documentation générée est-elle servie ? Ouverte par défaut — le
    /// contrat la veut accessible partout **sauf en production**, où elle
    /// décrirait à qui sonde le port la totalité de la surface d'appel.
    pub api_docs_enabled: bool,
    /// Adresse du SITE, pas de l'API : un lien de courriel mène à un écran.
    pub app_public_url: String,
    pub worker_id: String,
    /// Ceux dont on croit l'en-tête d'adresse d'origine. Vide par défaut : sans
    /// eux, n'importe quel client choisirait l'adresse qu'on enregistre.
    pub trusted_proxies: crate::net::TrustedProxies,
    pub auth: AuthConfig,
    pub mail: MailConfig,
    pub telemetry: TelemetryConfig,
}

#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub lockout_threshold: u16,
    pub lockout_duration: Duration,
    pub access_token_ttl: Duration,
    pub session_ttl: Duration,
    pub session_ttl_remembered: Duration,
    pub token_ttl: TokenTtls,
    pub signing_key: Secret,
    pub cookie_secure: bool,
    pub cookie_domain: Option<String>,
}

/// Une durée par valeur de `identity.token_purpose`. Aucun appelant ne pose
/// d'expiration lui-même : il donne la finalité, la configuration donne la
/// durée (FR-018).
#[derive(Debug, Clone)]
pub struct TokenTtls {
    pub email_verification: Duration,
    pub password_reset: Duration,
    pub invitation: Duration,
    pub magic_link: Duration,
    pub speaker_confirmation: Duration,
}

impl TokenTtls {
    /// La finalité est donnée par son nom en base : le noyau ne porte pas la
    /// machine à états du module, seulement ses réglages.
    pub fn for_purpose(&self, purpose: &str) -> Option<Duration> {
        match purpose {
            "email_verification" => Some(self.email_verification),
            "password_reset" => Some(self.password_reset),
            "invitation" => Some(self.invitation),
            "magic_link" => Some(self.magic_link),
            "speaker_confirmation" => Some(self.speaker_confirmation),
            _ => None,
        }
    }
}

/// Enveloppe un secret pour qu'il ne parte pas dans une trace : `Debug` ne rend
/// que sa longueur. Les TROIS secrets de la configuration passent par là —
/// l'URL de base porte un mot de passe au même titre que les deux autres.
#[derive(Clone)]
pub struct Secret(String);

impl Secret {
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl From<String> for Secret {
    fn from(valeur: String) -> Self {
        Self(valeur)
    }
}

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Secret({} octets)", self.0.len())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MailTransport {
    /// Remise HTTP au serveur du site : le seul chemin autorisé aujourd'hui.
    Relay,
    /// SMTP direct depuis le worker — laissé non branché jusqu'au jour où
    /// l'hébergeur de l'API autorise l'émission (research.md § R13).
    Smtp,
}

#[derive(Debug, Clone)]
pub struct MailConfig {
    pub transport: MailTransport,
    pub relay_url: String,
    pub relay_token: Secret,
}

#[derive(Debug, Clone)]
pub struct TelemetryConfig {
    pub otlp_endpoint: Option<String>,
    pub service_name: String,
    pub log_filter: String,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_figment(Figment::from(Env::raw()))
    }

    pub fn from_figment(figment: Figment) -> Result<Self, ConfigError> {
        let raw: Raw = figment.extract().map_err(Box::new)?;

        if raw.database_url.trim().is_empty() {
            return Err(invalid("DATABASE_URL est vide."));
        }
        if raw.auth_signing_key.trim().is_empty() {
            return Err(invalid(
                "AUTH_SIGNING_KEY est vide : le jeton d'accès ne peut pas être signé.",
            ));
        }
        if raw.auth_lockout_threshold == 0 {
            return Err(invalid(
                "AUTH_LOCKOUT_THRESHOLD vaut 0 : aucun compte ne pourrait se connecter.",
            ));
        }
        if !raw.app_public_url.starts_with("http://") && !raw.app_public_url.starts_with("https://")
        {
            return Err(invalid(
                "APP_PUBLIC_URL doit être une adresse absolue (http:// ou https://).",
            ));
        }

        let transport = match raw.mail_transport.as_str() {
            "relay" => MailTransport::Relay,
            "smtp" => MailTransport::Smtp,
            other => {
                return Err(invalid(format!(
                    "MAIL_TRANSPORT vaut « {other} » : valeurs acceptées, relay ou smtp."
                )))
            }
        };
        if transport == MailTransport::Relay {
            if raw.mail_relay_url.trim().is_empty() {
                return Err(invalid("MAIL_RELAY_URL est vide alors que MAIL_TRANSPORT vaut relay : aucun courriel ne partirait."));
            }
            if !raw.mail_relay_url.starts_with("http://")
                && !raw.mail_relay_url.starts_with("https://")
            {
                return Err(invalid(
                    "MAIL_RELAY_URL doit être une adresse absolue (http:// ou https://).",
                ));
            }
            if raw.mail_relay_token.trim().is_empty() {
                return Err(invalid(
                    "MAIL_RELAY_TOKEN est vide : le site refuserait la remise.",
                ));
            }
        }

        for (cle, duree) in [
            ("AUTH_LOCKOUT_DURATION", raw.auth_lockout_duration),
            ("AUTH_ACCESS_TOKEN_TTL", raw.auth_access_token_ttl),
            ("AUTH_SESSION_TTL", raw.auth_session_ttl),
            (
                "AUTH_SESSION_TTL_REMEMBERED",
                raw.auth_session_ttl_remembered,
            ),
            (
                "AUTH_TOKEN_TTL_EMAIL_VERIFICATION",
                raw.auth_token_ttl_email_verification,
            ),
            (
                "AUTH_TOKEN_TTL_PASSWORD_RESET",
                raw.auth_token_ttl_password_reset,
            ),
            ("AUTH_TOKEN_TTL_INVITATION", raw.auth_token_ttl_invitation),
            ("AUTH_TOKEN_TTL_MAGIC_LINK", raw.auth_token_ttl_magic_link),
            (
                "AUTH_TOKEN_TTL_SPEAKER_CONFIRMATION",
                raw.auth_token_ttl_speaker_confirmation,
            ),
        ] {
            if duree.is_zero() {
                return Err(invalid(format!("{cle} vaut une durée nulle.")));
            }
        }

        let trusted_proxies = crate::net::TrustedProxies::parse(&raw.trusted_proxies)
            .map_err(|e| invalid(format!("TRUSTED_PROXIES : {e}")))?;

        let worker_id = if raw.worker_id.trim().is_empty() {
            format!("worker-{}", uuid::Uuid::now_v7())
        } else {
            raw.worker_id
        };

        Ok(Self {
            database_url: Secret(raw.database_url),
            api_bind_addr: raw.api_bind_addr,
            api_docs_enabled: raw.api_docs_enabled,
            app_public_url: raw.app_public_url.trim_end_matches('/').to_owned(),
            worker_id,
            trusted_proxies,
            auth: AuthConfig {
                lockout_threshold: raw.auth_lockout_threshold,
                lockout_duration: raw.auth_lockout_duration,
                access_token_ttl: raw.auth_access_token_ttl,
                session_ttl: raw.auth_session_ttl,
                session_ttl_remembered: raw.auth_session_ttl_remembered,
                token_ttl: TokenTtls {
                    email_verification: raw.auth_token_ttl_email_verification,
                    password_reset: raw.auth_token_ttl_password_reset,
                    invitation: raw.auth_token_ttl_invitation,
                    magic_link: raw.auth_token_ttl_magic_link,
                    speaker_confirmation: raw.auth_token_ttl_speaker_confirmation,
                },
                signing_key: Secret(raw.auth_signing_key),
                cookie_secure: raw.auth_cookie_secure,
                cookie_domain: Some(raw.auth_cookie_domain).filter(|d| !d.trim().is_empty()),
            },
            mail: MailConfig {
                transport,
                relay_url: raw.mail_relay_url,
                relay_token: Secret(raw.mail_relay_token),
            },
            telemetry: TelemetryConfig {
                otlp_endpoint: Some(raw.otel_exporter_otlp_endpoint)
                    .filter(|e| !e.trim().is_empty()),
                service_name: raw.otel_service_name,
                log_filter: raw.rust_log,
            },
        })
    }
}
