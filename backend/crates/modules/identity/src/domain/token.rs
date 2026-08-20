//! Jetons à usage unique — les liens reçus par courriel.
//!
//! **Trois refus, et leur ordre compte.** « Déjà utilisé » l'emporte sur
//! « périmé » : un jeton consommé puis périmé dit que le travail est fait, là
//! où « le lien a expiré » enverrait redemander un courriel inutile. C'est
//! l'ordre que suivent déjà les écrans du site.

use serde::Serialize;

/// **Le service de jetons vit désormais dans le noyau** (`kernel::tokens`) :
/// trois des cinq finalités du modèle n'appartiennent pas à ce module, et aucun
/// crate de module ne peut dépendre d'un autre. Les trois types sont réexportés
/// ici pour que les appelants du module — et ses tests de B1 — ne changent pas.
pub use kernel::tokens::{ConsumedToken, TokenPurpose, TokenRejection};

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
