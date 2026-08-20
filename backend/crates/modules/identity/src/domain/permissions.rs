//! Les permissions du module, déclarées une fois.
//!
//! L'autorisation se teste par **permission**, jamais par nom de rôle : ces
//! constantes sont donc les seules chaînes que le code compare, et une faute de
//! frappe y devient une erreur de compilation partout où le type est exigé.

use kernel::auth::PermissionSpec;

pub const PERSON_READ: &str = "identity.person.read";
pub const PERSON_MANAGE: &str = "identity.person.manage";
pub const ROLE_ASSIGN: &str = "identity.role.assign";

pub struct PersonRead;
impl PermissionSpec for PersonRead {
    const CODE: &'static str = PERSON_READ;
}

pub struct PersonManage;
impl PermissionSpec for PersonManage {
    const CODE: &'static str = PERSON_MANAGE;
}

pub struct RoleAssign;
impl PermissionSpec for RoleAssign {
    const CODE: &'static str = ROLE_ASSIGN;
}
