//! Module `negotiation` — schéma PostgreSQL `negotiation`.
//!
//! Ne dépend que de `kernel` et de `contracts` (principe II). Il naît à
//! l'étape 0b de Guide Négo, avec l'admission : le code d'invitation, la demande
//! d'accès, et ce que l'IFDD en fait au back-office.
//!
//! # Les trois choses à savoir avant d'écrire une ligne ici
//!
//! **1. Ce module n'accorde rien par lui-même.** Le droit vit dans
//! `identity.role_assignments`, et il se teste par `identity.has_permission()`
//! avec sa portée — une fonction SQL, jamais un appel vers le crate `identity`.
//! `invitation_code_uses`, `space_members` et `network_memberships` sont des
//! historiques : dupliquer l'état du RBAC, c'est se préparer à deux vérités.
//!
//! **2. Les refus d'un code sortent en 200**, avec leur discriminant et leur
//! message, comme `mfa_required` le fait déjà pour la connexion. Sept refus
//! prévus par le parcours ne sont pas sept pannes, et aucun écran ne doit rester
//! sans action possible.
//!
//! **3. Les invariants sont portés par la base**, et le code les traduit en
//! français — un usage en double, un quota dépassé, une portée incohérente, une
//! demande déjà tranchée. Aucune vérification préalable en Rust : deux requêtes
//! simultanées la contourneraient.

use actix_web::web::ServiceConfig;
use kernel::config::Config;
use kernel::db::Db;
use kernel::jobs::JobHandler;
use kernel::mail::Mailer;
use std::sync::Arc;

pub mod domain;
pub mod jobs;
pub mod mail;
pub mod pdf;
pub mod repo;
pub mod routes;
pub mod service;
pub mod state;

pub use state::NegotiationState;

/// Ce que l'application appelle : l'accès de la personne connectée, et les
/// thématiques qu'elle suit.
///
/// Les chemins sont plats et vivent sous `/negotiation` — aucun autre module
/// n'y dépose, il n'y a donc rien à composer côté API.
pub fn routes(cfg: &mut ServiceConfig) {
    routes::acces::configurer(cfg);
    routes::themes::configurer(cfg);
}

/// Le back-office de l'admission.
///
/// **Des routes plates, jamais un `web::scope("/admin")`** : le préfixe
/// d'administration est partagé avec cinq autres modules, et deux scopes du même
/// préfixe ne se complètent pas — un scope ici rendrait muettes leurs routes.
pub fn admin_routes(cfg: &mut ServiceConfig) {
    routes::admin_codes::configurer(cfg);
    routes::admin_requests::configurer(cfg);
    routes::admin_admission::configurer(cfg);
}

/// Les travaux différés du module : les deux courriels de décision, la purge
/// des essais de code, et l'extraction des documents.
///
/// **C'est ce seul geste qui fait écouter la file « negotiation ».**
/// `JobRegistry::queues()` est construite à partir des files que les
/// gestionnaires nomment, et `platform.claim_jobs()` filtre strictement : un
/// travail déposé dans une file inécoutée s'empile sans erreur, sans trace, et
/// sans que rien ne l'exécute jamais.
///
/// Les quatre déclarent la file par défaut : aucun déclencheur du modèle ne les
/// dépose ailleurs.
pub fn job_handlers(db: Db, config: &Config, mailer: Arc<dyn Mailer>) -> Vec<Arc<dyn JobHandler>> {
    let url = config.app_public_url.clone();
    vec![
        Arc::new(jobs::emails::SendApprovedEmail::new(
            mailer.clone(),
            url.clone(),
        )),
        Arc::new(jobs::emails::SendRejectedEmail::new(mailer, url)),
        Arc::new(jobs::purge::PurgeInvitationAttempts::new(db.clone())),
        Arc::new(jobs::extract::ExtractDocument::new(
            db,
            kernel::storage::Entrepots::new(&config.media),
            config.negotiation.pdfium_lib_path.clone(),
        )),
    ]
}
