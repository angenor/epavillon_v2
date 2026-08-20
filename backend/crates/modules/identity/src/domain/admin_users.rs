//! Les écrans d'utilisateurs du back-office, tels que l'API les rend.
//!
//! La **personne** et le **compte** restent deux choses distinctes jusque dans
//! les noms de champ : `has_account` vaut faux pour quelqu'un créé par une
//! invitation, qui existe sans jamais s'être connecté. Les confondre ferait
//! afficher « jamais connecté » là où il faut lire « aucun compte ».

use kernel::auth::ScopeType;
use serde::Serialize;
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use super::ids::{AccountId, PersonId, RoleAssignmentId};
use super::login::PersonStatus;
use super::rbac::{EffectivePermissionsView, RoleAssignmentView, ScopeRef};
use super::scope::ScopeChoice;

#[derive(Debug, Clone, Serialize)]
pub struct UserListRow {
    pub person_id: PersonId,
    pub display_name: String,
    pub primary_email: String,
    #[serde(with = "time::serde::rfc3339::option")]
    pub email_verified_at: Option<OffsetDateTime>,
    pub job_title: Option<String>,
    pub country_name: Option<Value>,
    pub country_id: Option<Uuid>,
    pub organization_id: Option<Uuid>,
    pub organization_name: Option<String>,
    pub organization_acronym: Option<String>,
    pub status: PersonStatus,
    pub status_reason: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub suspended_until: Option<OffsetDateTime>,
    /// Attributions **en cours** seulement : l'historique se lit sur la fiche.
    pub roles: Vec<RoleAssignmentView>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub last_login_at: Option<OffsetDateTime>,
    pub has_account: bool,
    pub mfa_enabled: bool,
    #[serde(with = "time::serde::rfc3339::option")]
    pub locked_until: Option<OffsetDateTime>,
    pub open_privacy_request: Option<PrivacyRequestType>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize)]
