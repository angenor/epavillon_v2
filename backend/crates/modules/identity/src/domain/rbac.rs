//! Les droits, tels que l'API les rend.
//!
//! `effective_permissions()` rend `(permission, portée)` sans dire d'où ça
//! vient : suffisant pour autoriser, insuffisant pour expliquer. L'écran
//! demandé est un écran d'explication — d'où l'enrichissement par l'origine,
//! qui n'existe nulle part en base et se compose ici.

use kernel::auth::{AdminScope, ScopeType};
use serde::Serialize;
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use super::ids::{PersonId, RoleAssignmentId};

/// Une portée, avec le nom de ce qu'elle désigne.
///
/// `role_assignments.scope_id` n'a aucune clé étrangère : la cible vit dans un
/// autre module. Le nom se résout donc par une jointure applicative, et il peut
/// manquer — une édition supprimée laisse une attribution orpheline, que
/// l'écran doit montrer comme telle plutôt que de taire.
#[derive(Debug, Clone, Serialize)]
pub struct ScopeRef {
    pub scope_type: ScopeType,
    pub scope_id: Option<Uuid>,
    pub scope_label: Option<Value>,
    pub scope_hint: Option<String>,
    pub is_dangling: bool,
}

impl ScopeRef {
    pub fn global() -> Self {
        Self {
            scope_type: ScopeType::Global,
            scope_id: None,
            scope_label: None,
            scope_hint: None,
            is_dangling: false,
        }
    }
}

/// État calculé d'une attribution : il n'existe pas en base, il se déduit de
/// `revoked_at`, `valid_from` et `valid_until`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AssignmentState {
    Active,
    Scheduled,
    Expired,
    Revoked,
}

impl AssignmentState {
    pub fn compute(
        revoked_at: Option<OffsetDateTime>,
        valid_from: OffsetDateTime,
        valid_until: Option<OffsetDateTime>,
        maintenant: OffsetDateTime,
    ) -> Self {
        if revoked_at.is_some() {
            Self::Revoked
        } else if valid_from > maintenant {
            Self::Scheduled
        } else if valid_until.is_some_and(|fin| fin <= maintenant) {
            Self::Expired
        } else {
            Self::Active
        }
    }
}

/// Une attribution telle que l'écran la lit : la ligne de la table, sa portée
/// résolue, son rôle nommé, et les personnes qui l'ont accordée puis retirée.
#[derive(Debug, Clone, Serialize)]
pub struct RoleAssignmentView {
    pub id: RoleAssignmentId,
    pub person_id: PersonId,
    pub role_code: String,
    pub scope_type: ScopeType,
    pub scope_id: Option<Uuid>,
    pub granted_by: Option<PersonId>,
    #[serde(with = "time::serde::rfc3339")]
    pub granted_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub valid_from: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub valid_until: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub revoked_at: Option<OffsetDateTime>,
    pub revoked_by: Option<PersonId>,
    pub revoked_reason: Option<String>,
    pub note: Option<String>,
    pub scope_label: Option<Value>,
    pub scope_hint: Option<String>,
    pub is_dangling: bool,
    pub role_label: Value,
    pub role_description: Option<Value>,
    pub role_is_system: bool,
    pub role_permissions: Vec<String>,
    pub granted_by_name: Option<String>,
    pub revoked_by_name: Option<String>,
    pub state: AssignmentState,
}

/// Ligne brute de `identity.effective_permissions()` — ce que le site consomme
/// pour afficher ou masquer une entrée de menu. Le refus, lui, reste à l'API.
#[derive(Debug, Clone, Serialize)]
pub struct EffectivePermission {
    pub permission_code: String,
    pub scope_type: ScopeType,
    pub scope_id: Option<Uuid>,
}

/// Le périmètre d'administration, sous la forme que le site consomme déjà.
/// **Jamais nul** : les trois cas se lisent sans ambiguïté.
#[derive(Debug, Clone, Serialize)]
pub struct AdministeredEvents {
    pub is_global: bool,
    pub event_ids: Vec<Uuid>,
}

impl From<AdminScope> for AdministeredEvents {
    fn from(scope: AdminScope) -> Self {
        Self {
            is_global: scope.is_global,
            event_ids: scope.event_ids,
        }
    }
}

/// D'où vient une permission : quel rôle l'apporte, et sur quelle portée.
#[derive(Debug, Clone, Serialize)]
pub struct PermissionGrant {
    pub scope_type: ScopeType,
    pub scope_id: Option<Uuid>,
    pub scope_label: Option<Value>,
    pub scope_hint: Option<String>,
    pub is_dangling: bool,
    pub role_code: String,
    pub role_label: Value,
    pub assignment_id: RoleAssignmentId,
    #[serde(with = "time::serde::rfc3339::option")]
    pub valid_until: Option<OffsetDateTime>,
}

/// Une permission effective, et tout ce qui l'accorde. Il peut y avoir
/// plusieurs octrois pour la même permission.
#[derive(Debug, Clone, Serialize)]
pub struct EffectivePermissionRow {
    pub permission_code: String,
    pub label: Value,
    pub module_code: String,
    pub is_global: bool,
    pub grants: Vec<PermissionGrant>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PermissionModuleGroup {
    pub module_code: String,
    pub module_label: Value,
    pub rows: Vec<EffectivePermissionRow>,
}

/// Une permission du catalogue que la personne n'a pas — l'autre moitié de la
/// réponse à « que peut faire cette personne ? ».
#[derive(Debug, Clone, Serialize)]
pub struct MissingPermission {
    pub permission_code: String,
    pub label: Value,
    pub module_code: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct EffectivePermissionsView {
    pub person_id: PersonId,
    pub groups: Vec<PermissionModuleGroup>,
    pub administered: AdministeredEvents,
    pub total: usize,
    pub missing: Vec<MissingPermission>,
}
