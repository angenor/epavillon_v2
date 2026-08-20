//! Les permissions du module, déclarées une fois.
//!
//! L'autorisation se teste par **permission**, jamais par nom de rôle : ces
//! constantes sont les seules chaînes que le code compare.

use kernel::auth::PermissionSpec;

pub const EVENT_MANAGE: &str = "event.event.manage";
pub const CALL_MANAGE: &str = "event.call.manage";

/// **Permission d'un autre module, et cela ne crée aucune arête.**
///
/// Les deux routes du planificateur — le contrôle préalable et la publication —
/// sont gardées par elle, parce que le modèle décrit le rôle chargé de la
/// programmation comme celui qui « planifie les créneaux **et publie la
/// programmation** », et lui attribue cette permission-là. Le garder par
/// `event.event.manage` empêcherait un chargé de programmation de publier ce
/// que son rôle dit qu'il publie : ce rôle ne détient **aucune** permission de
/// ce module (écart n° 88).
///
/// Le garde vit dans `kernel` depuis B1, et une permission est une **chaîne
/// lue en base**, pas un symbole d'un autre crate : `cargo tree -p event` reste
/// sans arête (research.md § R12).
pub const SESSION_SCHEDULE: &str = "programme.session.schedule";

pub struct EventManage;
impl PermissionSpec for EventManage {
    const CODE: &'static str = EVENT_MANAGE;
}

pub struct CallManage;
impl PermissionSpec for CallManage {
    const CODE: &'static str = CALL_MANAGE;
}

pub struct SessionSchedule;
impl PermissionSpec for SessionSchedule {
    const CODE: &'static str = SESSION_SCHEDULE;
}