pub struct UserFacet {
    pub value: String,
    pub label: Value,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct UserListScreen {
    pub rows: Vec<UserListRow>,
    pub roles: Vec<AssignableRole>,
    pub countries: Vec<UserFacet>,
    pub organizations: Vec<UserFacet>,
    /// La liste a-t-elle été restreinte aux éditions administrées ?
    pub scoped_to_events: bool,
    pub open_privacy_requests: usize,
    pub restricted_accounts: usize,
}

/// Un rôle du catalogue, vu par celui qui attribue.
///
/// `allowed_scopes` vient de la base et dit ce que le rôle admet.
/// `grantable_scopes` — le croisement avec ce que l'acteur détient — appartient
/// au panneau d'attribution, qui est une écriture : il arrive avec elle.
#[derive(Debug, Clone, Serialize)]
pub struct AssignableRole {
    pub code: String,
    pub label: Value,
    pub description: Option<Value>,
    pub allowed_scopes: Vec<ScopeType>,
    pub is_system: bool,
    pub permissions: Vec<RolePermissionView>,
    pub active_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct RolePermissionView {
    pub code: String,
    pub label: Value,
    pub module_code: String,
}

/// Ce que le panneau d'attribution a besoin de savoir, en une réponse.
///
/// `negotiation_spaces` est **vide** et le restera tant que le module
/// Négociations n'a pas d'écran : le rôle qui admet cette portée existe en base,
/// le panneau l'affiche donc, désactivé et expliqué. Offrir un choix sans cible
/// vaudrait moins qu'un choix vide.
#[derive(Debug, Clone, Serialize)]
pub struct RoleAssignmentOptions {
    pub roles: Vec<AssignableRole>,
    pub events: Vec<ScopeChoice>,
    pub organizations: Vec<ScopeChoice>,
    pub negotiation_spaces: Vec<ScopeChoice>,
    /// L'acteur peut-il attribuer sur la portée **globale** ?
    pub can_assign_global: bool,
    /// Éditions sur lesquelles il peut attribuer — vide s'il est global, qui les
    /// couvre déjà toutes.
    pub grantable_event_ids: Vec<Uuid>,
}

/// Un compte de connexion, secrets exclus.
#[derive(Debug, Clone, Serialize)]
pub struct AccountView {
    pub id: AccountId,
    pub provider: String,
    #[serde(with = "time::serde::rfc3339::option")]
    pub last_login_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub password_changed_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub mfa_enabled_at: Option<OffsetDateTime>,
    pub failed_attempts: i16,
    #[serde(with = "time::serde::rfc3339::option")]
    pub locked_until: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AssignmentEvent {
    Granted,
    Revoked,
}

/// Une entrée de l'historique. Deux événements peuvent naître d'une même ligne
/// de table : une attribution n'est jamais supprimée, le retrait s'y ajoute.
#[derive(Debug, Clone, Serialize)]
pub struct AssignmentHistoryEntry {
    pub assignment_id: RoleAssignmentId,
    pub kind: AssignmentEvent,
    #[serde(with = "time::serde::rfc3339")]
    pub occurred_at: OffsetDateTime,
    pub role_code: String,
    pub role_label: Value,
    pub scope: ScopeRef,
    pub actor_name: Option<String>,
    /// `note` pour un octroi, `revoked_reason` pour un retrait — jamais l'un
    /// pour l'autre.
    pub reason: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub valid_until: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConsentView {
    pub purpose: String,
    pub is_granted: bool,
    pub policy_version: String,
    #[serde(with = "time::serde::rfc3339")]
    pub recorded_at: OffsetDateTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PrivacyRequestType {
    Export,
    Erasure,
    Rectification,
}

impl PrivacyRequestType {
    pub fn as_db(self) -> &'static str {
        match self {
            Self::Export => "export",
            Self::Erasure => "erasure",
            Self::Rectification => "rectification",
        }
    }

    pub fn from_db(valeur: &str) -> Option<Self> {
        match valeur {
            "export" => Some(Self::Export),
            "erasure" => Some(Self::Erasure),
            "rectification" => Some(Self::Rectification),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PrivacyRequestStatus {
    Received,
    InProgress,
    Completed,
    Rejected,
}

impl PrivacyRequestStatus {
    pub fn as_db(self) -> &'static str {
        match self {
            Self::Received => "received",
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
            Self::Rejected => "rejected",
        }
    }

    pub fn from_db(valeur: &str) -> Option<Self> {
        match valeur {
            "received" => Some(Self::Received),
            "in_progress" => Some(Self::InProgress),
            "completed" => Some(Self::Completed),
            "rejected" => Some(Self::Rejected),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PrivacyRequestView {
    pub id: Uuid,
    pub person_id: PersonId,
    pub person_name: String,
    pub person_email: String,
    pub request_type: PrivacyRequestType,
    pub status: PrivacyRequestStatus,
    #[serde(with = "time::serde::rfc3339")]
    pub due_at: OffsetDateTime,
    pub days_left: i32,
    pub is_overdue: bool,
    pub handled_by_name: Option<String>,
    pub resolution: Option<String>,
    pub result_asset_id: Option<Uuid>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub completed_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PersonEmailView {
    pub email: String,
    pub label: String,
    #[serde(with = "time::serde::rfc3339::option")]
    pub verified_at: Option<OffsetDateTime>,
}

/// La fiche complète, en une réponse.
///
/// `in_scope` faux ne cache rien : la fiche reste lisible, c'est l'écriture qui
/// se refusera. Masquer la personne ferait croire à sa disparition à un
/// administrateur qui la sait présente.
#[derive(Debug, Clone, Serialize)]
pub struct UserDetail {
    pub person_id: PersonId,
    pub display_name: String,
    pub first_name: String,
    pub last_name: String,
    pub civility: Option<String>,
    pub primary_email: String,
    #[serde(with = "time::serde::rfc3339::option")]
    pub email_verified_at: Option<OffsetDateTime>,
    pub other_emails: Vec<PersonEmailView>,
    pub phone: Option<String>,
    pub job_title: Option<String>,
    pub biography: Option<Value>,
    pub country_id: Option<Uuid>,
    pub country_name: Option<Value>,
    pub city: Option<String>,
    pub preferred_locale: String,
    pub timezone: String,
    pub organization_id: Option<Uuid>,
    pub organization_name: Option<String>,
    pub is_directory_visible: bool,
    pub status: PersonStatus,
    pub status_reason: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub status_changed_at: Option<OffsetDateTime>,
    pub status_changed_by_name: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub suspended_until: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    pub accounts: Vec<AccountView>,
    pub assignments: Vec<RoleAssignmentView>,
    pub history: Vec<AssignmentHistoryEntry>,
    pub permissions: EffectivePermissionsView,
    pub consents: Vec<ConsentView>,
    pub privacy_requests: Vec<PrivacyRequestView>,
    pub in_scope: bool,
}
