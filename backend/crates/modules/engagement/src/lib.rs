//! Module `engagement` — schéma PostgreSQL `engagement`.
//!
//! Ne dépend que de `kernel` et de `contracts` (principe II) — **`media`
//! compris** : le seul lien que le modèle établit entre les deux schémas
//! appartient à la messagerie directe, hors périmètre.
//!
//! # Ce module est le plus gros consommateur d'outbox du dépôt, et il n'émet
//! presque rien
//!
//! C'est le contraire de B3, et c'est délibéré : tout ce qu'il pourrait
//! annoncer, la base l'annonce déjà. **Un seul événement sort d'ici** —
//! `engagement.email.suppressed` —, et `engagement.reminders.scheduled` est émis
//! par la fonction du modèle, qui met aussi **un travail par rappel** en file. Le
//! service ne redouble ni l'un ni l'autre : il produirait deux courriels par
//! rappel, et le doublon ne se verrait qu'en production.
//!
//! # Le périmètre est étroit, et il est écrit
//!
//! Notifications, préférences, modèles de messages, courriels et leur
//! délivrabilité, rappels programmés. **Hors périmètre** : commentaires,
//! réactions, messagerie directe, mise en relation, infolettres — aucune ligne
//! de ce crate ne nomme ces tables.

use actix_web::web::ServiceConfig;
use kernel::config::Config;
use kernel::db::Db;
use kernel::events::EventConsumer;
use kernel::jobs::JobHandler;
use kernel::mail::Mailer;
use std::sync::Arc;

pub mod consumers;
pub mod domain;
pub mod jobs;
pub mod mail;
pub mod repo;
pub mod routes;
pub mod service;
pub mod state;

pub use mail::GardedMailer;
pub use state::EngagementState;

/// Routes exposées par le module, sous les préfixes qui n'appartiennent qu'à
/// lui. Le montage est décidé par l'API d'après `platform.modules` : le module
/// ne teste pas son propre état.
///
/// Les deux routes du calendrier des rappels **n'y sont pas** : elles vivent
/// sous `/sessions`, préfixe que `programme` ouvre depuis B5, et le scope est
/// donc composé par l'API — voir [`session_routes`].
pub fn routes(cfg: &mut ServiceConfig) {
    routes::notifications::configurer(cfg);
    routes::preferences::configurer(cfg);
    routes::rules::configurer(cfg);
    routes::templates::configurer(cfg);
    routes::suppressions::configurer(cfg);
    routes::broadcast::configurer(cfg);
}

/// Les routes du module qui vivent sous `/sessions`, **sans le préfixe**.
///
/// Deux `web::scope` du même préfixe **ne se complètent pas** : Actix retient le
/// premier dont le préfixe correspond et rend 404 si la route n'y figure pas,
/// sans essayer le suivant. Le défaut a coûté trois routes sur vingt et une en
/// B2. `programme` cesse donc d'ouvrir le scope, l'API le compose une fois, et
/// **aucune route de B5 ne change de chemin**.
pub fn session_routes(cfg: &mut ServiceConfig) {
    routes::sessions::configurer(cfg);
}

/// **La porte d'ingestion des retours du fournisseur — montée seulement si son
/// jeton est configuré.**
///
/// Elle vit hors de [`routes`] parce que son montage dépend de la
/// configuration, et non de `platform.modules` : un jeton absent la ferme, et
/// elle rend 404 comme un module éteint. Une porte d'ingestion sans secret vaut
/// mieux fermée (R30).
pub fn internal_routes(cfg: &mut ServiceConfig, jeton_configure: bool) {
    routes::internal::configurer(cfg, jeton_configure);
}

/// Travaux différés du module, montés par le worker sans qu'il les connaisse.
pub fn job_handlers(db: Db, config: &Config, mailer: Arc<dyn Mailer>) -> Vec<Arc<dyn JobHandler>> {
    vec![
        Arc::new(jobs::send_reminder::SendReminder::new(
            db.clone(),
            config,
            mailer,
        )),
        // Le récurrent, sur la file par défaut : rien ne l'enfile, il pose sa
        // propre occurrence suivante, et le démarrage du worker réarme la chaîne.
        Arc::new(jobs::partitions::EnsurePartitions::new(
            db,
            config.engagement.partition_interval,
        )),
    ]
}

/// Consommateurs d'outbox du module.
///
/// **Deux, et leurs noms ne se renomment pas** : ils entrent dans
/// `platform.inbox_events`, et les changer ferait rejouer tout l'historique.
pub fn event_consumers(db: Db, _mailer: Arc<dyn Mailer>) -> Vec<Arc<dyn EventConsumer>> {
    vec![
        Arc::new(consumers::reminders::RemindersConsumer),
        Arc::new(consumers::notifications::NotificationsConsumer::new(db)),
    ]
}
