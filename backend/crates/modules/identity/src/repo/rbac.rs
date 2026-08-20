//! Lectures du RBAC : attributions, catalogue, permissions effectives.
//!
//! **Aucune décision ici.** Le repo lit ; c'est le service qui compose et qui
//! refuse. Les portées, elles, se résolvent hors du schéma `identity` : une
//! attribution ne porte aucune clé étrangère vers sa cible, et la cible vit
//! dans un autre module — la jointure est applicative, et elle peut ne rien
//! trouver.

use kernel::auth::ScopeType;
use kernel::error::{ApiError, Result};
use serde_json::Value;
use sqlx::postgres::PgConnection;
use sqlx::PgPool;
use std::collections::HashMap;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::admin_users::{AssignableRole, RolePermissionView};
use crate::domain::ids::{PersonId, RoleAssignmentId};
use crate::domain::rbac::{EffectivePermission, MissingPermission};
use crate::domain::scope::ScopeChoice;

/// Une ligne d'attribution, son rôle joint, et le nom des deux personnes qui
/// l'ont accordée puis retirée. La portée n'est pas encore résolue.
#[derive(Debug, Clone)]
pub struct AssignmentRow {
    pub id: RoleAssignmentId,
    pub person_id: PersonId,
    pub role_code: String,
    pub scope_type: ScopeType,
    pub scope_id: Option<Uuid>,
    pub granted_by: Option<PersonId>,
    pub granted_at: OffsetDateTime,
    pub valid_from: OffsetDateTime,
    pub valid_until: Option<OffsetDateTime>,
    pub revoked_at: Option<OffsetDateTime>,
    pub revoked_by: Option<PersonId>,
    pub revoked_reason: Option<String>,
    pub note: Option<String>,
    pub role_label: Value,
    pub role_description: Option<Value>,
    pub role_is_system: bool,
    pub granted_by_name: Option<String>,
    pub revoked_by_name: Option<String>,
}

/// Les attributions de plusieurs personnes en une lecture.
///
/// `actives_seulement` sépare deux questions qui se ressemblent : ce qu'une
/// personne peut faire aujourd'hui, et ce qu'on lui a confié depuis toujours.
/// Une attribution révoquée n'est jamais supprimée — la seconde question a donc
/// toujours une réponse.
pub async fn assignments(
    pool: &PgPool,
    person_ids: &[Uuid],
    actives_seulement: bool,
) -> Result<Vec<AssignmentRow>> {
    let lignes = sqlx::query!(
        r#"SELECT ra.id,
                  ra.person_id,
                  ra.role_code,
                  ra.scope_type::text AS "scope_type!",
                  ra.scope_id,
                  ra.granted_by,
                  ra.granted_at,
                  ra.valid_from,
                  ra.valid_until,
                  ra.revoked_at,
                  ra.revoked_by,
                  ra.revoked_reason,
                  ra.note,
                  r.label       AS "role_label!: Value",
                  r.description AS "role_description?: Value",
                  r.is_system   AS "role_is_system!",
                  g.display_name AS "granted_by_name?",
                  v.display_name AS "revoked_by_name?"
             FROM identity.role_assignments ra
             JOIN identity.roles r ON r.code = ra.role_code
             LEFT JOIN identity.people g ON g.id = ra.granted_by
             LEFT JOIN identity.people v ON v.id = ra.revoked_by
            WHERE ra.person_id = ANY($1)
              AND (NOT $2 OR (ra.revoked_at IS NULL
                              AND ra.valid_from <= now()
                              AND (ra.valid_until IS NULL OR ra.valid_until > now())))
            ORDER BY ra.granted_at DESC"#,
        person_ids,
        actives_seulement
    )
    .fetch_all(pool)
    .await?;

    lignes
        .into_iter()
        .map(|l| {
            Ok(AssignmentRow {
                id: RoleAssignmentId(l.id),
                person_id: PersonId(l.person_id),
                role_code: l.role_code,
                scope_type: portee(&l.scope_type)?,
                scope_id: l.scope_id,
                granted_by: l.granted_by.map(PersonId),
                granted_at: l.granted_at,
                valid_from: l.valid_from,
                valid_until: l.valid_until,
                revoked_at: l.revoked_at,
                revoked_by: l.revoked_by.map(PersonId),
                revoked_reason: l.revoked_reason,
                note: l.note,
                role_label: l.role_label,
                role_description: l.role_description,
                role_is_system: l.role_is_system,
                granted_by_name: l.granted_by_name,
                revoked_by_name: l.revoked_by_name,
            })
        })
        .collect()
}

