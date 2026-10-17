//! Module `live` — schéma PostgreSQL `live`.
//!
//! Ne dépend que de `kernel` et de `contracts` (principe II).
//!
//! # CE QUE CE CRATE SERT
//!
//! Les messages d'incident (`live.incidents` et quatre fonctions du § 6 de
//! `080_live.sql`), la lecture publique de `live.current_streams`, et depuis le
//! 17/10 les **réunions Zoom** des séances : `live.meetings`, leurs inscrits
//! (`live.meeting_participants`), le journal des notifications
//! (`live.provider_webhook_events`), six travaux de la file « live » et deux
//! consommateurs (inscriptions, séances annulées ou déplacées). Zoom s'éteint entièrement sans sa
//! configuration (`ZOOM_*`, `LIVE_SECRETS_KEY`) : ni travaux, ni consommateur,
//! et les routes qui en ont besoin répondent 503.
//!
//! # Les trois choses à savoir avant d'écrire une ligne ici
//!
//! **1. LE CRATE N'ÉMET AUCUN ÉVÉNEMENT.** `live.publish_incident()` émet
//! `live.incident.published` et `live.unpublish_incident()` émet
//! `live.incident.resolved`, toutes deux **dans la transaction de l'appelant**.
//! Ajouter un `kernel::events::emit` « pour faire comme les autres modules »
//! doublerait chaque ligne d'outbox, donc chaque réveil de tout consommateur
//! futur. Un test compte les lignes et exige exactement une.
//!
//! **2. LE PÉRIMÈTRE NE PEUT PAS ÊTRE UN `WHERE event_id`.** `live.incidents`
//! n'a aucune colonne d'édition pour les portées `session`, `event_day` et
//! `organization` : le rattachement est un **calcul**, et c'est
//! `live.event_incidents()` qui le fait. Un filtre écrit à la main laisserait
//! fuir trois portées sur cinq, et l'écran paraîtrait juste — il montrerait
//! simplement moins que ce qui existe.
//!
//! **3. L'ÉTAT D'UN MESSAGE NE SE RECOMPOSE JAMAIS.** Publié, non dépublié,
//! fenêtre ouverte, portée concernée : quatre conditions cumulées, que la v1
//! oubliait une par une — d'où ses bandeaux restés en ligne des mois.
//! `live.event_incidents()` les calcule une fois ; le code lit `state`.

use actix_web::web::ServiceConfig;
use kernel::config::Config;
use kernel::db::Db;
use kernel::events::EventConsumer;
use kernel::jobs::JobHandler;
use std::sync::Arc;

pub mod consumers;
pub mod domain;
pub mod jobs;
pub mod repo;
pub mod routes;
pub mod service;
pub mod state;
pub mod zoom;

pub use state::LiveState;

/// Le back-office des messages d'incident.
///
/// **Des routes plates, jamais un `web::scope("/admin")`** : le préfixe
/// d'administration est partagé avec la vitrine, le planificateur et les règles
/// de rappel, et deux scopes du même préfixe ne se complètent pas — Actix
/// retient le premier dont le préfixe correspond et rend 404 si la route n'y
/// figure pas. C'est le défaut qui a coûté trois routes sur vingt et une en B2.
pub fn routes(cfg: &mut ServiceConfig) {
    routes::admin::configurer(cfg);
    routes::meeting::chemins_admin(cfg);
}

/// La lecture publique, sous `/events/{event_id}/incidents`.
///
/// **Route plate elle aussi.** Le module `event` déclare ses routes `/events/…`
/// à plat, et `/events/{slug}` porte un segment là où celle-ci en porte deux :
/// les motifs ne se recouvrent pas, il n'y a donc rien à composer côté API.
pub fn event_routes(cfg: &mut ServiceConfig) {
    routes::public::configurer(cfg);
}

/// Les routes du module qui vivent sous `/sessions`, **sans le préfixe** : le
/// scope est partagé avec `programme` et `engagement`, et l'API le compose.
pub fn session_routes(cfg: &mut ServiceConfig) {
    routes::public::chemins_de_seance(cfg);
    routes::meeting::chemins_de_seance(cfg);
}

/// Les notifications de Zoom, montées seulement si leur jeton est configuré.
pub fn webhook_routes(cfg: &mut ServiceConfig, monte: bool) {
    routes::webhook::configurer(cfg, monte);
}

/// Vrai si Zoom est configuré et que son jeton de notification l'est aussi.
pub fn zoom_webhook_configure(config: &Config) -> bool {
    config
        .zoom
        .as_ref()
        .is_some_and(|z| z.webhook_secret_token.is_some())
}

/// Les six travaux de la file « live ». Aucun sans Zoom : les travaux
/// déposés attendent alors en file, sans échouer, qu'il soit configuré.
pub fn job_handlers(db: Db, config: &Config) -> Vec<Arc<dyn JobHandler>> {
    let Some(zoom) = state::zoom_depuis(config) else {
        return Vec::new();
    };
    vec![
        Arc::new(jobs::sync_meeting::SyncMeeting::new(
            db.clone(),
            zoom.clone(),
        )),
        Arc::new(jobs::sync_participant::SyncParticipant::new(
            db.clone(),
            zoom.clone(),
        )),
        Arc::new(jobs::cancel_participant::CancelParticipant::new(
            db.clone(),
            zoom.clone(),
        )),
        Arc::new(jobs::cancel_meeting::CancelMeeting::new(db.clone(), zoom)),
        Arc::new(jobs::filet::RequeueParticipants::new(db.clone())),
        Arc::new(jobs::webhook::ProcessWebhook::new(db)),
    ]
}

/// Le consommateur des inscriptions. Aucun sans Zoom : une réunion créée sans
/// personne pour la pousser chez Zoom ne servirait à rien.
pub fn event_consumers(config: &Config) -> Vec<Arc<dyn EventConsumer>> {
    if config.zoom.is_none() {
        return Vec::new();
    }
    vec![
        Arc::new(consumers::registrations::RegistrationsConsumer),
        Arc::new(consumers::sessions::SessionsConsumer),
    ]
}
