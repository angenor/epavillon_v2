//! Module `event` — schéma PostgreSQL `event`.
//!
//! Ne dépend que de `kernel` et de `contracts` (principe II). Les éditions, leur
//! calendrier, leurs journées spéciales, le stand et ses salles, le canal du
//! direct, l'appel à propositions et sa grille d'évaluation.
//!
//! **Rien n'est à semer** : `platform.modules` porte déjà l'entrée `event`
//! (schéma `event`, dépendant de `org` et `identity`), posée par
//! `010_platform.sql` § 7.

use actix_web::web::ServiceConfig;
use kernel::config::Config;
use kernel::db::Db;
use kernel::jobs::JobHandler;
use std::sync::Arc;

pub mod domain;
pub mod jobs;
pub mod repo;
pub mod routes;
pub mod service;
pub mod state;

pub use state::EventState;

/// Routes exposées par le module. Le montage est décidé par l'API d'après
/// `platform.modules` : le module ne teste pas son propre état.
///
/// **Les deux routes du planificateur n'y sont pas** : leur préfixe
/// `/admin/planner` sera partagé avec le module Programmation en B5, et deux
/// `web::scope` du même préfixe **ne se complètent pas** — Actix retient le
/// premier et rend 404 si la route n'y figure pas, sans essayer le suivant. Le
/// défaut a coûté trois routes muettes sur vingt et une en B2. Elles sont donc
/// exposées à part, par [`planner_routes`], et l'API compose le préfixe une
/// seule fois (research.md § R11).
///
/// **L'ordre d'enregistrement compte aussi**, pour une autre raison : trois
/// chemins littéraux seraient capturés par un chemin paramétré s'ils étaient
/// déclarés après lui — `/events/public` par `/events/{slug}`,
/// `/admin/events/form-options` par `/admin/events/{id}`, et
/// `/admin/calls/default-criteria` par principe.
pub fn routes(cfg: &mut ServiceConfig) {
    use actix_web::web;

    cfg.service(web::scope("/admin/events").configure(routes::admin_events::configurer));
    cfg.service(web::scope("/admin/calls").configure(routes::admin_call::configurer));
    cfg.configure(routes::admin_tabs::configurer);
    cfg.configure(routes::public::configurer);
}

/// Ce que ce module dépose sous `/admin/planner`, **sans le préfixe** : c'est
/// l'API qui le compose. Voir [`routes`].
pub fn planner_routes(cfg: &mut ServiceConfig) {
    routes::planner::configurer(cfg);
}

/// Travaux différés du module, montés par le worker sans qu'il les connaisse.
///
/// Un seul : la clôture des appels échus. Sa cadence vient de la configuration,
/// validée au démarrage — une durée illisible arrête le service, jamais une
/// requête.
pub fn job_handlers(db: Db, config: &Config) -> Vec<Arc<dyn JobHandler>> {
    vec![Arc::new(jobs::autoclose::CloseExpiredCalls::new(
        db,
        config.event.call_autoclose_interval,
    ))]
}