/// Ce que chaque rôle apporte. Lu une fois et partagé : le catalogue est court
/// et ne change qu'à la migration.
pub async fn role_permission_codes(pool: &PgPool) -> Result<HashMap<String, Vec<String>>> {
    let lignes = sqlx::query!(
        "SELECT role_code, permission_code FROM identity.role_permissions ORDER BY permission_code"
    )
    .fetch_all(pool)
    .await?;

    let mut par_role: HashMap<String, Vec<String>> = HashMap::new();
    for ligne in lignes {
        par_role
            .entry(ligne.role_code)
            .or_default()
            .push(ligne.permission_code);
    }
    Ok(par_role)
}

pub async fn effective_permissions(
    pool: &PgPool,
    person_id: PersonId,
) -> Result<Vec<EffectivePermission>> {
    let lignes = sqlx::query!(
        r#"SELECT permission_code AS "permission_code!",
                  scope_type::text AS "scope_type!",
                  scope_id
             FROM identity.effective_permissions($1)
            ORDER BY permission_code"#,
        person_id.as_uuid()
    )
    .fetch_all(pool)
    .await?;

    lignes
        .into_iter()
        .map(|l| {
            Ok(EffectivePermission {
                permission_code: l.permission_code,
                scope_type: portee(&l.scope_type)?,
                scope_id: l.scope_id,
            })
        })
        .collect()
}

/// Le catalogue complet des permissions, avec leur module. Sert les deux
/// moitiés de l'écran d'explication : ce que la personne détient, et ce qui lui
/// manque.
pub async fn permission_catalog(pool: &PgPool) -> Result<Vec<MissingPermission>> {
    let lignes = sqlx::query!(
        r#"SELECT code, label AS "label!: Value", module_code
             FROM identity.permissions
            ORDER BY module_code, code"#
    )
    .fetch_all(pool)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| MissingPermission {
            permission_code: l.code,
            label: l.label,
            module_code: l.module_code,
        })
        .collect())
}

pub async fn module_labels(pool: &PgPool) -> Result<HashMap<String, Value>> {
    let lignes = sqlx::query!(
        r#"SELECT code, display_name AS "display_name!: Value" FROM platform.modules"#
    )
    .fetch_all(pool)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| (l.code, l.display_name))
        .collect())
}

/// Le catalogue des rôles, avec ce que chacun apporte et le nombre
/// d'attributions en cours — toutes portées confondues.
pub async fn assignable_roles(pool: &PgPool) -> Result<Vec<AssignableRole>> {
    let lignes = sqlx::query!(
        r#"SELECT r.code,
                  r.label          AS "label!: Value",
                  r.description    AS "description?: Value",
                  r.allowed_scopes AS "allowed_scopes!",
                  r.is_system,
                  (SELECT count(*) FROM identity.role_assignments ra
                    WHERE ra.role_code = r.code
                      AND ra.revoked_at IS NULL
                      AND ra.valid_from <= now()
                      AND (ra.valid_until IS NULL OR ra.valid_until > now())) AS "active_count!"
             FROM identity.roles r
            ORDER BY r.code"#
    )
    .fetch_all(pool)
    .await?;

    let permissions = role_permissions_detaillees(pool).await?;

    lignes
        .into_iter()
        .map(|l| {
            let allowed_scopes = l
                .allowed_scopes
                .iter()
                .map(|s| portee(s))
                .collect::<Result<Vec<_>>>()?;

            Ok(AssignableRole {
                permissions: permissions.get(&l.code).cloned().unwrap_or_default(),
                code: l.code,
                label: l.label,
                description: l.description,
                allowed_scopes,
                is_system: l.is_system,
                active_count: l.active_count as usize,
            })
        })
        .collect()
}

