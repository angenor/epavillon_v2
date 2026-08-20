//! Les écrans d'utilisateurs du back-office.
//!
//! Le périmètre d'administration est un **argument**, jamais une variable
//! d'ambiance : il entre dans la lecture, et la lecture ne sait rien faire sans
//! lui. Les trois cas restent distincts jusqu'au bout — un périmètre vide se
//! refuse à l'entrée (`Perimeter`), il ne produit pas une liste vide.

use contracts::identity as contrats;
use kernel::auth::{self, AdminScope, Scope, ScopeType};
use kernel::context::RequestContext;
use kernel::error::{ApiError, Result};
use kernel::events::{self as evenements, DomainEvent};
use sqlx::PgPool;
use std::collections::BTreeMap;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::admin_users::{
    AssignmentEvent, AssignmentHistoryEntry, RoleAssignmentOptions, UserDetail, UserFacet,
    UserListRow, UserListScreen,
};
use crate::domain::ids::{PersonId, RoleAssignmentId};
use crate::domain::login::PersonStatus;
use crate::domain::permissions;
use crate::domain::rbac::RoleAssignmentView;
use crate::domain::scope::{portee_visee, PersonWriteOutcome, RoleWriteOutcome};
use crate::repo::{admin_users, privacy, rbac as repo_rbac};
use crate::service::{rbac, session};
use crate::state::IdentityState;

pub async fn list_screen(
    pool: &PgPool,
    locale: &str,
    perimetre: &AdminScope,
) -> Result<UserListScreen> {
    let lignes = admin_users::list(pool, perimetre).await?;
    let ids: Vec<_> = lignes.iter().map(|l| l.person_id.as_uuid()).collect();
    let mut attributions = rbac::active_assignments_by_person(pool, locale, &ids).await?;

    let restricted_accounts = lignes
        .iter()
        .filter(|l| matches!(l.status, PersonStatus::Suspended | PersonStatus::Blocked))
        .count();

    let countries = facettes(lignes.iter().filter_map(|l| {
        l.country_id
            .map(|id| (id.to_string(), l.country_name.clone().unwrap_or_default()))
    }));
    let organizations = facettes(lignes.iter().filter_map(|l| {
        l.organization_id.map(|id| {
            (
                id.to_string(),
                serde_json::Value::String(l.organization_name.clone().unwrap_or_default()),
            )
        })
    }));

    let rows = lignes
        .into_iter()
        .map(|l| UserListRow {
            roles: attributions
                .remove(&l.person_id.as_uuid())
                .unwrap_or_default(),
            person_id: l.person_id,
            display_name: l.display_name,
            primary_email: l.primary_email,
            email_verified_at: l.email_verified_at,
            job_title: l.job_title,
            country_name: l.country_name,
            country_id: l.country_id,
            organization_id: l.organization_id,
            organization_name: l.organization_name,
            organization_acronym: l.organization_acronym,
            status: l.status,
            status_reason: l.status_reason,
            suspended_until: l.suspended_until,
            last_login_at: l.last_login_at,
            has_account: l.has_account,
            mfa_enabled: l.mfa_enabled,
            locked_until: l.locked_until,
            open_privacy_request: l.open_privacy_request,
            created_at: l.created_at,
        })
        .collect();

    Ok(UserListScreen {
        rows,
        roles: repo_rbac::assignable_roles(pool).await?,
        countries,
        organizations,
        scoped_to_events: !perimetre.is_global,
        open_privacy_requests: privacy::open_count(pool).await? as usize,
        restricted_accounts,
    })
}

/// La fiche d'une personne. `None` quand elle n'existe pas — et **jamais** pour
/// une personne hors périmètre, qui se lit avec `in_scope` à faux : la taire
/// ferait croire à sa disparition.
pub async fn detail(
    pool: &PgPool,
    locale: &str,
    perimetre: &AdminScope,
    person_id: PersonId,
) -> Result<Option<UserDetail>> {
    let Some(entete) = admin_users::header(pool, person_id).await? else {
        return Ok(None);
    };

    let assignments = rbac::assignments(pool, locale, person_id, true).await?;
    let toutes = rbac::assignments(pool, locale, person_id, false).await?;

    Ok(Some(UserDetail {
        person_id: entete.person_id,
        display_name: entete.display_name,
        first_name: entete.first_name,
        last_name: entete.last_name,
        civility: entete.civility,
        primary_email: entete.primary_email,
        email_verified_at: entete.email_verified_at,
        other_emails: admin_users::other_emails(pool, person_id).await?,
        phone: entete.phone,
        job_title: entete.job_title,
        biography: entete.biography,
        country_id: entete.country_id,
        country_name: entete.country_name,
        city: entete.city,
        preferred_locale: entete.preferred_locale,
        timezone: entete.timezone,
        organization_id: entete.organization_id,
        organization_name: entete.organization_name,
        is_directory_visible: entete.is_directory_visible,
        status: entete.status,
        status_reason: entete.status_reason,
        status_changed_at: entete.status_changed_at,
        status_changed_by_name: entete.status_changed_by_name,
        suspended_until: entete.suspended_until,
        created_at: entete.created_at,
        accounts: admin_users::accounts(pool, person_id).await?,
        assignments,
        history: historique(toutes),
        permissions: rbac::effective_permissions_view(pool, locale, person_id).await?,
        consents: privacy::consents(pool, person_id).await?,
        privacy_requests: privacy::of_person(pool, person_id).await?,
        in_scope: admin_users::in_scope(pool, perimetre, person_id).await?,
    }))
}

