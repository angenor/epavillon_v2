//! **La seule définition de « qui, dans une organisation, peut agir sur ce
//! dossier ».**
//!
//! # Un point resté ouvert, et l'hypothèse tenue en attendant
//!
//! La question a été posée au commanditaire et n'a pas reçu de réponse.
//! L'hypothèse de la spécification est tenue : **toute personne dont l'adhésion
//! est active** peut corriger, renvoyer et retirer — ce que l'écran suppose
//! déjà en rouvrant un dossier déposé deux mois plus tôt par une collègue.
//! Une personne dont la demande attend le référent ne touche qu'à ses propres
//! dossiers (arbitré le 28/09).
//!
//! **Elle est isolée ici, et nulle part ailleurs.** Si le commanditaire tranche
//! autrement — seule la déposante, ou la déposante et les référents —, une
//! fonction change et rien d'autre. Répandue dans douze gardes, la même
//! décision coûterait une relecture complète du module.
//!
//! # Ce qui ne passe pas par ce fichier
//!
//! Le **périmètre d'administration** : une organisation n'administre rien, et
//! son accès n'est pas un périmètre mais une adhésion (R13). Les deux voies
//! d'accès à un dossier sont distinctes et testées séparément.

use kernel::error::{ApiError, Result};

/// L'adhésion telle que `org.memberships` la porte, réduite à ce qui décide.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Adhesion {
    /// `org.membership_status` vaut `active`.
    pub active: bool,
    /// Demande d'adhésion que le référent n'a pas encore acceptée. Une
    /// invitation en attente n'en est pas une : la personne n'a rien accepté.
    pub en_attente: bool,
}

/// Cette personne agit-elle au nom de **toute** l'organisation — tous ses
/// dossiers, ses membres ?
pub fn peut_agir(adhesion: Option<Adhesion>) -> bool {
    adhesion.is_some_and(|a| a.active)
}

/// Peut-elle déposer au nom de l'organisation ?
///
/// **Rejoindre ne bloque pas le dépôt** (arbitré le 28/09) : une demande en
/// attente suffit. Le doublon d'organisation, que l'attente devait éviter,
/// coûte plus cher qu'un dossier déposé par une personne que le référent
/// n'a pas encore reconnue.
pub fn peut_deposer(adhesion: Option<Adhesion>) -> bool {
    adhesion.is_some_and(|a| a.active || a.en_attente)
}

/// Peut-elle agir sur **ce** dossier ? En attente de validation, seulement sur
/// les siens : rejoindre ne doit pas ouvrir le travail des collègues.
pub fn peut_agir_sur(adhesion: Option<Adhesion>, est_la_deposante: bool) -> bool {
    peut_agir(adhesion) || (est_la_deposante && peut_deposer(adhesion))
}

/// Le refus correspondant.
///
/// **C'est un `NOT_FOUND`, pas un `FORBIDDEN`**, et la nuance est celle du
/// principe IX : un dossier d'une organisation dont on n'est pas membre ne doit
/// pas se distinguer d'un dossier inexistant. Un 403 dirait à qui forge une URL
/// que le dossier existe.
pub fn exiger(autorise: bool) -> Result<()> {
    if autorise {
        Ok(())
    } else {
        Err(ApiError::not_found())
    }
}
