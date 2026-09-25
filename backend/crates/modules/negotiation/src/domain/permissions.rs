//! Les permissions du module, déclarées une fois.
//!
//! L'autorisation se teste par **permission** et par **portée**, jamais par nom
//! de rôle — et surtout pas par le nom `negotiator`, qui est celui du rôle que
//! l'admission attribue. Les deux portées possibles sont exactement les
//! `allowed_scopes` de ce rôle : `negotiation_space` ou `global`.

use kernel::auth::PermissionSpec;

pub const SPACE_ACCESS: &str = "negotiation.space.access";
pub const SPACE_MANAGE: &str = "negotiation.space.manage";
pub const DOCUMENT_PUBLISH: &str = "negotiation.document.publish";
pub const CORRECTION_POST: &str = "negotiation.correction.post";
pub const CORRECTION_WITHDRAW: &str = "negotiation.correction.withdraw";
pub const REPORT_VALIDATE: &str = "negotiation.report.validate";

/// Entrer dans l'espace réservé. C'est ce que le code d'invitation ouvre.
pub struct SpaceAccess;
impl PermissionSpec for SpaceAccess {
    const CODE: &'static str = SPACE_ACCESS;
}

/// Tenir les codes, trancher les demandes, retirer un accès. Le back-office.
pub struct SpaceManage;
impl PermissionSpec for SpaceManage {
    const CODE: &'static str = SPACE_MANAGE;
}

/// Créer, modifier, publier et dépublier un document de Guide Négo.
pub struct DocumentPublish;
impl PermissionSpec for DocumentPublish {
    const CODE: &'static str = DOCUMENT_PUBLISH;
}

/// Poser une note de correction : le geste de l'expert.
pub struct CorrectionPost;
impl PermissionSpec for CorrectionPost {
    const CODE: &'static str = CORRECTION_POST;
}

/// Retirer une note de correction.
pub struct CorrectionWithdraw;
impl PermissionSpec for CorrectionWithdraw {
    const CODE: &'static str = CORRECTION_WITHDRAW;
}

/// Valider, refuser, retirer un signalement du réseau (3b). Portée globale.
pub struct ReportValidate;
impl PermissionSpec for ReportValidate {
    const CODE: &'static str = REPORT_VALIDATE;
}
