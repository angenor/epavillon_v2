//! Module `live` — schéma PostgreSQL `live`.
//!
//! Ne dépend que de `kernel` et de `contracts` (principe II).
//!
//! # CE QUE CE CRATE NE SERT PAS
//!
//! Les quatre cinquièmes du schéma. `live.meetings` et leurs participants,
//! `live.provider_webhook_events`, `live.streams`, les vues `meetings_public`
//! et `current_streams`, les fonctions `build_embed_url()`,
//! `requeue_failed_participants()` et `replay_webhook_event()` : **rien de tout
//! cela n'est lu ni écrit ici**, parce qu'aucun écran du jalon ne le demande.
//! Le modèle les porte pour le jour où la visioconférence sera branchée ; les
//! servir d'avance produirait des routes qu'aucun appelant ne frappe, donc des
//! routes dont personne ne saurait qu'elles sont cassées.
//!
//! Ce crate sert **une** table — `live.incidents` — et **quatre** fonctions du
//! § 6 de `080_live.sql`.
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

pub mod domain;
pub mod repo;
pub mod routes;
pub mod service;
pub mod state;

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
}

/// La lecture publique, sous `/events/{event_id}/incidents`.
///
/// **Route plate elle aussi.** Le module `event` déclare ses routes `/events/…`
/// à plat, et `/events/{slug}` porte un segment là où celle-ci en porte deux :
/// les motifs ne se recouvrent pas, il n'y a donc rien à composer côté API.
pub fn event_routes(cfg: &mut ServiceConfig) {
    routes::public::configurer(cfg);
}
