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
    /// Jeton du sens INVERSE — le relais qui remonte ce que le fournisseur dit
    /// d'un courriel. **Vide vaut « route non montée »**, jamais « route
    /// ouverte » (B6, R30).
    #[serde(default)]
    mail_webhook_token: String,

    #[serde(default = "default_media_storage")]
    media_storage: String,
    #[serde(default = "default_media_fs_root")]
    media_fs_root: String,
    #[serde(default = "default_media_max_upload_bytes")]
    media_max_upload_bytes: u64,
    #[serde(default = "default_media_scanner")]
    media_scanner: String,
    #[serde(default)]
    media_clamd_addr: String,
    #[serde(default = "default_media_scan_max_bytes")]
    media_scan_max_bytes: u64,
    #[serde(with = "humantime_serde", default = "hours_6")]
    media_purge_interval: Duration,
    #[serde(with = "humantime_serde", default = "hours_24")]
    media_reconcile_interval: Duration,

    #[serde(default = "default_s3_endpoint")]
    s3_endpoint: String,
    #[serde(default = "default_s3_region")]
    s3_region: String,
    #[serde(default = "default_s3_bucket")]
    s3_bucket: String,
    #[serde(default)]
    s3_access_key_id: String,
    #[serde(default)]
    s3_secret_access_key: String,
    #[serde(default = "vrai")]
    s3_force_path_style: bool,

    #[serde(with = "humantime_serde", default = "hours_24")]
    engagement_partition_interval: Duration,

    #[serde(default = "default_duplicate_score_threshold")]
    org_duplicate_score_threshold: u16,
    #[serde(default = "default_duplicate_scan_batch")]
    org_duplicate_scan_batch: u32,
    #[serde(with = "humantime_serde", default = "minutes_5")]
    org_scorecard_refresh_window: Duration,

    #[serde(with = "humantime_serde", default = "hours_1")]
    event_call_autoclose_interval: Duration,

    /// Version de la politique de confidentialité opposée à qui consent, écrite
    /// dans `identity.consents.policy_version`. C'est un réglage
    /// d'exploitation : la mettre en base la rendrait modifiable par migration
    /// seulement, et une preuve de consentement doit nommer le texte accepté.
    #[serde(default = "default_privacy_policy_version")]
    privacy_policy_version: String,

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
fn default_privacy_policy_version() -> String {
    "2026-01".to_owned()
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
fn default_duplicate_score_threshold() -> u16 {
    60
}
fn default_duplicate_scan_batch() -> u32 {
    200
}
fn default_media_storage() -> String {
    "s3".to_owned()
}
fn default_media_fs_root() -> String {
    "./.media".to_owned()
}
/// 200 Mio : le poids du fond vidéo de la page d'accueil, le plus gros fichier
/// que la plateforme accepte.
fn default_media_max_upload_bytes() -> u64 {
    209_715_200
}
fn default_media_scanner() -> String {
    "none".to_owned()
}
/// 50 Mio. Au-delà, verdict « non pris en charge » plutôt qu'une analyse de
/// cinq minutes qui bloque un fil du worker.
fn default_media_scan_max_bytes() -> u64 {
    52_428_800
}
fn default_s3_endpoint() -> String {
    "http://localhost:3900".to_owned()
}
fn default_s3_region() -> String {
    "garage".to_owned()
}
fn default_s3_bucket() -> String {
    "epavillon".to_owned()
}
fn hours_6() -> Duration {
    Duration::from_secs(6 * 3600)
}
fn minutes_5() -> Duration {
    Duration::from_secs(5 * 60)
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
    pub org: OrgConfig,
    pub event: EventConfig,
    pub programme: ProgrammeConfig,
    pub media: MediaConfig,
    pub engagement: EngagementConfig,
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

/// Réglages du module Organisations. Le seuil s'exprime sur l'échelle de
/// `org.find_similar_organizations()`, qui monte à 175 — nom entier 100,
/// domaine partagé 40, sigle exact 25, pays 10.
#[derive(Debug, Clone)]
pub struct OrgConfig {
    pub duplicate_score_threshold: u16,
    pub duplicate_scan_batch: u32,
    pub scorecard_refresh_window: Duration,
}

/// Réglages du module Événements. Une seule clé : la cadence à laquelle le
/// worker clôt les appels dont l'échéance effective est passée.
#[derive(Debug, Clone)]
pub struct EventConfig {
    pub call_autoclose_interval: Duration,
}

/// Réglages du module Programmation. Une seule clé : la version de la politique
/// de confidentialité que porte la preuve d'un consentement recueilli au
/// formulaire d'inscription (B5, R22).
#[derive(Debug, Clone)]
pub struct ProgrammeConfig {
    pub privacy_policy_version: String,
}

/// Où le module Média dépose les octets. Deux implémentations, choisies par la
/// configuration — exactement le patron de `MailTransport`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaStorage {
    /// Garage, ou tout stockage compatible S3. Le mode normal.
    S3,
    /// Des fichiers sous `fs_root`. Celui des tests d'intégration : `check-db`
    /// efface le layout de Garage, et des tests qui le frapperaient
    /// échoueraient après chaque vérification complète (B6, R7).
    Filesystem,
}

/// Le moteur d'analyse antivirus. `None` est un moteur DÉCLARÉ, pas une
/// absence : il rend « non pris en charge » et jamais « sain » (B6, R13).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaScanner {
    None,
    Clamd,
}