async fn role_permissions_detaillees(
    pool: &PgPool,
) -> Result<HashMap<String, Vec<RolePermissionView>>> {
    let lignes = sqlx::query!(
        r#"SELECT rp.role_code,
                  p.code,
                  p.label AS "label!: Value",
                  p.module_code
             FROM identity.role_permissions rp
             JOIN identity.permissions p ON p.code = rp.permission_code
            ORDER BY p.module_code, p.code"#
    )
    .fetch_all(pool)
    .await?;

    let mut par_role: HashMap<String, Vec<RolePermissionView>> = HashMap::new();
    for ligne in lignes {
        par_role
            .entry(ligne.role_code)
            .or_default()
            .push(RolePermissionView {
                code: ligne.code,
                label: ligne.label,
                module_code: ligne.module_code,
            });
    }
    Ok(par_role)
}

// -----------------------------------------------------------------------------
// Résolution des portées — hors du schéma `identity`, et sans clé étrangère
// -----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ResolvedScope {
    pub label: Value,
    pub hint: Option<String>,
}

/// Le nom des cibles visées par un lot d'attributions.
///
/// Une cible absente de la table rendue est **orpheline** : l'édition a été
/// supprimée, l'attribution lui survit. L'appelant l'affiche comme telle plutôt
/// que de la taire — une portée muette ressemblerait à une portée globale.
pub async fn resolve_scopes(
    pool: &PgPool,
    locale: &str,
    cibles: &[(ScopeType, Uuid)],
) -> Result<HashMap<(ScopeType, Uuid), ResolvedScope>> {
    let mut resolues = HashMap::new();

    let ids = |voulu: ScopeType| -> Vec<Uuid> {
        cibles
            .iter()
            .filter(|(t, _)| *t == voulu)
            .map(|(_, id)| *id)
            .collect()
    };

    let evenements = ids(ScopeType::Event);
    if !evenements.is_empty() {
        let lignes = sqlx::query!(
            r#"SELECT e.id,
                      e.title AS "title!: Value",
                      to_char(e.starts_at AT TIME ZONE e.timezone, 'YYYY-MM-DD')
                        || ' → '
                        || to_char(e.ends_at AT TIME ZONE e.timezone, 'YYYY-MM-DD') AS "dates!"
                 FROM event.events e
                WHERE e.id = ANY($1)"#,
            &evenements
        )
        .fetch_all(pool)
        .await?;

        for ligne in lignes {
            resolues.insert(
                (ScopeType::Event, ligne.id),
                ResolvedScope {
                    label: ligne.title,
                    hint: Some(ligne.dates),
                },
            );
        }
    }

    let organisations = ids(ScopeType::Organization);
    if !organisations.is_empty() {
        // `legal_name` est un texte simple : le rendre sous la forme i18n que le
        // site attend est une conversion de transport, pas une traduction.
        let lignes = sqlx::query!(
            r#"SELECT o.id AS "id!",
                      o.legal_name,
                      COALESCE(platform.t(c.name, $2), o.city) AS "hint?"
                 FROM org.organizations o
                 LEFT JOIN reference.countries c ON c.id = o.country_id
                WHERE o.id = ANY($1)"#,
            &organisations,
            locale
        )
        .fetch_all(pool)
        .await?;

        for ligne in lignes {
            resolues.insert(
                (ScopeType::Organization, ligne.id),
                ResolvedScope {
                    label: serde_json::json!({ "fr": ligne.legal_name }),
                    hint: ligne.hint,
                },
            );
        }
    }

    let espaces = ids(ScopeType::NegotiationSpace);
    if !espaces.is_empty() {
        let lignes = sqlx::query!(
            r#"SELECT s.id, s.name AS "name!: Value"
                 FROM negotiation.spaces s
                WHERE s.id = ANY($1)"#,
            &espaces
        )
        .fetch_all(pool)
        .await?;

        for ligne in lignes {
            resolues.insert(
                (ScopeType::NegotiationSpace, ligne.id),
                ResolvedScope {
                    label: ligne.name,
                    hint: None,
                },
            );
        }
    }

    Ok(resolues)
}

