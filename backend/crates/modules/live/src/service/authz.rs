//! **La portée VISÉE par une écriture** — pas celle depuis laquelle on agit.
//!
//! Un message de portée `event`, `event_day`, `session` ou `organization` se
//! vérifie sur **l'édition à laquelle sa cible se rattache**. Un message de
//! portée `global` se vérifie sur la **portée globale** : il s'affiche partout,
//! et un compte détaché sur une seule édition n'a pas à en poser ni à en
//! retirer un.
//!
//! **La différence est portée par `identity.has_permission()`, sans une ligne de
//! code supplémentaire** : un compte détaché ne détient `live.incident.publish`
//! que sur `event:<son édition>`, jamais sur la portée globale.
//!
//! Sur une correction, c'est la portée **d'arrivée** qui compte : déplacer un
//! message d'une édition vers la portée globale exige la permission globale.

use kernel::auth::Scope;
use uuid::Uuid;

/// La permission qui ouvre les quatre écritures. Testée par **permission**, et
/// jamais par nom de rôle.
pub const INCIDENT_PUBLISH: &str = "live.incident.publish";

/// La portée sur laquelle l'autorisation se vérifie.
///
/// `edition_de_la_cible` est celle que le dépôt a calculée pour la cible visée ;
/// elle est ignorée pour un message global, qui n'en a aucune.
pub fn portee_visee(scope: &str, edition_de_la_cible: Option<Uuid>) -> Option<Scope> {
    match scope {
        "global" => Some(Scope::Global),
        "event" | "event_day" | "session" | "organization" => edition_de_la_cible.map(Scope::Event),
        _ => None,
    }
}
