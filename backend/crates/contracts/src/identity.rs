//! Événements du module `identity`.
//!
//! La forme du type est imposée par `ck_outbox_event_type_format` : trois
//! segments exactement. Les constantes évitent qu'une faute de frappe échoue à
//! l'exécution plutôt qu'à la compilation.
//!
//! **Aucune charge utile ne porte de secret** — ni mot de passe, ni empreinte,
//! ni jeton. `platform.outbox_events` est durable, indexée par agrégat, faite
//! pour être relue et rejouée : ce qu'on y dépose est là pour longtemps et pour
//! beaucoup de monde. Aucune adresse de courriel non plus, sauf quand elle
//! **est** le sujet de l'événement.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

pub const AGGREGATE_SCHEMA: &str = "identity";
pub const AGGREGATE_PERSON: &str = "person";
pub const AGGREGATE_ACCOUNT: &str = "account";
pub const AGGREGATE_ROLE_ASSIGNMENT: &str = "role_assignment";
pub const AGGREGATE_PRIVACY_REQUEST: &str = "privacy_request";

pub const PERSON_REGISTERED: &str = "identity.person.registered";
pub const PERSON_EMAIL_VERIFIED: &str = "identity.person.email_verified";
pub const PERSON_STATUS_CHANGED: &str = "identity.person.status_changed";
/// **Émis par la base** : `identity.anonymize_person()` appelle elle-même
/// `platform.emit_event()`. Le service qui l'invoque n'émet rien de plus —
/// deux lignes s'écriraient sans erreur, et tout consommateur idempotent
/// traiterait la première puis ignorerait la mauvaise.
pub const PERSON_ANONYMIZED: &str = "identity.person.anonymized";
pub const ACCOUNT_PASSWORD_CHANGED: &str = "identity.account.password_changed";
pub const ACCOUNT_LOCKED: &str = "identity.account.locked";
pub const ROLE_GRANTED: &str = "identity.role.granted";
pub const ROLE_REVOKED: &str = "identity.role.revoked";
pub const PRIVACY_REQUEST_RECEIVED: &str = "identity.privacy_request.received";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonRegistered {
    pub person_id: Uuid,
    pub preferred_locale: String,
    pub country_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonEmailVerified {
    pub person_id: Uuid,
    #[serde(with = "time::serde::rfc3339")]
    pub verified_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonStatusChanged {
    pub person_id: Uuid,
    pub previous_status: String,
    pub new_status: String,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub suspended_until: Option<OffsetDateTime>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonAnonymized {
    pub reason: Option<String>,
}

/// Par où le mot de passe a changé. `reset` vient d'un lien reçu par courriel,
/// `profile` d'un écran où la personne était déjà connectée.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PasswordChangeChannel {
    Reset,
    Profile,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountPasswordChanged {
    pub person_id: Uuid,
    pub channel: PasswordChangeChannel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountLocked {
    pub person_id: Uuid,
    #[serde(with = "time::serde::rfc3339")]
    pub locked_until: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleGranted {
    pub person_id: Uuid,
    pub role_code: String,
    pub scope_type: String,
    pub scope_id: Option<Uuid>,
    pub granted_by: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleRevoked {
    pub person_id: Uuid,
    pub role_code: String,
    pub scope_type: String,
    pub scope_id: Option<Uuid>,
    pub revoked_by: Option<Uuid>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyRequestReceived {
    pub request_id: Uuid,
    pub person_id: Uuid,
    pub request_type: String,
    #[serde(with = "time::serde::rfc3339")]
    pub due_at: OffsetDateTime,
}