fn portee(valeur: &str) -> Result<ScopeType> {
    ScopeType::from_db(valeur)
        .ok_or_else(|| ApiError::internal(format!("type de portée inconnu : {valeur}")))
}

// -----------------------------------------------------------------------------
// Écritures — attribution et retrait
// -----------------------------------------------------------------------------

/// Pourquoi la base a refusé une attribution.
///
/// **Les deux refus viennent d'elle**, jamais d'une règle recopiée ici : le
/// premier de `ux_role_assignments_active`, le second du trigger
/// `tg_role_assignments_check_scope`, dont le message français est repris tel
/// quel (principe VIII).
#[derive(Debug, Clone)]
pub enum GrantRejection {
    Duplicate,
    ScopeNotAllowed(String),
}

pub struct NewAssignment<'a> {
    pub person_id: PersonId,
    pub role_code: &'a str,
    pub scope_type: ScopeType,
    pub scope_id: Option<Uuid>,
    pub granted_by: Option<PersonId>,
    pub valid_from: Option<OffsetDateTime>,
    pub valid_until: Option<OffsetDateTime>,
    pub note: Option<&'a str>,
}

/// **Un refus avorte la transaction.** PostgreSQL n'accepte plus rien après une
/// erreur, et l'appelant ne peut donc pas composer sa réponse dedans : il
/// abandonne la transaction et relit sur le pool. C'est le prix de laisser la
/// base trancher, et il est juste — vérifier d'abord serait une course.
pub async fn grant(
    conn: &mut PgConnection,
    assignment: NewAssignment<'_>,
) -> Result<std::result::Result<RoleAssignmentId, GrantRejection>> {
    let insertion = sqlx::query_scalar!(
        "INSERT INTO identity.role_assignments
             (person_id, role_code, scope_type, scope_id, granted_by,
              valid_from, valid_until, note)
         VALUES ($1, $2, $3::text::identity.scope_type, $4, $5,
                 COALESCE($6, now()), $7, $8)
         RETURNING id",
        assignment.person_id.as_uuid(),
        assignment.role_code,
        assignment.scope_type.as_str(),
        assignment.scope_id,
        assignment.granted_by.map(PersonId::as_uuid),
        assignment.valid_from,
        assignment.valid_until,
        assignment.note
    )
    .fetch_one(conn)
    .await;

    match insertion {
        Ok(id) => Ok(Ok(RoleAssignmentId(id))),
        Err(e) if kernel::pg_error::constraint(&e) == Some("ux_role_assignments_active") => {
            Ok(Err(GrantRejection::Duplicate))
        }
        Err(e) => match kernel::pg_error::restrict_violation_message(&e) {
            Some(message) => Ok(Err(GrantRejection::ScopeNotAllowed(message.to_owned()))),
            // Fenêtre invalide, portée incohérente, rôle inconnu : le catalogue
            // les nomme, et ce sont de vraies erreurs — 422, pas un discriminant.
            None => Err(kernel::pg_error::translate(&e)),
        },
    }
}

