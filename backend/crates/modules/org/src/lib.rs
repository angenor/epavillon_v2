//! Module `org` — schéma PostgreSQL `org`.
//!
//! Ne dépend que de `kernel` et de `contracts` (principe II). Le référentiel des
//! organisations, ses dénominations, ses domaines, ses adhésions, la détection
//! de ses doublons et leur fusion.
//!
//! **C'est la surface des quatre verrous** que `040_organizations.sql` a posés
//! contre le défaut n° 1 de la version 1 : deux fiches pour une même
//! organisation, l'une créée par qui cherchait le nom complet, l'autre par qui
//! cherchait le sigle.

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

pub use state::OrgState;

/// Routes exposées par le module. Le montage est décidé par l'API d'après
/// `platform.modules` : le module ne teste pas son propre état.
///
/// **Un seul `web::scope` par préfixe, et ce n'est pas une préférence de
/// style.** Actix retient le **premier** scope dont le préfixe correspond et
/// rend 404 si la route n'y figure pas — il n'essaie pas le suivant. Deux
/// `scope("/organizations")` enregistrés séparément ne se complètent donc pas :
/// le second est muet, et rien ne le signale à la compilation. Le défaut s'est
/// produit, et `crates/api/tests/routes_org.rs` est écrit pour qu'il ne se
/// reproduise pas.
pub fn routes(cfg: &mut ServiceConfig) {
    use actix_web::web;

    cfg.service(
        web::scope("/organizations")
            .configure(routes::public::configurer)
            .configure(routes::memberships::organisations),
    )
    .service(web::scope("/memberships").configure(routes::memberships::adhesions))
    .service(web::scope("/admin/organizations").configure(routes::admin::configurer));
}

/// Les routes du module qui vivent sous `/people`, préfixe que le module
/// `identity` monte déjà. **Elles sont donc montées à part par l'API**, dans le
/// scope existant : deux scopes du même préfixe ne se complètent pas.
pub fn people_routes(cfg: &mut ServiceConfig) {
    routes::memberships::personnes(cfg);
}

/// Travaux différés du module, montés par le worker sans qu'il les connaisse.
///
/// Six : trois de fond — détection, score, projection — et trois courriels. Le
/// transport du courriel est **choisi par la configuration** et passé ici :
/// aucun travail ne sait par où part son message.
pub fn job_handlers(db: Db, config: &Config, mailer: Arc<dyn Mailer>) -> Vec<Arc<dyn JobHandler>> {
    let url = config.app_public_url.clone();
    vec![
        Arc::new(jobs::duplicates::ScanDuplicates::new(db.clone(), config)),
        Arc::new(jobs::trust_score::RecomputeTrustScore::new(db.clone())),
        Arc::new(jobs::scorecard::RefreshScorecard::new(db)),
        Arc::new(jobs::emails::SendInvitationEmail::new(
            mailer.clone(),
            url.clone(),
        )),
        Arc::new(jobs::emails::SendRequestEmail::new(
            mailer.clone(),
            url.clone(),
        )),
        Arc::new(jobs::emails::SendApprovedEmail::new(mailer, url)),
    ]
}