/// Deux événements peuvent naître d'une même ligne : l'octroi, puis le retrait.
/// C'est ce qui permet de répondre six mois plus tard à « pourquoi cette
/// personne n'est-elle plus au comité ? » — la ligne n'est jamais supprimée.
fn historique(attributions: Vec<RoleAssignmentView>) -> Vec<AssignmentHistoryEntry> {
    let mut entrees = Vec::new();

    for attribution in attributions {
        let scope = crate::domain::rbac::ScopeRef {
            scope_type: attribution.scope_type,
            scope_id: attribution.scope_id,
            scope_label: attribution.scope_label.clone(),
            scope_hint: attribution.scope_hint.clone(),
            is_dangling: attribution.is_dangling,
        };

        if let (Some(retire_le), reason) = (attribution.revoked_at, attribution.revoked_reason) {
            entrees.push(AssignmentHistoryEntry {
                assignment_id: attribution.id,
                kind: AssignmentEvent::Revoked,
                occurred_at: retire_le,
                role_code: attribution.role_code.clone(),
                role_label: attribution.role_label.clone(),
                scope: scope.clone(),
                actor_name: attribution.revoked_by_name,
                reason,
                valid_until: attribution.valid_until,
            });
        }

        entrees.push(AssignmentHistoryEntry {
            assignment_id: attribution.id,
            kind: AssignmentEvent::Granted,
            occurred_at: attribution.granted_at,
            role_code: attribution.role_code,
            role_label: attribution.role_label,
            scope,
            actor_name: attribution.granted_by_name,
            reason: attribution.note,
            valid_until: attribution.valid_until,
        });
    }

    entrees.sort_by_key(|e| std::cmp::Reverse(e.occurred_at));
    entrees
}

/// Les facettes se comptent sur les lignes **déjà bornées** : compter en base
/// sans le filtre afficherait des effectifs qu'on ne peut pas atteindre.
fn facettes(valeurs: impl Iterator<Item = (String, serde_json::Value)>) -> Vec<UserFacet> {
    let mut par_valeur: BTreeMap<String, (serde_json::Value, usize)> = BTreeMap::new();
    for (value, label) in valeurs {
        let entree = par_valeur.entry(value).or_insert((label, 0));
        entree.1 += 1;
    }

    par_valeur
        .into_iter()
        .map(|(value, (label, count))| UserFacet {
            value,
            label,
            count,
        })
        .collect()
}

// -----------------------------------------------------------------------------
// Écritures de rôle
//
// **La permission se vérifie sur la PORTÉE VISÉE, pas sur celle de l'acteur**
// (FR-046, FR-053). Une administratrice détachée sur la COP31 ne peut attribuer
// que là ; et pour retirer, la portée qui compte est celle de **l'attribution
// visée** — sans quoi elle pourrait défaire un rôle global qu'elle n'aurait
// jamais pu accorder.
//
// **Aucun paramètre par lequel le client déclarerait ses propres droits**
// (FR-055). Le site en passe encore deux à ses fonctions ; ils ne franchissent
// pas le réseau, et s'ils le franchissaient, rien ici ne les lit. La forme des
// charges utiles suffit : serde ignore ce qu'il ne connaît pas, et aucune ne
// déclare de champ de droits.
// -----------------------------------------------------------------------------

