//! Lectures des écrans d'utilisateurs du back-office.
//!
//! **Le filtre de périmètre est porté par le SQL, pas par le code appelant.**
//! Une liste qu'on filtrerait après coup laisserait passer le jour où l'on
//! oublie l'appel — et c'est exactement ce que la règle métier n° 8 interdit.
//!
//! Une personne n'appartient à aucune édition : le rattachement se lit par
//! l'autre bout, celui des attributions de rôle portant sur les éditions
//! administrées. Tant que les modules `event` et `programme` n'ont pas d'écran,
//! c'est le seul lien qui existe entre une personne et une édition — les autres
//! (propositions, sessions, inscriptions) s'ajouteront à ce filtre, sans le
//! remplacer.

use kernel::auth::AdminScope;
use kernel::error::{ApiError, Result};
use serde_json::Value;
use sqlx::postgres::PgConnection;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::admin_users::{AccountView, PersonEmailView, PrivacyRequestType};
use crate::domain::ids::{AccountId, PersonId};
use crate::domain::login::PersonStatus;

/// Une ligne de la liste, sans ses attributions : elles se lisent en un seul
/// coup pour toute la page.
#[derive(Debug, Clone)]
pub struct UserRow {
    pub person_id: PersonId,
    pub display_name: String,
    pub primary_email: String,
    pub email_verified_at: Option<OffsetDateTime>,
    pub job_title: Option<String>,
    pub country_name: Option<Value>,
    pub country_id: Option<Uuid>,
    pub organization_id: Option<Uuid>,
    pub organization_name: Option<String>,
    pub organization_acronym: Option<String>,
    pub status: PersonStatus,
    pub status_reason: Option<String>,
    pub suspended_until: Option<OffsetDateTime>,
    pub last_login_at: Option<OffsetDateTime>,
    pub has_account: bool,
    pub mfa_enabled: bool,
    pub locked_until: Option<OffsetDateTime>,
    pub open_privacy_request: Option<PrivacyRequestType>,
    pub created_at: OffsetDateTime,
}

pub async fn list(pool: &PgPool, perimetre: &AdminScope) -> Result<Vec<UserRow>> {
    let lignes = sqlx::query!(
        r#"SELECT p.id,
                  p.display_name        AS "display_name!",
                  p.primary_email::text AS "primary_email!",
                  p.email_verified_at,
                  p.job_title,
                  c.name                AS "country_name?: Value",
                  p.country_id,
                  p.primary_organization_id,
                  o.legal_name          AS "organization_name?",
                  o.acronym             AS "organization_acronym?",
                  p.status::text        AS "statut!",
                  p.status_reason,
                  p.suspended_until,
                  p.created_at,
                  (SELECT max(a.last_login_at) FROM identity.accounts a
                    WHERE a.person_id = p.id) AS "last_login_at?",
                  EXISTS (SELECT 1 FROM identity.accounts a
                           WHERE a.person_id = p.id) AS "has_account!",
                  EXISTS (SELECT 1 FROM identity.accounts a
                           WHERE a.person_id = p.id
                             AND a.mfa_enabled_at IS NOT NULL) AS "mfa_enabled!",
                  (SELECT max(a.locked_until) FROM identity.accounts a
                    WHERE a.person_id = p.id) AS "locked_until?",
                  (SELECT pr.request_type::text FROM identity.privacy_requests pr
                    WHERE pr.person_id = p.id
                      AND pr.status IN ('received', 'in_progress')
                    ORDER BY pr.due_at LIMIT 1) AS "open_privacy_request?"
             FROM identity.people p
             LEFT JOIN reference.countries c ON c.id = p.country_id
             LEFT JOIN org.organizations o ON o.id = p.primary_organization_id
            WHERE $1 OR EXISTS (
                    SELECT 1 FROM identity.role_assignments ra
                     WHERE ra.person_id = p.id
                       AND ra.revoked_at IS NULL
                       AND ra.scope_type = 'event'
                       AND ra.scope_id = ANY($2))
            ORDER BY p.display_name"#,
        perimetre.is_global,
        &perimetre.event_ids
    )
    .fetch_all(pool)
    .await?;

    lignes
        .into_iter()
        .map(|l| {
            Ok(UserRow {
                person_id: PersonId(l.id),
                display_name: l.display_name,
                primary_email: l.primary_email,
                email_verified_at: l.email_verified_at,
                job_title: l.job_title,
                country_name: l.country_name,
                country_id: l.country_id,
                organization_id: l.primary_organization_id,
                organization_name: l.organization_name,
                organization_acronym: l.organization_acronym,
                status: statut(&l.statut)?,
                status_reason: l.status_reason,
                suspended_until: l.suspended_until,
                last_login_at: l.last_login_at,
                has_account: l.has_account,
                mfa_enabled: l.mfa_enabled,
                locked_until: l.locked_until,
                open_privacy_request: l
                    .open_privacy_request
                    .as_deref()
                    .map(crate::repo::privacy::finalite)
                    .transpose()?,
                created_at: l.created_at,
            })
        })
        .collect()
}

