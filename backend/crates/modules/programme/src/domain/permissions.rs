//! Les permissions du module, déclarées une fois.
//!
//! L'autorisation se teste par **permission**, jamais par nom de rôle : ces
//! constantes sont les seules chaînes que le code compare.
//!
//! **Détenir l'une n'accorde aucune autre**, et les combinaisons sont testées.
//! Deux conséquences valent d'être dites ici, parce qu'elles surprennent :
//!
//! - le rôle d'administration **ne détient pas** `programme.review.write` — il
//!   ne peut donc pas demander de corrections. C'est une ligne de la table des
//!   droits, modifiable au back-office, pas une fatalité du code (écart n° 50) ;
//! - `programme.proposal.submit` **ne s'appuie sur aucune portée** : c'est le
//!   droit du membre d'organisation, et le vrai contrôle est l'**adhésion
//!   active** (`domain/ownership.rs`).

use kernel::auth::PermissionSpec;

pub const PROPOSAL_SUBMIT: &str = "programme.proposal.submit";
pub const PROPOSAL_READ_ALL: &str = "programme.proposal.read_all";
pub const REVIEW_WRITE: &str = "programme.review.write";
pub const PROPOSAL_DECIDE: &str = "programme.proposal.decide";

/// **Permission d'un autre module, et cela ne crée aucune arête.**
///
/// C'est elle qui garde l'**affectation** d'un membre du comité à un dossier,
/// et non une permission de ce module : composer le comité et répartir sa
/// charge sont le même geste, celui de qui tient la campagne. L'écart n° 48 a
/// été tranché en ce sens par l'écran A7.
///
/// Le garde vit dans `kernel` depuis B1, et une permission est une **chaîne lue
/// en base**, pas un symbole d'un autre crate : `cargo tree -p programme` reste
/// sans arête (research.md § R12, précédent de B3).
pub const CALL_MANAGE: &str = "event.call.manage";

pub struct ProposalSubmit;
impl PermissionSpec for ProposalSubmit {
    const CODE: &'static str = PROPOSAL_SUBMIT;
}

pub struct ProposalReadAll;
impl PermissionSpec for ProposalReadAll {
    const CODE: &'static str = PROPOSAL_READ_ALL;
}

pub struct ReviewWrite;
impl PermissionSpec for ReviewWrite {
    const CODE: &'static str = REVIEW_WRITE;
}

pub struct ProposalDecide;
impl PermissionSpec for ProposalDecide {
    const CODE: &'static str = PROPOSAL_DECIDE;
}

pub struct CallManage;
impl PermissionSpec for CallManage {
    const CODE: &'static str = CALL_MANAGE;
}