/// Attribution d'un rôle.
///
/// `person_id` vient de **l'URL**, jamais du corps : accepter celui du corps
/// laisserait attribuer un rôle à quelqu'un d'autre que la personne dont on
/// regarde la fiche.
pub struct GrantRequest<'a> {
    pub person_id: PersonId,
    pub role_code: &'a str,
    pub scope_type: ScopeType,
    pub scope_id: Option<Uuid>,
    pub valid_from: Option<OffsetDateTime>,
    pub valid_until: Option<OffsetDateTime>,
    pub note: Option<&'a str>,
}

pub async fn grant_role(
    state: &IdentityState,
    ctx: &RequestContext,
    acteur: Uuid,
    demande: GrantRequest<'_>,
) -> Result<RoleWriteOutcome> {
    let pool = state.pool();
    let locale = &ctx.locale;

    // Une portée incohérente sort **avant** l'autorisation : sans cible, il n'y
    // a rien sur quoi tester le droit.
    let portee = portee_visee(demande.scope_type, demande.scope_id)?;

    if !auth::has_permission(pool, acteur, permissions::ROLE_ASSIGN, portee).await? {
        return Ok(RoleWriteOutcome::forbidden_scope());
    }

    if admin_users::header(pool, demande.person_id)
        .await?
        .is_none()
    {
        return Ok(RoleWriteOutcome::not_found());
    }

    let mut tx = state.db().write(&ctx.with_actor(acteur)).await?;
    let ecriture = repo_rbac::grant(
        &mut tx,
        repo_rbac::NewAssignment {
            person_id: demande.person_id,
            role_code: demande.role_code,
            scope_type: demande.scope_type,
            scope_id: demande.scope_id,
            granted_by: Some(PersonId(acteur)),
            valid_from: demande.valid_from,
            valid_until: demande.valid_until,
            note: demande.note,
        },
    )
    .await?;

    let refus = match ecriture {
        Ok(id) => {
            evenements::emit(
                &mut tx,
                DomainEvent {
                    aggregate_schema: contrats::AGGREGATE_SCHEMA,
                    aggregate_type: contrats::AGGREGATE_ROLE_ASSIGNMENT,
                    aggregate_id: id.as_uuid(),
                    event_type: contrats::ROLE_GRANTED,
                    payload: serde_json::to_value(contrats::RoleGranted {
                        person_id: demande.person_id.as_uuid(),
                        role_code: demande.role_code.to_owned(),
                        scope_type: demande.scope_type.as_str().to_owned(),
                        scope_id: demande.scope_id,
                        granted_by: Some(acteur),
                    })
                    .map_err(ApiError::internal)?,
                },
            )
            .await?;
            tx.commit().await?;

            let en_cours = rbac::assignments(pool, locale, demande.person_id, true).await?;
            let posee = en_cours.iter().find(|a| a.id == id).cloned();
            return Ok(match posee {
                Some(vue) => RoleWriteOutcome::granted(vue, en_cours),
                // L'attribution est écrite mais pas encore en vigueur : sa prise
                // d'effet est dans le futur. Elle n'est donc pas dans la liste
                // des attributions en cours, et c'est correct.
                None => RoleWriteOutcome::granted(
                    rbac::assignments(pool, locale, demande.person_id, false)
                        .await?
                        .into_iter()
                        .find(|a| a.id == id)
                        .ok_or_else(|| {
                            ApiError::internal("attribution introuvable après écriture")
                        })?,
                    en_cours,
                ),
            });
        }
        Err(refus) => refus,
    };

    // Un refus de la base a avorté la transaction : plus rien n'y passe. On
    // l'abandonne, et on relit sur le pool pour composer la réponse.
    tx.rollback().await?;
    let en_cours = rbac::assignments(pool, locale, demande.person_id, true).await?;

    Ok(match refus {
        repo_rbac::GrantRejection::Duplicate => {
            // `ux_role_assignments_active` ne filtre que sur `revoked_at IS
            // NULL` : l'attribution en conflit peut être expirée, donc absente
            // des attributions en cours. On la cherche dans toutes.
            let conflit = rbac::assignments(pool, locale, demande.person_id, false)
                .await?
                .into_iter()
                .find(|a| {
                    a.revoked_at.is_none()
                        && a.role_code == demande.role_code
                        && a.scope_type == demande.scope_type
                        && a.scope_id == demande.scope_id
                });
            RoleWriteOutcome::duplicate(conflit, en_cours)
        }
        repo_rbac::GrantRejection::ScopeNotAllowed(message) => {
            RoleWriteOutcome::scope_not_allowed(message, en_cours)
        }
    })
}

