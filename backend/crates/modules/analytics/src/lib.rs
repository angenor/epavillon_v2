//! Module `analytics` — schéma PostgreSQL `analytics`.
//!
//! Ne dépend que de `kernel` et de `contracts` (principe II).
//!
//! # LA LECTURE HORS SCHÉMA LA PLUS LARGE DU DÉPÔT, ET POURQUOI ELLE TIENT
//!
//! La composition du tableau de bord lit **quatre schémas métier** — `event`,
//! `programme`, `org`, `live` — en **lecture seule**, sous `repo/cross/`. Ce
//! n'est pas une entorse au principe II, et le dossier est la preuve : il liste
//! **exactement** ce qu'un découplage aurait à couper, un fichier par frontière.
//!
//! Trois choses le tiennent :
//!
//! - **aucune écriture**, dans aucun schéma autre que le sien — `platform` et
//!   `reference` compris —, et un test le vérifie mécaniquement ;
//! - **aucun appel à un autre crate de module** : les incidents actifs se lisent
//!   par `live.active_incidents_for_event()`, une fonction SQL, jamais par le
//!   crate `live` ;
//! - la question posée reste **la sienne** : « où en est cette édition ? » est
//!   la définition même d'un tableau de bord, et la mesure est ce que ce module
//!   fait.
//!
//! **`platform` et `reference` ne sont PAS dans `cross/`** : le principe III les
//! nomme comme noyau partagé. Les y ranger ferait perdre au dossier son sens.
//! Ils vivent dans `repo/settings.rs` et `repo/reference.rs`.
//!
//! # Ce que ce module ne fait pas
//!
//! Il **n'émet aucun événement**, et n'a rien à émettre : il mesure, il ne
//! change aucun état. Il **n'écoute aucun événement** non plus — l'écart que
//! gagnerait un consommateur d'outbox serait au plus d'un intervalle, et son
//! effet serait invisible à l'écran, donc invérifiable (voir
//! `specs/007-direct-tableaux-de-bord/contracts/events.md`). Ce qu'il porte,
//! c'est **un** travail différé : le rafraîchissement périodique des huit
//! projections.

use actix_web::web::ServiceConfig;
use kernel::config::Config;
use kernel::db::Db;
use kernel::jobs::JobHandler;
use std::sync::Arc;

pub mod authz;
pub mod domain;
pub mod jobs;
pub mod repo;
pub mod routes;
pub mod service;
pub mod state;

pub use state::AnalyticsState;

/// Le tableau de bord du back-office. **Chemin plat**, même raison que partout
/// ailleurs : `/admin` est un préfixe partagé, et deux scopes du même préfixe
/// ne se complètent pas.
pub fn routes(cfg: &mut ServiceConfig) {
    routes::admin::configurer(cfg);
}

/// **C'est ce seul geste qui fait écouter la file « analytics ».**
///
/// `JobRegistry::queues()` est construite à partir des files que les
/// gestionnaires nomment, et `platform.claim_jobs()` filtre strictement : un
/// travail déposé dans une file inécoutée s'empile **sans erreur, sans trace, et
/// sans que rien ne l'exécute jamais**.
pub fn job_handlers(db: Db, config: &Config) -> Vec<Arc<dyn JobHandler>> {
    vec![Arc::new(jobs::refresh::RefreshAll::new(
        db,
        config.analytics.refresh_interval,
        config.analytics.refresh_debounce,
    ))]
}
