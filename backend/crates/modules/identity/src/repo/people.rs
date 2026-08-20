//! Lectures de `identity.people`.
//!
//! **Aucune mise en minuscules côté service.** `platform.email` est un domaine
//! sur `citext` : la comparaison ignore déjà la casse. Le transtypage
//! `$1::text::citext` n'est pas décoratif — sans lui, PostgreSQL rabat le
//! `citext` sur `text` par sa conversion implicite, et la comparaison redevient
//! sensible à la casse sans que rien ne le dise.

use kernel::error::{ApiError, Result};
use sqlx::postgres::PgConnection;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::ids::{AccountId, PersonId};
use crate::domain::login::PersonStatus;
use crate::domain::person::PersonView;

/// Ce que la connexion a besoin de savoir, en une seule lecture : la personne,
/// et son compte mot de passe s'il existe.
#[derive(Debug, Clone)]
pub struct AuthCandidate {
    pub person_id: PersonId,
    pub email: String,
    pub status: PersonStatus,
    pub email_verified_at: Option<OffsetDateTime>,
    pub suspended_until: Option<OffsetDateTime>,
    pub account_id: Option<AccountId>,
    pub password_hash: Option<String>,
    pub failed_attempts: i16,
    pub locked_until: Option<OffsetDateTime>,
    pub mfa_enabled_at: Option<OffsetDateTime>,
}

pub async fn find_for_login(pool: &PgPool, email: &str) -> Result<Option<AuthCandidate>> {
    let ligne = sqlx::query!(
        r#"SELECT p.id,
                  p.primary_email::text  AS "email!",
                  p.status::text         AS "statut!",
                  p.email_verified_at,
                  p.suspended_until,
                  a.id                   AS "account_id?",
                  a.password_hash        AS "password_hash?",
                  a.failed_attempts      AS "failed_attempts?",
                  a.locked_until         AS "locked_until?",
                  a.mfa_enabled_at       AS "mfa_enabled_at?"
             FROM identity.people p
             LEFT JOIN identity.accounts a
                    ON a.person_id = p.id AND a.provider = 'password'
            WHERE p.primary_email = $1::text::citext"#,
        email
    )
    .fetch_optional(pool)
    .await?;

    ligne
        .map(|l| {
            Ok(AuthCandidate {
                person_id: PersonId(l.id),
                email: l.email,
                status: statut(&l.statut)?,
                email_verified_at: l.email_verified_at,
                suspended_until: l.suspended_until,
                account_id: l.account_id.map(AccountId),
                password_hash: l.password_hash,
                failed_attempts: l.failed_attempts.unwrap_or(0),
                locked_until: l.locked_until,
                mfa_enabled_at: l.mfa_enabled_at,
            })
        })
        .transpose()
}

/// Toutes les personnes, dans l'ordre d'affichage. `display_name` est une
/// colonne générée : le tri se fait en base, pas après coup.
pub async fn list(pool: &PgPool) -> Result<Vec<PersonView>> {
    let lignes = sqlx::query!(
        r#"SELECT p.id,
                  p.primary_email::text AS "primary_email!",
                  p.email_verified_at,
                  p.first_name,
                  p.last_name,
                  p.civility,
                  p.display_name        AS "display_name!",
                  p.phone,
                  p.job_title,
                  p.biography::jsonb    AS "biography?",
                  p.country_id,
                  p.city,
                  p.preferred_locale,
                  p.timezone::text      AS "timezone!",
                  p.primary_organization_id,
                  p.status::text        AS "statut!",
                  p.status_reason,
                  p.status_changed_by,
                  p.status_changed_at,
                  p.suspended_until,
                  p.is_directory_visible,
                  p.created_at,
                  p.updated_at
             FROM identity.people p
            ORDER BY p.display_name"#
    )
    .fetch_all(pool)
    .await?;

    lignes
        .into_iter()
        .map(|l| {
            Ok(PersonView {
                id: PersonId(l.id),
                primary_email: l.primary_email,
                email_verified_at: l.email_verified_at,
                first_name: l.first_name,
                last_name: l.last_name,
                civility: l.civility,
                display_name: l.display_name,
                phone: l.phone,
                job_title: l.job_title,
                biography: l.biography,
                country_id: l.country_id,
                city: l.city,
                preferred_locale: l.preferred_locale,
                timezone: l.timezone,
                primary_organization_id: l.primary_organization_id,
                status: statut(&l.statut)?,
                status_reason: l.status_reason,
                status_changed_by: l.status_changed_by.map(PersonId),
                status_changed_at: l.status_changed_at,
                suspended_until: l.suspended_until,
                is_directory_visible: l.is_directory_visible,
                created_at: l.created_at,
                updated_at: l.updated_at,
            })
        })
        .collect()
}

pub async fn view(pool: &PgPool, person_id: PersonId) -> Result<Option<PersonView>> {
    let ligne = sqlx::query!(
        r#"SELECT p.id,
                  p.primary_email::text AS "primary_email!",
                  p.email_verified_at,
                  p.first_name,
                  p.last_name,
                  p.civility,
                  p.display_name        AS "display_name!",
                  p.phone,
                  p.job_title,
                  p.biography::jsonb    AS "biography?",
                  p.country_id,
                  p.city,
                  p.preferred_locale,
                  p.timezone::text      AS "timezone!",
                  p.primary_organization_id,
                  p.status::text        AS "statut!",
                  p.status_reason,
                  p.status_changed_by,
                  p.status_changed_at,
                  p.suspended_until,
                  p.is_directory_visible,
                  p.created_at,
                  p.updated_at
             FROM identity.people p
            WHERE p.id = $1"#,
        person_id.as_uuid()
    )
    .fetch_optional(pool)
    .await?;

    ligne
        .map(|l| {
            Ok(PersonView {
                id: PersonId(l.id),
                primary_email: l.primary_email,
                email_verified_at: l.email_verified_at,
                first_name: l.first_name,
                last_name: l.last_name,
                civility: l.civility,
                display_name: l.display_name,
                phone: l.phone,
                job_title: l.job_title,
                biography: l.biography,
                country_id: l.country_id,
                city: l.city,
                preferred_locale: l.preferred_locale,
                timezone: l.timezone,
                primary_organization_id: l.primary_organization_id,
                status: statut(&l.statut)?,
                status_reason: l.status_reason,
                status_changed_by: l.status_changed_by.map(PersonId),
                status_changed_at: l.status_changed_at,
                suspended_until: l.suspended_until,
                is_directory_visible: l.is_directory_visible,
                created_at: l.created_at,
                updated_at: l.updated_at,
            })
        })
        .transpose()
}