/// Retrait d'un rôle. **La ligne n'est jamais supprimée** : `revoked_at`,
/// `revoked_by` et `revoked_reason` s'y ajoutent, et l'historique de la fiche
/// les relit.
pub async fn revoke_role(
    state: &IdentityState,
    ctx: &RequestContext,
    acteur: Uuid,
    assignment_id: RoleAssignmentId,
    reason: &str,
) -> Result<RoleWriteOutcome> {
    let pool = state.pool();
    let locale = &ctx.locale;

    let Some(cible) = repo_rbac::find_assignment(pool, assignment_id).await? else {
        return Ok(RoleWriteOutcome::not_found());
    };

    // **La portée de l'attribution, pas celle de l'acteur.** C'est toute la
    // symétrie : retirer exige le droit qu'il aurait fallu pour accorder.
    let portee = portee_visee(cible.scope_type, cible.scope_id)?;
    if !auth::has_permission(pool, acteur, permissions::ROLE_ASSIGN, portee).await? {
        return Ok(RoleWriteOutcome::forbidden_scope());
    }

    let mut tx = state.db().write(&ctx.with_actor(acteur)).await?;
    if !repo_rbac::revoke(&mut tx, assignment_id, Some(PersonId(acteur)), reason).await? {
        tx.rollback().await?;
        // Déjà retirée — par quelqu'un d'autre, ou deux fois par le même écran.
        // Il n'y a plus d'attribution en cours à cet identifiant, et c'est ce
        // que `not_found` dit.
        return Ok(RoleWriteOutcome::not_found());
    }

    evenements::emit(
        &mut tx,
        DomainEvent {
            aggregate_schema: contrats::AGGREGATE_SCHEMA,
            aggregate_type: contrats::AGGREGATE_ROLE_ASSIGNMENT,
            aggregate_id: assignment_id.as_uuid(),
            event_type: contrats::ROLE_REVOKED,
            payload: serde_json::to_value(contrats::RoleRevoked {
                person_id: cible.person_id.as_uuid(),
                role_code: cible.role_code.clone(),
                scope_type: cible.scope_type.as_str().to_owned(),
                scope_id: cible.scope_id,
                revoked_by: Some(acteur),
                reason: Some(reason.to_owned()),
            })
            .map_err(ApiError::internal)?,
        },
    )
    .await?;
    tx.commit().await?;

    let toutes = rbac::assignments(pool, locale, cible.person_id, false).await?;
    let retiree = toutes
        .iter()
        .find(|a| a.id == assignment_id)
        .cloned()
        .ok_or_else(|| ApiError::internal("attribution introuvable après retrait"))?;
    let en_cours = rbac::assignments(pool, locale, cible.person_id, true).await?;

    Ok(RoleWriteOutcome::revoked(retiree, en_cours))
}

/// Ce que le panneau d'attribution a besoin de savoir — **restreint à ce que
/// l'appelant peut réellement accorder** (FR-057).
///
/// Offrir un rôle ou une cible que l'API refusera ensuite est le pire des deux
/// mondes : l'écran laisse composer une attribution, puis la refuse sans que
/// personne comprenne pourquoi. Ce qui est hors d'atteinte reste **visible et
/// désactivé** : la taire ferait croire à un bogue à qui cherche une édition
/// qu'il sait présente.
pub async fn role_options(
    pool: &PgPool,
    locale: &str,
    acteur: Uuid,
) -> Result<RoleAssignmentOptions> {
    let can_assign_global =
        auth::has_permission(pool, acteur, permissions::ROLE_ASSIGN, Scope::Global).await?;

    // Les portées ciblées où l'acteur détient réellement le droit. Une
    // attribution globale les couvre toutes — `has_permission` le dit déjà —,
    // d'où la lecture des portées seulement quand elle est nécessaire.
    let detenues = repo_rbac::effective_permissions(pool, PersonId(acteur)).await?;
    let portees_de = |voulu: ScopeType| -> Vec<Uuid> {
        detenues
            .iter()
            .filter(|p| p.permission_code == permissions::ROLE_ASSIGN && p.scope_type == voulu)
            .filter_map(|p| p.scope_id)
            .collect()
    };

    let evenements_permis = portees_de(ScopeType::Event);
    let organisations_permises = portees_de(ScopeType::Organization);

    let mut events = repo_rbac::event_choices(pool, locale).await?;
    for choix in &mut events {
        choix.disabled = !can_assign_global && !evenements_permis.contains(&choix.scope_id);
    }

    let mut organizations = repo_rbac::organization_choices(pool, locale).await?;
    for choix in &mut organizations {
        choix.disabled = !can_assign_global && !organisations_permises.contains(&choix.scope_id);
    }

    // Un rôle dont **aucune** portée autorisée n'est atteignable ne sert à rien
    // dans ce panneau : le proposer ferait cliquer pour rien.
    let atteignables = |allowed: &[ScopeType]| -> bool {
        allowed.iter().any(|portee| match portee {
            ScopeType::Global => can_assign_global,
            ScopeType::Event => can_assign_global || !evenements_permis.is_empty(),
            ScopeType::Organization => can_assign_global || !organisations_permises.is_empty(),
            ScopeType::NegotiationSpace => {
                can_assign_global || !portees_de(ScopeType::NegotiationSpace).is_empty()
            }
        })
    };

    let roles = repo_rbac::assignable_roles(pool)
        .await?
        .into_iter()
        .filter(|r| atteignables(&r.allowed_scopes))
        .collect();

    Ok(RoleAssignmentOptions {
        roles,
        events,
        organizations,
        // Le module Négociations n'a pas d'écran dans ce jalon : offrir un choix
        // sans cible vaut moins qu'un choix vide et expliqué.
        negotiation_spaces: Vec::new(),
        can_assign_global,
        grantable_event_ids: if can_assign_global {
            Vec::new()
        } else {
            evenements_permis
        },
    })
}

