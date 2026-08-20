//! Exigences opposables du mot de passe.
//!
//! Les trois mêmes que `frontend/app/utils/password-strength.ts` applique déjà,
//! arrêtées par le commanditaire le 17/08 : huit signes, une majuscule, une
//! minuscule. Le chiffre et le caractère spécial n'en font pas partie — ils
//! conseillent, ils n'interdisent pas.
//!
//! L'indicateur de robustesse du site n'est pas repris : il guide une saisie,
//! il ne décide de rien. Ici, on décide.

use kernel::error::{ApiError, ErrorCode, Result};

pub const LONGUEUR_MIN: usize = 8;

pub fn est_conforme(mot_de_passe: &str) -> bool {
    mot_de_passe.chars().count() >= LONGUEUR_MIN
        && mot_de_passe.chars().any(char::is_uppercase)
        && mot_de_passe.chars().any(char::is_lowercase)
}

/// Le message du catalogue nomme les trois conditions d'un coup : les séparer
/// ferait trois messages pour un seul défaut de saisie.
pub fn exiger(mot_de_passe: &str) -> Result<()> {
    if est_conforme(mot_de_passe) {
        Ok(())
    } else {
        Err(ApiError::new(ErrorCode::IdentityPasswordTooWeak).field("password"))
    }
}
