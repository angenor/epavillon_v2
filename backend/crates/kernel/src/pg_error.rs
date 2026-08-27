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

/// Nom du **type** mis en cause, quand la base le donne (`PG_DIAG_DATATYPE_NAME`).
///
/// C'est la seule information fiable sur une violation de DOMAINE : le nom de
/// contrainte y est celui du domaine — `slug_check` pour `platform.slug` — et ne
/// dit ni la table ni la colonne. Deux champs d'une même charge utile portant le
/// même domaine seraient indiscernables sans lui.
pub fn data_type(err: &sqlx::Error) -> Option<&str> {
    match err {
        sqlx::Error::Database(db) => db
            .try_downcast_ref::<sqlx::postgres::PgDatabaseError>()
            .and_then(|pg| pg.data_type()),
        _ => None,
    }
}

/// Le domaine qu'une violation met en cause — `timezone_name`, `slug`, `url`,
/// `email`. C'est ce dont un module se sert pour poser le champ fautif, que lui
/// seul connaît.
///
/// **Le nom vient nu, sans son schéma** : PostgreSQL envoie le schéma dans un
/// champ à part (`PG_DIAG_SCHEMA_NAME`). Mesuré sur la base plutôt que supposé —
/// `SELECT 'Mauvais Slug'::platform.slug` rend « DATATYPE NAME: slug ». Le
/// dernier segment est pris malgré tout, pour que la valeur reste juste si une
/// version future qualifiait le nom.
pub fn violated_domain(err: &sqlx::Error) -> Option<&str> {
    let nom = data_type(err)?;
    Some(nom.rsplit('.').next().unwrap_or(nom))
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

        // --- Événements (B3) : unicités ---------------------------------------
        // La quasi-totalité de ces refus est reprise par le service, qui les
        // rend en 200 sous la forme du contrat du front. Ce qu'on écrit ici est
        // la réponse quand un refus ÉCHAPPE à ce chemin : elle nomme le champ
        // plutôt que de rendre un conflit anonyme.
        ("23505", "ux_events_slug") => ApiError::new(Conflict).field("slug"),
        ("23505", "ux_events_series_edition") => ApiError::new(Conflict).field("edition_label"),
        ("23505", "ux_event_days_slug") | ("23505", "ux_programme_tracks_slug") => {
            ApiError::new(Conflict).field("slug")
        }
        ("23505", "ux_programme_tracks_code")
        | ("23505", "ux_rooms_code")
        | ("23505", "ux_broadcast_channels_code")
        | ("23505", "ux_calls_code")
        | ("23505", "ux_review_criteria") => ApiError::new(Conflict).field("code"),
        ("23505", "ux_calls_one_per_event") => ApiError::new(Conflict),

        // Ces trois-là NE DOIVENT JAMAIS remonter, et le dire ici est le seul
        // moyen de s'en apercevoir : la génération du calendrier ne crée que
        // les dates absentes, calculées dans la même transaction (R4) ; le
        // canal par défaut se retire AVANT d'être posé (R6) ; la composition du
        // comité est dédoublonnée par le service. Les voir signifie que l'ordre
        // a été inversé — un défaut de code, pas une donnée de l'utilisateur.
        ("23505", "ux_event_days_date")
        | ("23505", "ux_broadcast_channels_default")
        | ("23505", "call_reviewers_pkey") => ApiError::new(Internal),

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

        // --- Événements (B3) : vérifications ----------------------------------
        ("23514", "ck_events_period") => ApiError::new(ValidationFailed).field("ends_at"),
        ("23514", "ck_events_coordinates") => ApiError::new(ValidationFailed).field("latitude"),
        ("23514", "events_latitude_check") => ApiError::new(ValidationFailed).field("latitude"),
        ("23514", "events_longitude_check") => ApiError::new(ValidationFailed).field("longitude"),
        ("23514", "ck_events_physical_location") => {
            ApiError::new(ValidationFailed).field("country_id")
        }
        ("23514", "events_edition_year_check") => {
            ApiError::new(ValidationFailed).field("edition_year")
        }
        ("23514", "ck_programme_tracks_period") => ApiError::new(ValidationFailed).field("ends_on"),
        ("23514", "rooms_capacity_check") => ApiError::new(ValidationFailed).field("capacity"),
        ("23514", "ck_calls_window") => ApiError::new(ValidationFailed).field("closes_at"),
        ("23514", "ck_calls_extension") => ApiError::new(ValidationFailed).field("extended_until"),
        ("23514", "ck_calls_speakers") => ApiError::new(ValidationFailed).field("max_speakers"),
        // Une contrainte, TROIS conditions : borne basse, borne haute et durée
        // par défaut. Le champ nommé ici est le cas courant ; c'est au service
        // de désigner plus finement, en comparant les trois valeurs — sans
        // jamais réimplémenter la vérification.
        ("23514", "ck_calls_duration_bounds") => {
            ApiError::new(ValidationFailed).field("default_duration_minutes")
        }
        ("23514", "ck_calls_daily_window") => ApiError::new(ValidationFailed).field("daily_end_time"),
        ("23514", "calls_for_proposals_required_reviews_check") => {
            ApiError::new(ValidationFailed).field("required_reviews")
        }
        ("23514", "calls_for_proposals_max_proposals_per_organization_check") => {
            ApiError::new(ValidationFailed).field("max_proposals_per_organization")
        }
        ("23514", "review_criteria_max_score_check") => {
            ApiError::new(ValidationFailed).field("max_score")
        }
        ("23514", "review_criteria_weight_check") => ApiError::new(ValidationFailed).field("weight"),

        // Forme d'un code, et forme d'une couleur : le message précise ce qui
        // était attendu, sinon l'écran ne peut rien dire d'utile.
        ("23514", "programme_tracks_code_check")
        | ("23514", "broadcast_channels_code_check")
        | ("23514", "calls_for_proposals_code_check")
        | ("23514", "review_criteria_code_check") => ApiError::with_message(
            ValidationFailed,
            "Le code doit commencer par une lettre minuscule et ne contenir que des lettres, des chiffres ou des tirets bas.",
        )
        .field("code"),
        ("23514", "event_days_color_hex_check") | ("23514", "programme_tracks_color_hex_check") => {
            ApiError::with_message(ValidationFailed, "La couleur doit s'écrire sous la forme #0a1b2c.")
                .field("color_hex")
        }

        // Vocabulaires fermés du modèle : une valeur hors liste n'est pas une
        // faute de saisie, c'est une référence inconnue.
        ("23514", "venues_kind_check") => ApiError::new(EventUnknownReference).field("kind"),
        ("23514", "broadcast_channels_provider_check") => {
            ApiError::new(EventUnknownReference).field("provider")
        }

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

        // --- Événements (B3) : clés étrangères --------------------------------
        // **Déclarées avant la garde par sous-chaîne ci-dessous**, qui prendrait
        // `events_country_id_fkey` pour une référence du module Identité.
        ("23503", "events_series_id_fkey") => {
            ApiError::new(EventUnknownReference).field("series_id")
        }
        ("23503", "events_country_id_fkey") => {
            ApiError::new(EventUnknownReference).field("country_id")
        }
        ("23503", "broadcast_channels_locale_fkey") => {
            ApiError::new(EventUnknownReference).field("locale")
        }
        ("23503", "xmod_fk_programme_tracks_curator") => {
            ApiError::new(EventUnknownReference).field("curated_by")
        }
        ("23503", "xmod_fk_call_reviewers_person") => {
            ApiError::new(EventUnknownReference).field("person_id")
        }
        // L'acteur vient de la session : s'il n'existe pas, ce n'est pas la
        // charge utile qui est en cause.
        ("23503", "xmod_fk_events_creator") | ("23503", "xmod_fk_calls_creator") => {
            ApiError::new(Internal)
        }

        ("23503", c) if c.contains("country_id") || c.contains("preferred_locale") => {
            let champ = if c.contains("country_id") {
                "country_id"
            } else {
                "preferred_locale"
            };
            ApiError::new(IdentityUnknownReference).field(champ)
        }

        // --- Direct (B9) : les trois contraintes de `live.incidents` ----------
        // Les deux premières sont doublées par une validation en amont, qui rend
        // l'issue que le formulaire pose sur son champ. Elles ne remontent donc
        // qu'à un refus qui ÉCHAPPE au chemin nominal — écriture concurrente,
        // donnée reprise, chemin ajouté plus tard sans validation.
        ("23514", "ck_incidents_scope_target") => {
            ApiError::new(LiveIncidentScopeTargetMismatch).field("scope")
        }
        ("23514", "ck_incidents_window") => {
            ApiError::new(LiveIncidentWindowInvalid).field("display_until")
        }
        // **CELLE-CI NE DOIT JAMAIS REMONTER**, et la déclarer est le seul
        // moyen de s'apercevoir qu'elle l'a fait. `live.unpublish_incident()`
        // exige déjà `published_at IS NOT NULL` : la contrainte est
        // inatteignable par les fonctions du modèle. Si elle répond, c'est
        // qu'une écriture les a contournées — même parti que les trois
        // « défauts de code » ci-dessus.
        ("23514", "ck_incidents_unpublish_shape") => ApiError::new(Conflict),

        // Conversion impossible : uuid, date ou valeur d'énumération mal formée.
        // Le domaine `platform.email` ne passe PAS par là — `citext` accepte
        // toute chaîne, et c'est son CHECK qui refuse, en 23514.
        ("22P02", _) => ApiError::new(ValidationFailed),

        // Trigger du modèle : le message français existe déjà, on le rend.
        ("23001", _) => ApiError::with_message(ValidationFailed, message),

        // **Un `check_violation` SANS nom de contrainte** ne peut venir que
        // d'un `RAISE … USING ERRCODE = 'check_violation'` dans un déclencheur :
        // une vraie contrainte `CHECK` porte toujours son nom. Le message est
        // alors écrit en français par le modèle, pour être lu — `Note 6.00
        // supérieure au maximum autorisé (5.00) pour ce critère.` —, et le
        // repli anonyme le transformait en 500 (écart n° 106, relevé en B4 :
        // le contrat annonçait un 422, la mesure a rendu un 500).
        ("23514", "") => ApiError::with_message(ValidationFailed, message),

        // Les trois exceptions que `org.merge_organizations()` et
        // `org.tg_forbid_merge_chains()` lèvent. **Leurs codes ont été relevés
        // SUR LA BASE**, jamais recopiés d'un document : les noms de condition
        // que le SQL écrit — `integrity_constraint_violation`,
        // `invalid_parameter_value`, `no_data_found` — se traduisent en 23000,
        // 22023 et P0002, et B1 a payé une fois d'avoir supposé au lieu de
        // mesurer.
        //
        // Le message est rendu **tel quel** dans les trois cas : le modèle
        // l'écrit en français, pour être lu. « Cibler la fiche finale » dit à
        // l'opérateur exactement quoi faire ; le reformuler produirait un second
        // libellé qui se périmerait à la première évolution du SQL.
        ("23000", _) => ApiError::with_message(Conflict, message),
        ("22023", _) => ApiError::with_message(ValidationFailed, message),
        ("P0002", _) => ApiError::with_message(NotFound, message),

        _ => ApiError::new(Internal),
    }
}
