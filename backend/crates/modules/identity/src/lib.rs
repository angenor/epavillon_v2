//! Module `identity` — schéma PostgreSQL `identity`.
//!
//! Ne dépend que de `kernel` et de `contracts` (principe II).

use actix_web::web::ServiceConfig;
use kernel::config::Config;
use kernel::db::Db;
use kernel::jobs::JobHandler;
use kernel::mail::Mailer;
use std::sync::Arc;

pub mod domain;
pub mod jobs;
pub mod mail;
pub mod repo;
pub mod routes;
pub mod service;
pub mod state;

pub use domain::access_token::AccessTokenCodec;
pub use routes::auth::{resolve_actor, COOKIE_ACCES};
pub use state::IdentityState;

/// Routes exposées par le module. Le montage est décidé par l'API d'après
/// `platform.modules` : le module ne teste pas son propre état.
pub fn routes(cfg: &mut ServiceConfig) {
    routes::auth::configurer(cfg);
    routes::people::configurer(cfg);
    routes::admin_users::configurer(cfg);
    routes::admin_privacy::configurer(cfg);
}

/// Travaux différés du module, montés par le worker sans qu'il les connaisse.
///
/// Le transport du courriel est **choisi par la configuration** et passé ici :
/// aucun travail ne sait par où part son message, ce qui est exactement ce qui
/// rendra la bascule vers l'envoi direct indolore (research.md § R13).
pub fn job_handlers(db: Db, config: &Config, mailer: Arc<dyn Mailer>) -> Vec<Arc<dyn JobHandler>> {
    let url = config.app_public_url.clone();
    vec![
        Arc::new(jobs::purge::PurgeExpiredTokens::new(db)),
        Arc::new(jobs::emails::SendVerificationEmail::new(
            mailer.clone(),
            url.clone(),
        )),
        Arc::new(jobs::emails::SendExistingAccountNotice::new(
            mailer.clone(),
            url.clone(),
        )),
        Arc::new(jobs::emails::SendPasswordResetEmail::new(mailer, url)),
    ]
}