/// Identifiants du stockage objet. Ils existent dans `.env.example` depuis le
/// 16/08 et n'avaient jamais servi : B6 est le premier jalon qui écrit un octet.
#[derive(Debug, Clone)]
pub struct S3Config {
    pub endpoint: String,
    pub region: String,
    pub bucket: String,
    pub access_key_id: String,
    pub secret_access_key: Secret,
    /// `true` : l'adresse porte le nom du bucket dans son chemin plutôt que
    /// dans son nom d'hôte. Garage ne sait faire que cela.
    pub force_path_style: bool,
}

/// Réglages du module Média.
#[derive(Debug, Clone)]
pub struct MediaConfig {
    pub storage: MediaStorage,
    pub fs_root: String,
    /// Plafond ABSOLU d'un dépôt. Il ne remplace pas les limites par rôle, qui
    /// vivent en base (`media.attachable_roles.max_byte_size`) — il les couvre
    /// toutes.
    pub max_upload_bytes: u64,
    pub scanner: MediaScanner,
    pub clamd_addr: String,
    pub scan_max_bytes: u64,
    pub purge_interval: Duration,
    pub reconcile_interval: Duration,
    pub s3: S3Config,
}

/// Réglages du module Engagement. Une seule clé : la cadence à laquelle le
/// worker prépare les partitions mensuelles du journal d'expédition. Le modèle
/// amorce le trimestre courant puis annonce un worker de maintenance qui
/// n'existait pas (écart n° 137) : sans lui, la purge par bascule de partition
/// cesse de fonctionner au bout de trois mois.
#[derive(Debug, Clone)]
pub struct EngagementConfig {
    pub partition_interval: Duration,
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
    /// Le secret de SORTIE : l'API se fait reconnaître du site quand elle lui
    /// remet un message.
    pub relay_token: Secret,
    /// Le secret d'ENTRÉE : le site se fait reconnaître de l'API quand il lui
    /// remonte ce que le fournisseur a dit d'un courriel. **Deux jetons et non
    /// un** — confondre les deux ferait d'un jeton de sortie un jeton d'entrée,
    /// donc ouvrirait la porte d'ingestion à qui a seulement lu la
    /// configuration du relais (B6, R30).
    ///
    /// `None` ne signifie pas « route ouverte » mais **route non montée** :
    /// elle rend 404, comme un module éteint.
    pub webhook_token: Option<Secret>,
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

        // L'échelle du score va de 0 à 175 : un seuil au-delà ne laisserait
        // jamais entrer une paire, et la file resterait vide sans que rien ne
        // le dise.
        if raw.org_duplicate_score_threshold > 175 {
            return Err(invalid(format!(
                "ORG_DUPLICATE_SCORE_THRESHOLD vaut {} : l'échelle du score va de 0 à 175, et aucune paire n'entrerait dans la file.",
                raw.org_duplicate_score_threshold
            )));
        }
        if raw.org_duplicate_scan_batch == 0 {
            return Err(invalid(
                "ORG_DUPLICATE_SCAN_BATCH vaut 0 : le balayage n'avancerait jamais.",
            ));
        }
        if raw.org_scorecard_refresh_window.is_zero() {
            return Err(invalid(
                "ORG_SCORECARD_REFRESH_WINDOW vaut une durée nulle.",
            ));
        }

        // Une cadence nulle ferait replanifier la clôture des appels à
        // l'instant même où elle vient de tourner : la file se remplirait
        // aussi vite que le worker la vide.
        if raw.event_call_autoclose_interval.is_zero() {
            return Err(invalid(
                "EVENT_CALL_AUTOCLOSE_INTERVAL vaut une durée nulle : la clôture des appels se replanifierait sans fin.",
            ));
        }

        // Vide, la preuve de consentement ne nommerait aucun texte : une
        // colonne NOT NULL remplie d'une chaîne vide est une preuve qui
        // n'oppose rien.
        if raw.privacy_policy_version.trim().is_empty() {
            return Err(invalid(
                "PRIVACY_POLICY_VERSION est vide : une preuve de consentement doit nommer le texte accepté.",
            ));
        }

