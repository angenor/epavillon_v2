//! Lecture des droits : ce qu'une personne peut faire, et **d'où ça vient**.
//!
//! La base répond à la première question ; la seconde n'a de réponse nulle
//! part. `effective_permissions()` rend `(permission, portée)` — assez pour
//! autoriser, pas pour expliquer. L'origine se recompose ici, en repartant des
//! attributions et de ce que chaque rôle apporte.

use kernel::auth::{self, ScopeType};
use kernel::error::Result;
use sqlx::PgPool;
use std::collections::{BTreeMap, HashMap, HashSet};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::ids::PersonId;
use crate::domain::rbac::{
    AdministeredEvents, AssignmentState, EffectivePermission, EffectivePermissionRow,
    EffectivePermissionsView, PermissionGrant, PermissionModuleGroup, RoleAssignmentView, ScopeRef,
};
use crate::repo::rbac::{self, AssignmentRow, ResolvedScope};

/// Les attributions d'une personne, portées résolues et rôles nommés.
pub async fn assignments(
    pool: &PgPool,
    locale: &str,
    person_id: PersonId,
    actives_seulement: bool,
) -> Result<Vec<RoleAssignmentView>> {
    let lignes = rbac::assignments(pool, &[person_id.as_uuid()], actives_seulement).await?;
    composer_attributions(pool, locale, lignes).await
}

/// Les attributions **en cours** de plusieurs personnes, groupées par personne.
/// Une seule lecture pour toute une liste : la composer personne par personne
/// ferait autant de requêtes que de lignes affichées.
pub async fn active_assignments_by_person(
    pool: &PgPool,
    locale: &str,
    person_ids: &[Uuid],
) -> Result<HashMap<Uuid, Vec<RoleAssignmentView>>> {
    let lignes = rbac::assignments(pool, person_ids, true).await?;
    let vues = composer_attributions(pool, locale, lignes).await?;

    let mut par_personne: HashMap<Uuid, Vec<RoleAssignmentView>> = HashMap::new();
    for vue in vues {
        par_personne
            .entry(vue.person_id.as_uuid())
            .or_default()
            .push(vue);
    }
    Ok(par_personne)
}

pub async fn effective_permissions(
    pool: &PgPool,
    person_id: PersonId,
) -> Result<Vec<EffectivePermission>> {
    rbac::effective_permissions(pool, person_id).await
}

pub async fn administered_events(pool: &PgPool, person_id: PersonId) -> Result<AdministeredEvents> {
    Ok(auth::administered_events(pool, person_id.as_uuid())
        .await?
        .into())
}

/// L'écran « ce que cette personne peut faire, et où ».
pub async fn effective_permissions_view(
    pool: &PgPool,
    locale: &str,
    person_id: PersonId,
) -> Result<EffectivePermissionsView> {
    let attributions = assignments(pool, locale, person_id, true).await?;
    let catalogue = rbac::permission_catalog(pool).await?;
    let modules = rbac::module_labels(pool).await?;

    // Une permission peut venir de plusieurs rôles, sur plusieurs portées : la
    // question posée à l'écran est « pourquoi », et il peut y avoir plusieurs
    // réponses.
    let mut par_permission: BTreeMap<String, Vec<PermissionGrant>> = BTreeMap::new();
    for attribution in &attributions {
        for code in &attribution.role_permissions {
            par_permission
                .entry(code.clone())
                .or_default()
                .push(PermissionGrant {
                    scope_type: attribution.scope_type,
                    scope_id: attribution.scope_id,
                    scope_label: attribution.scope_label.clone(),
                    scope_hint: attribution.scope_hint.clone(),
                    is_dangling: attribution.is_dangling,
                    role_code: attribution.role_code.clone(),
                    role_label: attribution.role_label.clone(),
                    assignment_id: attribution.id,
                    valid_until: attribution.valid_until,
                });
        }
    }

    let mut groupes: BTreeMap<String, Vec<EffectivePermissionRow>> = BTreeMap::new();
    let mut manquantes = Vec::new();

    for permission in catalogue {
        match par_permission.remove(&permission.permission_code) {
            Some(grants) => groupes
                .entry(permission.module_code.clone())
                .or_default()
                .push(EffectivePermissionRow {
                    permission_code: permission.permission_code,
                    label: permission.label,
                    module_code: permission.module_code,
                    is_global: grants.iter().any(|g| g.scope_type == ScopeType::Global),
                    grants,
                }),
            None => manquantes.push(permission),
        }
    }

    let total = groupes.values().map(Vec::len).sum();
    let groups = groupes
        .into_iter()
        .map(|(module_code, rows)| PermissionModuleGroup {
            module_label: modules
                .get(&module_code)
                .cloned()
                .unwrap_or(serde_json::Value::Null),
            module_code,
            rows,
        })
        .collect();

    Ok(EffectivePermissionsView {
        person_id,
        groups,
        administered: administered_events(pool, person_id).await?,
        total,
        missing: manquantes,
    })
}

/// Résout les portées et les permissions de rôle d'un lot d'attributions.
async fn composer_attributions(
    pool: &PgPool,
    locale: &str,
    lignes: Vec<AssignmentRow>,
) -> Result<Vec<RoleAssignmentView>> {
    if lignes.is_empty() {
        return Ok(Vec::new());
    }

    let cibles: Vec<(ScopeType, Uuid)> = lignes
        .iter()
        .filter_map(|l| l.scope_id.map(|id| (l.scope_type, id)))
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    let portees = rbac::resolve_scopes(pool, locale, &cibles).await?;
    let permissions = rbac::role_permission_codes(pool).await?;
    let maintenant = OffsetDateTime::now_utc();

    Ok(lignes
        .into_iter()
        .map(|l| {
            let resolue = l
                .scope_id
                .and_then(|id| portees.get(&(l.scope_type, id)).cloned());
            let ScopeRef {
                scope_label,
                scope_hint,
                is_dangling,
                ..
            } = scope_ref(l.scope_type, l.scope_id, resolue);

            RoleAssignmentView {
                state: AssignmentState::compute(
                    l.revoked_at,
                    l.valid_from,
                    l.valid_until,
                    maintenant,
                ),
                role_permissions: permissions.get(&l.role_code).cloned().unwrap_or_default(),
                id: l.id,
                person_id: l.person_id,
                role_code: l.role_code,
                scope_type: l.scope_type,
                scope_id: l.scope_id,
                granted_by: l.granted_by,
                granted_at: l.granted_at,
                valid_from: l.valid_from,
                valid_until: l.valid_until,
                revoked_at: l.revoked_at,
                revoked_by: l.revoked_by,
                revoked_reason: l.revoked_reason,
                note: l.note,
                scope_label,
                scope_hint,
                is_dangling,
                role_label: l.role_label,
                role_description: l.role_description,
                role_is_system: l.role_is_system,
                granted_by_name: l.granted_by_name,
                revoked_by_name: l.revoked_by_name,
            }
        })
        .collect())
}

/// Une portée ciblée dont la cible est introuvable est **orpheline**, pas
/// globale : les deux se distinguent par `scope_type`, jamais par l'absence de
/// nom.
pub fn scope_ref(
    scope_type: ScopeType,
    scope_id: Option<Uuid>,
    resolue: Option<ResolvedScope>,
) -> ScopeRef {
    match scope_id {
        None => ScopeRef::global(),
        Some(id) => ScopeRef {
            scope_type,
            scope_id: Some(id),
            scope_label: resolue.as_ref().map(|r| r.label.clone()),
            scope_hint: resolue.as_ref().and_then(|r| r.hint.clone()),
            is_dangling: resolue.is_none(),
        },
    }
}