/// L'attribution visée, telle qu'il faut la connaître **avant** d'autoriser :
/// la permission se vérifie sur la portée de l'attribution, pas sur celle de
/// l'acteur.
#[derive(Debug, Clone)]
pub struct AssignmentTarget {
    pub person_id: PersonId,
    pub role_code: String,
    pub scope_type: ScopeType,
    pub scope_id: Option<Uuid>,
    pub revoked_at: Option<OffsetDateTime>,
}

pub async fn find_assignment(
    pool: &PgPool,
    assignment_id: RoleAssignmentId,
) -> Result<Option<AssignmentTarget>> {
    let ligne = sqlx::query!(
        r#"SELECT person_id, role_code, scope_type::text AS "scope_type!", scope_id, revoked_at
             FROM identity.role_assignments
            WHERE id = $1"#,
        assignment_id.as_uuid()
    )
    .fetch_optional(pool)
    .await?;

    ligne
        .map(|l| {
            Ok(AssignmentTarget {
                person_id: PersonId(l.person_id),
                role_code: l.role_code,
                scope_type: portee(&l.scope_type)?,
                scope_id: l.scope_id,
                revoked_at: l.revoked_at,
            })
        })
        .transpose()
}

/// **Le retrait pose trois colonnes, il ne supprime jamais.** La ligne reste :
/// c'est elle qui répond, six mois plus tard, à « pourquoi cette personne
/// n'est-elle plus au comité ? ».
///
/// `WHERE revoked_at IS NULL` fait de la base l'arbitre : deux retraits
/// simultanés n'en écrivent qu'un, et le second lit `false`.
pub async fn revoke(
    conn: &mut PgConnection,
    assignment_id: RoleAssignmentId,
    revoked_by: Option<PersonId>,
    reason: &str,
) -> Result<bool> {
    let touchees = sqlx::query!(
        "UPDATE identity.role_assignments
            SET revoked_at = now(), revoked_by = $2, revoked_reason = $3
          WHERE id = $1 AND revoked_at IS NULL",
        assignment_id.as_uuid(),
        revoked_by.map(PersonId::as_uuid),
        reason
    )
    .execute(conn)
    .await?
    .rows_affected();

    Ok(touchees == 1)
}

// -----------------------------------------------------------------------------
// Les cibles offertes au panneau d'attribution
// -----------------------------------------------------------------------------

/// Les éditions offertes au choix. Elles sont peu nombreuses — une par COP —,
/// et l'écran les affiche toutes.
pub async fn event_choices(pool: &PgPool, locale: &str) -> Result<Vec<ScopeChoice>> {
    let lignes = sqlx::query!(
        r#"SELECT e.id,
                  platform.t(e.title, $1) AS "label!",
                  to_char(e.starts_at AT TIME ZONE e.timezone, 'YYYY-MM-DD') AS "hint!"
             FROM event.events e
            ORDER BY e.starts_at DESC"#,
        locale
    )
    .fetch_all(pool)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| ScopeChoice {
            scope_type: ScopeType::Event,
            scope_id: l.id,
            label: l.label,
            hint: Some(l.hint),
            disabled: false,
        })
        .collect())
}

/// Les organisations offertes au choix.
///
/// Les fusionnées sont écartées : attribuer un référent à une fiche absorbée
/// donnerait un droit sur une organisation qui n'existe plus sous ce nom.
pub async fn organization_choices(pool: &PgPool, locale: &str) -> Result<Vec<ScopeChoice>> {
    let lignes = sqlx::query!(
        r#"SELECT o.id,
                  o.legal_name,
                  COALESCE(platform.t(c.name, $1), o.city) AS "hint?"
             FROM org.organizations o
             LEFT JOIN reference.countries c ON c.id = o.country_id
            WHERE o.status <> 'merged'
            ORDER BY o.legal_name"#,
        locale
    )
    .fetch_all(pool)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| ScopeChoice {
            scope_type: ScopeType::Organization,
            scope_id: l.id,
            label: l.legal_name,
            hint: l.hint,
            disabled: false,
        })
        .collect())
}
