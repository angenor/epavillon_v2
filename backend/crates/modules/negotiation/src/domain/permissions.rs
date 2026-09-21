//! Les deux permissions du module, déclarées une fois.
//!
//! L'autorisation se teste par **permission** et par **portée**, jamais par nom
//! de rôle — et surtout pas par le nom `negotiator`, qui est celui du rôle que
//! l'admission attribue. Les deux portées possibles sont exactement les
//! `allowed_scopes` de ce rôle : `negotiation_space` ou `global`.

use kernel::auth::PermissionSpec;

pub const SPACE_ACCESS: &str = "negotiation.space.access";
pub const SPACE_MANAGE: &str = "negotiation.space.manage";

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
