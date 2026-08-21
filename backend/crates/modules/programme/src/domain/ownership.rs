//! **La seule définition de « qui, dans une organisation, peut agir sur ce
//! dossier ».**
//!
//! # Un point resté ouvert, et l'hypothèse tenue en attendant
//!
//! La question a été posée au commanditaire et n'a pas reçu de réponse.
//! L'hypothèse de la spécification est tenue : **toute personne dont l'adhésion
//! est active** peut corriger, renvoyer et retirer — ce que l'écran suppose
//! déjà en rouvrant un dossier déposé deux mois plus tôt par une collègue.
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
    /// `org.membership_status` : `pending`, `active` ou `revoked`.
    pub active: bool,
}

/// Cette personne peut-elle agir au nom de l'organisation porteuse ?
///
/// **L'adhésion en attente ne suffit pas.** Une demande d'adhésion non
/// approuvée donnerait à quiconque connaît le nom d'une organisation le droit
/// d'écrire en son nom — c'est le seul point où l'hypothèse ci-dessus ne fait
/// aucun doute.
pub fn peut_agir(adhesion: Option<Adhesion>) -> bool {
    adhesion.is_some_and(|a| a.active)
}

/// Le refus correspondant.
///
/// **C'est un `NOT_FOUND`, pas un `FORBIDDEN`**, et la nuance est celle du
/// principe IX : un dossier d'une organisation dont on n'est pas membre ne doit
/// pas se distinguer d'un dossier inexistant. Un 403 dirait à qui forge une URL
/// que le dossier existe.
pub fn exiger(adhesion: Option<Adhesion>) -> Result<()> {
    if peut_agir(adhesion) {
        Ok(())
    } else {
        Err(ApiError::not_found())
    }
}