// -----------------------------------------------------------------------------
// Changement de statut
// -----------------------------------------------------------------------------

pub struct StatusRequest<'a> {
    pub person_id: PersonId,
    pub status: PersonStatus,
    pub reason: &'a str,
    pub suspended_until: Option<OffsetDateTime>,
    pub revoke_sessions: bool,
}

/// Suspendre, exclure, rétablir.
///
/// **Les sessions ouvertes cessent de valoir sans attendre cette écriture**
/// (FR-033) : la résolution de session teste le statut à chaque requête, et une
/// personne suspendue n'est plus résolue. `revoke_sessions` ferme les lignes
/// **en plus**, avec leur motif — c'est ce qui rend la coupure visible dans la
/// fiche plutôt que seulement effective.
pub async fn set_status(
    state: &IdentityState,
    ctx: &RequestContext,
    acteur: Uuid,
    perimetre: &AdminScope,
    demande: StatusRequest<'_>,
) -> Result<PersonWriteOutcome> {
    let pool = state.pool();

    let mut tx = state.db().write(&ctx.with_actor(acteur)).await?;
    let Some(avant) = admin_users::lock_status(&mut tx, demande.person_id).await? else {
        tx.rollback().await?;
        return Ok(PersonWriteOutcome::not_found());
    };

    // Une personne anonymisée n'a plus d'identité à administrer : son nom, son
    // adresse et ses comptes ont été purgés par `identity.anonymize_person()`.
    // Lui reposer un statut ferait revivre une fiche que le RGPD a fermée.
    if avant == PersonStatus::Anonymized {
        tx.rollback().await?;
        return Ok(PersonWriteOutcome::not_found());
    }

    let ecrite = admin_users::set_status(
        &mut tx,
        demande.person_id,
        demande.status,
        demande.reason,
        Some(PersonId(acteur)),
        demande.suspended_until,
    )
    .await?;

    if !ecrite {
        // Une suspension sans terme : la base l'a refusée, la transaction est
        // avortée, et la fiche se relit sur le pool pour que l'écran se recale.
        tx.rollback().await?;
        let fiche = detail(pool, &ctx.locale, perimetre, demande.person_id).await?;
        return Ok(PersonWriteOutcome::missing_deadline(fiche));
    }

    if demande.revoke_sessions {
        session::cut_on_status_change(&mut tx, demande.person_id).await?;
    }

    if avant != demande.status {
        evenements::emit(
            &mut tx,
            DomainEvent {
                aggregate_schema: contrats::AGGREGATE_SCHEMA,
                aggregate_type: contrats::AGGREGATE_PERSON,
                aggregate_id: demande.person_id.as_uuid(),
                event_type: contrats::PERSON_STATUS_CHANGED,
                payload: serde_json::to_value(contrats::PersonStatusChanged {
                    person_id: demande.person_id.as_uuid(),
                    previous_status: avant.as_db().to_owned(),
                    new_status: demande.status.as_db().to_owned(),
                    suspended_until: demande.suspended_until,
                    reason: Some(demande.reason.to_owned()),
                })
                .map_err(ApiError::internal)?,
            },
        )
        .await?;
    }

    tx.commit().await?;

    let fiche = detail(pool, &ctx.locale, perimetre, demande.person_id)
        .await?
        .ok_or_else(|| ApiError::internal("fiche introuvable après écriture"))?;
    Ok(PersonWriteOutcome::saved(fiche))
}