/// La personne relève-t-elle du périmètre de l'appelant ?
///
/// La réponse ne conditionne pas l'accès à la fiche — hors périmètre, elle se
/// lit quand même — mais l'écriture, qui viendra s'y adosser.
pub async fn in_scope(pool: &PgPool, perimetre: &AdminScope, person_id: PersonId) -> Result<bool> {
    if perimetre.is_global {
        return Ok(true);
    }

    let dedans = sqlx::query_scalar!(
        r#"SELECT EXISTS (
               SELECT 1 FROM identity.role_assignments ra
                WHERE ra.person_id = $1
                  AND ra.revoked_at IS NULL
                  AND ra.scope_type = 'event'
                  AND ra.scope_id = ANY($2)
           ) AS "dedans!""#,
        person_id.as_uuid(),
        &perimetre.event_ids
    )
    .fetch_one(pool)
    .await?;

    Ok(dedans)
}

/// L'en-tête de la fiche : ce que `PersonView` ne porte pas — le pays et
/// l'organisation résolus, l'auteur du dernier changement de statut.
#[derive(Debug, Clone)]
pub struct PersonHeader {
    pub person_id: PersonId,
    pub display_name: String,
    pub first_name: String,
    pub last_name: String,
    pub civility: Option<String>,
    pub primary_email: String,
    pub email_verified_at: Option<OffsetDateTime>,
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
    pub status_changed_at: Option<OffsetDateTime>,
    pub status_changed_by_name: Option<String>,
    pub suspended_until: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
}

pub async fn header(pool: &PgPool, person_id: PersonId) -> Result<Option<PersonHeader>> {
    let ligne = sqlx::query!(
        r#"SELECT p.id,
                  p.display_name        AS "display_name!",
                  p.first_name,
                  p.last_name,
                  p.civility,
                  p.primary_email::text AS "primary_email!",
                  p.email_verified_at,
                  p.phone,
                  p.job_title,
                  p.biography::jsonb    AS "biography?",
                  p.country_id,
                  c.name                AS "country_name?: Value",
                  p.city,
                  p.preferred_locale,
                  p.timezone::text      AS "timezone!",
                  p.primary_organization_id,
                  o.legal_name          AS "organization_name?",
                  p.is_directory_visible,
                  p.status::text        AS "statut!",
                  p.status_reason,
                  p.status_changed_at,
                  s.display_name        AS "status_changed_by_name?",
                  p.suspended_until,
                  p.created_at
             FROM identity.people p
             LEFT JOIN reference.countries c ON c.id = p.country_id
             LEFT JOIN org.organizations o ON o.id = p.primary_organization_id
             LEFT JOIN identity.people s ON s.id = p.status_changed_by
            WHERE p.id = $1"#,
        person_id.as_uuid()
    )
    .fetch_optional(pool)
    .await?;

    ligne
        .map(|l| {
            Ok(PersonHeader {
                person_id: PersonId(l.id),
                display_name: l.display_name,
                first_name: l.first_name,
                last_name: l.last_name,
                civility: l.civility,
                primary_email: l.primary_email,
                email_verified_at: l.email_verified_at,
                phone: l.phone,
                job_title: l.job_title,
                biography: l.biography,
                country_id: l.country_id,
                country_name: l.country_name,
                city: l.city,
                preferred_locale: l.preferred_locale,
                timezone: l.timezone,
                organization_id: l.primary_organization_id,
                organization_name: l.organization_name,
                is_directory_visible: l.is_directory_visible,
                status: statut(&l.statut)?,
                status_reason: l.status_reason,
                status_changed_at: l.status_changed_at,
                status_changed_by_name: l.status_changed_by_name,
                suspended_until: l.suspended_until,
                created_at: l.created_at,
            })
        })
        .transpose()
}

