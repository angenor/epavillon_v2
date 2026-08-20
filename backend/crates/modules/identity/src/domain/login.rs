//! Les six issues de connexion, et les énumérations du modèle qu'elles portent.
//!
//! L'ordre dans lequel elles sont produites est une règle, pas un détail
//! (FR-019 à FR-021) : tant que le mot de passe n'est pas juste, la seule issue
//! possible est « identifiants invalides ». Verrou, suspension, adresse non
//! vérifiée et second facteur ne se signalent qu'ensuite, et dans cet ordre —
//! chacun suppose que l'identité soit déjà prouvée, sans quoi la réponse
//! renseignerait qui ne connaît pas le mot de passe.

use serde::Serialize;
use time::OffsetDateTime;
use uuid::Uuid;

use super::person::PersonView;

/// ENUM `identity.person_status`.
///
/// Les valeurs traversent SQL en texte : la macro SQLx ne sait pas typer un
/// paramètre d'énumération dans un schéma nommé, et un transtypage explicite
/// vaut mieux qu'une correspondance de type devinée à la compilation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PersonStatus {
    Active,
    Suspended,
    Blocked,
    Anonymized,
}

impl PersonStatus {
    pub fn as_db(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Suspended => "suspended",
            Self::Blocked => "blocked",
            Self::Anonymized => "anonymized",
        }
    }

    /// Un statut inconnu est un défaut de code — l'énuméré est fermé en base —,
    /// jamais une donnée d'utilisateur : on ne replie pas sur `active`.
    pub fn from_db(valeur: &str) -> Option<Self> {
        match valeur {
            "active" => Some(Self::Active),
            "suspended" => Some(Self::Suspended),
            "blocked" => Some(Self::Blocked),
            "anonymized" => Some(Self::Anonymized),
            _ => None,
        }
    }

    pub fn peut_se_connecter(self) -> bool {
        self == Self::Active
    }
}

/// ENUM `identity.auth_provider`. Seul `Password` est atteignable dans ce
/// jalon : la fédération reste hors périmètre (décision du 17/08).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthProvider {
    Password,
    Google,
    Microsoft,
    Linkedin,
    Oidc,
}

impl AuthProvider {
    pub fn as_db(self) -> &'static str {
        match self {
            Self::Password => "password",
            Self::Google => "google",
            Self::Microsoft => "microsoft",
            Self::Linkedin => "linkedin",
            Self::Oidc => "oidc",
        }
    }

    pub fn from_db(valeur: &str) -> Option<Self> {
        match valeur {
            "password" => Some(Self::Password),
            "google" => Some(Self::Google),
            "microsoft" => Some(Self::Microsoft),
            "linkedin" => Some(Self::Linkedin),
            "oidc" => Some(Self::Oidc),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MfaMethod {
    Totp,
}

/// Issue d'une tentative de connexion — l'union que `LoginResult` déclare côté
/// site. **Les six sortent en 200** : un refus prévu par le contrat n'est pas
/// une erreur HTTP, et le client du front lève sur tout statut d'erreur.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum LoginOutcome {
    Authenticated {
        person: Box<PersonView>,
    },
    /// Emplacement réservé : aucune route ne complète le défi dans ce jalon
    /// (arbitrage du 20/08). Le contrat reste honorable, l'écran affiche ce
    /// qu'il affiche déjà.
    MfaRequired {
        challenge_id: Uuid,
        method: MfaMethod,
    },
    InvalidCredentials,
    Locked {
        #[serde(with = "time::serde::rfc3339")]
        until: OffsetDateTime,
    },
    Suspended {
        #[serde(with = "time::serde::rfc3339::option")]
        until: Option<OffsetDateTime>,
    },
    EmailUnverified {
        email: String,
    },
}

impl LoginOutcome {
    pub fn est_authentifie(&self) -> bool {
        matches!(self, Self::Authenticated { .. })
    }
}