/// L'énuméré est fermé en base : une valeur inconnue signale que le code et le
/// modèle ont divergé, pas qu'un utilisateur a mal saisi quelque chose.
fn statut(valeur: &str) -> Result<PersonStatus> {
    PersonStatus::from_db(valeur)
        .ok_or_else(|| ApiError::internal(format!("statut de personne inconnu : {valeur}")))
}

/// Ce que l'inscription a besoin de savoir d'une personne déjà connue : de quoi
/// composer un courriel, et rien de plus. Aucun secret, aucun statut de compte.
#[derive(Debug, Clone)]
pub struct RegistrationTarget {
    pub person_id: PersonId,
    pub email: String,
    pub first_name: String,
    pub preferred_locale: String,
    pub email_verified_at: Option<OffsetDateTime>,
}

/// La comparaison d'adresse est faite par la base : `platform.email` est un
/// domaine sur `citext`, et le transtypage explicite empêche PostgreSQL de le
/// rabattre sur `text` — auquel cas la casse redeviendrait significative sans
/// que rien ne le dise.
pub async fn find_by_email(
    conn: &mut PgConnection,
    email: &str,
) -> Result<Option<RegistrationTarget>> {
    let ligne = sqlx::query!(
        r#"SELECT p.id,
                  p.primary_email::text AS "email!",
                  p.first_name,
                  p.preferred_locale,
                  p.email_verified_at
             FROM identity.people p
            WHERE p.primary_email = $1::text::platform.email"#,
        email
    )
    .fetch_optional(conn)
    .await?;

    Ok(ligne.map(|l| RegistrationTarget {
        person_id: PersonId(l.id),
        email: l.email,
        first_name: l.first_name,
        preferred_locale: l.preferred_locale,
        email_verified_at: l.email_verified_at,
    }))
}

pub struct NewPerson<'a> {
    pub first_name: &'a str,
    pub last_name: &'a str,
    pub email: &'a str,
    pub country_id: Option<Uuid>,
    pub preferred_locale: &'a str,
    pub timezone: &'a str,
}

/// Crée la personne. `display_name` est une colonne générée : jamais écrite,
/// seulement relue — d'où le `RETURNING` plutôt qu'une recomposition en Rust.
///
/// `preferred_locale` et `timezone` viennent de l'interface et du navigateur :
/// deux champs de formulaire en moins, deux colonnes `NOT NULL` remplies
/// quand même.
pub async fn create(
    conn: &mut PgConnection,
    personne: NewPerson<'_>,
) -> Result<RegistrationTarget> {
    let ligne = sqlx::query!(
        r#"INSERT INTO identity.people
               (first_name, last_name, primary_email, country_id, preferred_locale, timezone)
           VALUES ($1, $2, $3::text::platform.email, $4, $5, $6::text::platform.timezone_name)
        RETURNING id, primary_email::text AS "email!", first_name, preferred_locale, email_verified_at"#,
        personne.first_name,
        personne.last_name,
        personne.email,
        personne.country_id,
        personne.preferred_locale,
        personne.timezone
    )
    .fetch_one(conn)
    .await?;

    Ok(RegistrationTarget {
        person_id: PersonId(ligne.id),
        email: ligne.email,
        first_name: ligne.first_name,
        preferred_locale: ligne.preferred_locale,
        email_verified_at: ligne.email_verified_at,
    })
}

pub async fn create_password_account(
    conn: &mut PgConnection,
    person_id: PersonId,
    empreinte: &str,
) -> Result<AccountId> {
    let id = sqlx::query_scalar!(
        "INSERT INTO identity.accounts (person_id, provider, password_hash, password_changed_at)
         VALUES ($1, 'password', $2, now())
         RETURNING id",
        person_id.as_uuid(),
        empreinte
    )
    .fetch_one(conn)
    .await?;

    Ok(AccountId(id))
}

/// Pose la date de vérification **si elle ne l'était pas**, et rend l'instant
/// posé. `None` : l'adresse était déjà vérifiée — il n'y a alors aucun
/// changement d'état, donc aucun événement à émettre.
pub async fn mark_email_verified(
    conn: &mut PgConnection,
    person_id: PersonId,
) -> Result<Option<OffsetDateTime>> {
    let instant = sqlx::query_scalar!(
        "UPDATE identity.people
            SET email_verified_at = now()
          WHERE id = $1 AND email_verified_at IS NULL
        RETURNING email_verified_at",
        person_id.as_uuid()
    )
    .fetch_optional(conn)
    .await?;

    Ok(instant.flatten())
}

pub async fn email_of(conn: &mut PgConnection, person_id: PersonId) -> Result<Option<String>> {
    let email = sqlx::query_scalar!(
        r#"SELECT primary_email::text AS "email!" FROM identity.people WHERE id = $1"#,
        person_id.as_uuid()
    )
    .fetch_optional(conn)
    .await?;

    Ok(email)
}