/// Les comptes de connexion, **secrets exclus** : ni empreinte, ni secret de
/// second facteur, ni codes de secours ne franchissent cette lecture.
pub async fn accounts(pool: &PgPool, person_id: PersonId) -> Result<Vec<AccountView>> {
    let lignes = sqlx::query!(
        r#"SELECT id,
                  provider::text AS "provider!",
                  last_login_at,
                  password_changed_at,
                  mfa_enabled_at,
                  failed_attempts,
                  locked_until,
                  created_at
             FROM identity.accounts
            WHERE person_id = $1
            ORDER BY created_at"#,
        person_id.as_uuid()
    )
    .fetch_all(pool)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| AccountView {
            id: AccountId(l.id),
            provider: l.provider,
            last_login_at: l.last_login_at,
            password_changed_at: l.password_changed_at,
            mfa_enabled_at: l.mfa_enabled_at,
            failed_attempts: l.failed_attempts,
            locked_until: l.locked_until,
            created_at: l.created_at,
        })
        .collect())
}

pub async fn other_emails(pool: &PgPool, person_id: PersonId) -> Result<Vec<PersonEmailView>> {
    let lignes = sqlx::query!(
        r#"SELECT email::text AS "email!", label, verified_at
             FROM identity.person_emails
            WHERE person_id = $1
            ORDER BY created_at"#,
        person_id.as_uuid()
    )
    .fetch_all(pool)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| PersonEmailView {
            email: l.email,
            label: l.label,
            verified_at: l.verified_at,
        })
        .collect())
}

fn statut(valeur: &str) -> Result<PersonStatus> {
    PersonStatus::from_db(valeur)
        .ok_or_else(|| ApiError::internal(format!("statut de personne inconnu : {valeur}")))
}

/// Le statut courant, **verrouillé** : deux changements simultanés se
/// sérialisent, et l'événement annonce l'état d'avant qui a réellement précédé
/// celui d'après.
///
/// `None` quand la personne n'existe pas.
pub async fn lock_status(
    conn: &mut PgConnection,
    person_id: PersonId,
) -> Result<Option<PersonStatus>> {
    let ligne = sqlx::query_scalar!(
        r#"SELECT status::text AS "status!" FROM identity.people WHERE id = $1 FOR UPDATE"#,
        person_id.as_uuid()
    )
    .fetch_optional(conn)
    .await?;

    ligne
        .map(|s| {
            PersonStatus::from_db(&s)
                .ok_or_else(|| ApiError::internal(format!("statut de personne inconnu : {s}")))
        })
        .transpose()
}

/// Pose le statut, son motif et son auteur.
///
/// `suspended_until` est écrite **quel que soit le statut** : réactiver
/// quelqu'un sans effacer son terme laisserait une date de fin de suspension
/// sur un compte actif, que toute lecture affichant « suspendu jusqu'au… »
/// reprendrait telle quelle.
///
/// `ck_people_suspension_window` refuse une suspension sans terme, et c'est la
/// base qui le dit — le service traduit, il ne redouble pas.
/// Rend **faux** quand la base refuse une suspension sans terme, et l'erreur
/// est alors la seule attendue : le nom de la contrainte le dit. Tout autre
/// échec reste une erreur. Un refus avorte la transaction — l'appelant
/// l'abandonne et relit sur le pool.
pub async fn set_status(
    conn: &mut PgConnection,
    person_id: PersonId,
    statut: PersonStatus,
    reason: &str,
    actor_id: Option<PersonId>,
    suspended_until: Option<OffsetDateTime>,
) -> Result<bool> {
    let ecriture = sqlx::query!(
        "UPDATE identity.people
            SET status = $2::text::identity.person_status,
                status_reason = $3,
                status_changed_by = $4,
                status_changed_at = now(),
                suspended_until = $5
          WHERE id = $1",
        person_id.as_uuid(),
        statut.as_db(),
        reason,
        actor_id.map(PersonId::as_uuid),
        suspended_until
    )
    .execute(conn)
    .await;

    match ecriture {
        Ok(_) => Ok(true),
        Err(e) if kernel::pg_error::constraint(&e) == Some("ck_people_suspension_window") => {
            Ok(false)
        }
        Err(e) => Err(kernel::pg_error::translate(&e)),
    }
}
