//! Traduction `(SQLSTATE, nom de contrainte) → erreur d'API`.
//!
//! Principe VIII : le code ne redouble pas une contrainte de la base, il
//! traduit son refus. Le catalogue vit dans
//! `specs/001-socle-identite/contracts/errors.md`.
//!
//! Ce qui n'est pas répertorié sort en `INTERNAL` : le texte brut d'une erreur
//! PostgreSQL porte des noms de tables, parfois des valeurs.

use crate::error::{ApiError, ErrorCode};

/// Nom de la contrainte violée, quand la base le donne.
pub fn constraint(err: &sqlx::Error) -> Option<&str> {
    match err {
        sqlx::Error::Database(db) => db.constraint(),
        _ => None,
    }
}

pub fn sqlstate(err: &sqlx::Error) -> Option<String> {
    match err {
        sqlx::Error::Database(db) => db.code().map(|c| c.into_owned()),
        _ => None,
    }
}

/// Message français levé par un trigger du modèle, à reprendre **tel quel**.
/// Le reformuler produirait deux libellés pour un même refus, et le second se
/// périmerait à la première évolution du SQL.
pub fn restrict_violation_message(err: &sqlx::Error) -> Option<&str> {
    match err {
        sqlx::Error::Database(db) if db.code().as_deref() == Some("23001") => Some(db.message()),
        _ => None,
    }
}

/// Vrai quand l'échec est une collision d'aléa sur une empreinte de jeton :
/// l'appelant régénère et rejoue une fois avant d'abandonner.
pub fn is_token_hash_collision(err: &sqlx::Error) -> bool {
    matches!(
        constraint(err),
        Some("sessions_refresh_token_hash_key") | Some("one_time_tokens_token_hash_key")
    )
}

pub fn translate(err: &sqlx::Error) -> ApiError {
    match err {
        sqlx::Error::RowNotFound => ApiError::not_found(),
        sqlx::Error::PoolTimedOut | sqlx::Error::PoolClosed => {
            ApiError::new(ErrorCode::ServiceUnavailable).detail(err)
        }
        sqlx::Error::Database(db) => {
            let code = db.code().unwrap_or_default().into_owned();
            let contrainte = db.constraint().unwrap_or_default();
            translate_database(&code, contrainte, db.message()).detail(err)
        }
        other => ApiError::internal(other),
    }
}

fn translate_database(sqlstate: &str, contrainte: &str, message: &str) -> ApiError {
    use ErrorCode::*;

    match (sqlstate, contrainte) {
        ("23505", "ux_people_primary_email") => {
            ApiError::new(IdentityEmailAlreadyUsed).field("primary_email")
        }
        ("23505", "ux_person_emails") => ApiError::new(IdentityEmailAlreadyUsed).field("email"),
        ("23505", "ux_accounts_password_per_person") => ApiError::new(IdentityAccountAlreadyExists),
        ("23505", "ux_accounts_provider_subject") => ApiError::new(Conflict),
        ("23505", _) => ApiError::new(Conflict),

        ("23514", "ck_role_assignment_window") => {
            ApiError::new(IdentityRoleWindowInvalid).field("valid_until")
        }
        ("23514", "ck_role_assignment_scope") => {
            ApiError::new(IdentityRoleScopeMismatch).field("scope_id")
        }
        ("23514", "ck_role_assignment_revocation") => ApiError::new(IdentityRoleRevocationInvalid),
        ("23514", "people_first_name_check") => ApiError::new(ValidationFailed).field("first_name"),
        ("23514", "people_last_name_check") => ApiError::new(ValidationFailed).field("last_name"),

        // Un domaine à CHECK lève 23514, jamais 22P02— mesuré sur la base. Le
        // refus ne porte NI table NI colonne : seuls le schéma, le domaine et
        // la contrainte. Le champ fautif est donc ajouté par le module, qui
        // seul sait d'où venait la valeur.
        ("23514", "email_check")
        | ("23514", "timezone_name_check")
        | ("23514", "url_check")
        | ("23514", "slug_check") => ApiError::new(ValidationFailed),

        // Défauts de code, pas données de l'utilisateur : un mot de passe mal
        // formé, un type d'événement à deux segments, un texte multilingue mal
        // bâti, une civilité ou un libellé hors d'une liste que l'interface
        // choisit elle-même.
        ("23514", "ck_accounts_password_shape")
        | ("23514", "ck_outbox_event_type_format")
        | ("23514", "i18n_text_check")
        | ("23514", "people_civility_check")
        | ("23514", "person_emails_label_check") => ApiError::new(Internal),

        ("23503", c) if c.contains("country_id") || c.contains("preferred_locale") => {
            let champ = if c.contains("country_id") {
                "country_id"
            } else {
                "preferred_locale"
            };
            ApiError::new(IdentityUnknownReference).field(champ)
        }

        // Conversion impossible : uuid, date ou valeur d'énumération mal formée.
        // Le domaine `platform.email` ne passe PAS par là — `citext` accepte
        // toute chaîne, et c'est son CHECK qui refuse, en 23514.
        ("22P02", _) => ApiError::new(ValidationFailed),

        // Trigger du modèle : le message français existe déjà, on le rend.
        ("23001", _) => ApiError::with_message(ValidationFailed, message),

        _ => ApiError::new(Internal),
    }
}