        let media_storage = match raw.media_storage.as_str() {
            "s3" => MediaStorage::S3,
            "filesystem" => MediaStorage::Filesystem,
            other => {
                return Err(invalid(format!(
                    "MEDIA_STORAGE vaut « {other} » : valeurs acceptées, s3 ou filesystem."
                )))
            }
        };
        if media_storage == MediaStorage::S3 {
            // Sans clé, chaque dépôt échouerait en 403 au moment de signer —
            // une panne à l'exécution là où le démarrage peut le dire.
            if raw.s3_access_key_id.trim().is_empty() || raw.s3_secret_access_key.trim().is_empty()
            {
                return Err(invalid(
                    "S3_ACCESS_KEY_ID ou S3_SECRET_ACCESS_KEY est vide alors que MEDIA_STORAGE vaut s3 : aucun fichier ne pourrait être déposé. Lancer `make garage-init`.",
                ));
            }
            if !raw.s3_endpoint.starts_with("http://") && !raw.s3_endpoint.starts_with("https://") {
                return Err(invalid(
                    "S3_ENDPOINT doit être une adresse absolue (http:// ou https://).",
                ));
            }
        }
        if media_storage == MediaStorage::Filesystem && raw.media_fs_root.trim().is_empty() {
            return Err(invalid(
                "MEDIA_FS_ROOT est vide alors que MEDIA_STORAGE vaut filesystem.",
            ));
        }

        let media_scanner = match raw.media_scanner.as_str() {
            "none" => MediaScanner::None,
            "clamd" => MediaScanner::Clamd,
            other => {
                return Err(invalid(format!(
                    "MEDIA_SCANNER vaut « {other} » : valeurs acceptées, none ou clamd."
                )))
            }
        };
        if media_scanner == MediaScanner::Clamd && raw.media_clamd_addr.trim().is_empty() {
            return Err(invalid(
                "MEDIA_CLAMD_ADDR est vide alors que MEDIA_SCANNER vaut clamd : aucun fichier ne serait analysé.",
            ));
        }

        if raw.media_max_upload_bytes == 0 {
            return Err(invalid(
                "MEDIA_MAX_UPLOAD_BYTES vaut 0 : aucun fichier ne pourrait être déposé.",
            ));
        }

        // Une cadence nulle ferait replanifier le travail à l'instant même où
        // il vient de tourner : la file se remplirait aussi vite qu'elle se
        // vide. Même raisonnement que pour la clôture des appels de B3.
        for (cle, duree) in [
            ("MEDIA_PURGE_INTERVAL", raw.media_purge_interval),
            ("MEDIA_RECONCILE_INTERVAL", raw.media_reconcile_interval),
            (
                "ENGAGEMENT_PARTITION_INTERVAL",
                raw.engagement_partition_interval,
            ),
        ] {
            if duree.is_zero() {
                return Err(invalid(format!(
                    "{cle} vaut une durée nulle : le travail se replanifierait sans fin."
                )));
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
            org: OrgConfig {
                duplicate_score_threshold: raw.org_duplicate_score_threshold,
                duplicate_scan_batch: raw.org_duplicate_scan_batch,
                scorecard_refresh_window: raw.org_scorecard_refresh_window,
            },
            event: EventConfig {
                call_autoclose_interval: raw.event_call_autoclose_interval,
            },
            programme: ProgrammeConfig {
                privacy_policy_version: raw.privacy_policy_version.trim().to_owned(),
            },
            media: MediaConfig {
                storage: media_storage,
                fs_root: raw.media_fs_root,
                max_upload_bytes: raw.media_max_upload_bytes,
                scanner: media_scanner,
                clamd_addr: raw.media_clamd_addr,
                scan_max_bytes: raw.media_scan_max_bytes,
                purge_interval: raw.media_purge_interval,
                reconcile_interval: raw.media_reconcile_interval,
                s3: S3Config {
                    endpoint: raw.s3_endpoint.trim_end_matches('/').to_owned(),
                    region: raw.s3_region,
                    bucket: raw.s3_bucket,
                    access_key_id: raw.s3_access_key_id,
                    secret_access_key: Secret(raw.s3_secret_access_key),
                    force_path_style: raw.s3_force_path_style,
                },
            },
            engagement: EngagementConfig {
                partition_interval: raw.engagement_partition_interval,
            },
            mail: MailConfig {
                transport,
                relay_url: raw.mail_relay_url,
                relay_token: Secret(raw.mail_relay_token),
                webhook_token: Some(raw.mail_webhook_token)
                    .filter(|j| !j.trim().is_empty())
                    .map(Secret),
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
