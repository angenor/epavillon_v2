//! Les trois permissions du module, déclarées une fois.
//!
//! L'autorisation se teste par **permission**, jamais par nom de rôle. La
//! qualité de **référent** d'une organisation n'est pas ici : ce n'est pas une
//! permission mais une adhésion, lue en base à chaque écriture (research.md
//! § R16).

use kernel::auth::PermissionSpec;

pub const ORGANIZATION_READ: &str = "org.organization.read";
pub const ORGANIZATION_MANAGE: &str = "org.organization.manage";
pub const ORGANIZATION_MERGE: &str = "org.organization.merge";

pub struct OrganizationRead;
impl PermissionSpec for OrganizationRead {
    const CODE: &'static str = ORGANIZATION_READ;
}

pub struct OrganizationManage;
impl PermissionSpec for OrganizationManage {
    const CODE: &'static str = ORGANIZATION_MANAGE;
}

pub struct OrganizationMerge;
impl PermissionSpec for OrganizationMerge {
    const CODE: &'static str = ORGANIZATION_MERGE;
}
