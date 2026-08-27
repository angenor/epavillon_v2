//! La permission du tableau de bord, déclarée par le module auquel elle
//! appartient.
//!
//! Elle vivait dans `api/src/routes/health.rs`, auprès de la seule route qui la
//! testait, avec un commentaire disant que « le module `analytics` n'a pas de
//! crate dans ce jalon ». Il en a un ; la déclaration revient chez elle, et les
//! deux routes qui la testent — `/health` et `/admin/dashboard` — nomment
//! désormais le même type.
//!
//! **Elle est testée par PERMISSION et par PORTÉE**, jamais par nom de rôle. Le
//! rôle `programmer` la détient depuis le 27/08, sur la portée de son
//! attribution : un compte détaché sur la COP31 ne l'a que sur `event:COP31`.

use kernel::auth::PermissionSpec;

pub struct DashboardRead;

impl PermissionSpec for DashboardRead {
    const CODE: &'static str = "analytics.dashboard.read";
}
