//! Jetons à usage unique — les liens reçus par courriel.
//!
//! **Trois refus, et leur ordre compte.** « Déjà utilisé » l'emporte sur
//! « périmé » : un jeton consommé puis périmé dit que le travail est fait, là
//! où « le lien a expiré » enverrait redemander un courriel inutile. C'est
//! l'ordre que suivent déjà les écrans du site.

use serde::Serialize;
use time::OffsetDateTime;

use super::ids::{PersonId, TokenId};

/// Valeurs de `identity.token_purpose`. La finalité **détermine la durée de
/// validité** : aucun appelant ne pose d'expiration lui-même (FR-017, FR-018).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenPurpose {
    EmailVerification,
    PasswordReset,
    Invitation,
    MagicLink,
    SpeakerConfirmation,
}

impl TokenPurpose {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::EmailVerification => "email_verification",
            Self::PasswordReset => "password_reset",
            Self::Invitation => "invitation",
            Self::MagicLink => "magic_link",
            Self::SpeakerConfirmation => "speaker_confirmation",
        }
    }
}

/// Pourquoi un jeton a été refusé.
///
/// Le modèle ne distingue pas ces trois cas — il porte `consumed_at` et
/// `expires_at`, rien de plus. L'écran, lui, ne propose pas la même suite : un
/// lien périmé se redemande, un lien déjà consommé signifie que c'est fait.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenRejection {
    Invalid,
    Expired,
    AlreadyUsed,
}

impl TokenRejection {
    /// L'ordre du contrat, en un seul endroit. `consommee` avant `expiree` :
    /// inverser les deux lignes suffirait à renvoyer quelqu'un demander un
    /// courriel dont il n'a plus besoin.
    pub fn from_state(
        consommee: Option<OffsetDateTime>,
        expiration: OffsetDateTime,
        maintenant: OffsetDateTime,
    ) -> Self {
        if consommee.is_some() {
            Self::AlreadyUsed
        } else if expiration <= maintenant {
            Self::Expired
        } else {
            // Le jeton est valide : l'appelant n'aurait pas dû demander de refus.
            // Le cas n'arrive que si la ligne a changé entre deux lectures, et
            // « invalide » est alors le seul refus honnête.
            Self::Invalid
        }
    }
}

/// Un jeton consommé, et ce qu'il portait. Le clair n'existe plus à ce stade :
/// il a servi à retrouver la ligne, et rien d'autre.
#[derive(Debug, Clone)]
pub struct ConsumedToken {
    pub id: TokenId,
    pub person_id: Option<PersonId>,
    pub payload: serde_json::Value,
}

/// Ce que rend une vérification d'adresse.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum VerifyEmailOutcome {
    Verified { email: String },
    Rejected { reason: TokenRejection },
}

/// **Issue unique.** L'adresse était libre ou déjà prise : la réponse ne le dit
/// pas (FR-035). C'est le courriel envoyé qui diffère — un lien de vérification
/// d'un côté, un rappel « vous avez déjà un compte » de l'autre.
#[derive(Debug, Clone, Serialize)]
pub struct RegisterOutcome {
    pub status: &'static str,
    pub email: String,
}

impl RegisterOutcome {
    pub fn verification_sent(email: impl Into<String>) -> Self {
        Self {
            status: "verification_sent",
            email: email.into(),
        }
    }
}

/// Renvoi du lien de vérification. Issue unique, elle aussi : rien ne doit se
/// déduire de la réponse (FR-036).
#[derive(Debug, Clone, Serialize)]
pub struct ResendOutcome {
    pub status: &'static str,
}

impl ResendOutcome {
    pub fn sent() -> Self {
        Self { status: "sent" }
    }
}

/// Demande de réinitialisation. **Issue unique**, adresse connue ou non
/// (FR-036) : seul le courriel diffère, et il n'arrive que si le compte existe.
#[derive(Debug, Clone, Serialize)]
pub struct PasswordResetRequestOutcome {
    pub status: &'static str,
}

impl PasswordResetRequestOutcome {
    pub fn sent() -> Self {
        Self { status: "sent" }
    }
}

/// Contrôle d'un jeton **avant** d'afficher le formulaire. L'adresse revient
/// avec la réponse valide : l'écran l'affiche pour que la personne sache quel
/// compte elle est en train de reprendre.
///
/// Ce contrôle ne consomme rien et **ne vaut aucune garantie** : le jeton est
/// revérifié à l'enregistrement (FR-042).
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum TokenCheckOutcome {
    Valid { email: String },
    Rejected { reason: TokenRejection },
}

/// Enregistrement du nouveau mot de passe. Les trois refus de jeton sortent
/// **en 200** avec leur discriminant ; seul un mot de passe refusé sort en 422,
/// sur le champ `password`.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum PasswordResetOutcome {
    Reset { email: String },
    Rejected { reason: TokenRejection },
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Duration;

    #[test]
    fn deja_utilise_lemporte_sur_perime() {
        let maintenant = OffsetDateTime::now_utc();
        let hier = maintenant - Duration::days(1);

        assert_eq!(
            TokenRejection::from_state(Some(hier), hier, maintenant),
            TokenRejection::AlreadyUsed,
            "un jeton consommé PUIS périmé dit que le travail est fait"
        );
        assert_eq!(
            TokenRejection::from_state(None, hier, maintenant),
            TokenRejection::Expired
        );
    }
}
