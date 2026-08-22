//! Module `media` — schéma PostgreSQL `media`.
//!
//! Ne dépend que de `kernel` et de `contracts` (principe II) — **`engagement`
//! compris**.
//!
//! # Les deux choses à savoir avant d'écrire une ligne ici
//!
//! **1. Insérer une ligne dans `media.assets` déclenche tout.**
//! `media.tg_enqueue_processing()` met le traitement en file **et** émet
//! `media.asset.uploaded`. Le service n'appelle donc ni `kernel::jobs::enqueue`,
//! ni `kernel::events::emit` pour ce geste : il produirait deux traitements par
//! fichier, et le doublon ne se verrait qu'en production. Le **seul** événement
//! que ce module émet est `media.asset.purged`, quand un objet a réellement
//! quitté le stockage.
//!
//! **2. Aucune permission `media.*` n'existe.** Le droit de poser un fichier est
//! le droit d'écrire sur ce qu'il illustre, et la table qui l'exprime est
//! [`domain::guards`] — le fichier le plus important du module. Une combinaison
//! qui n'y figure pas est **refusée**, jamais autorisée par défaut.

use actix_web::web::ServiceConfig;
use kernel::config::Config;
use kernel::db::Db;
use kernel::jobs::JobHandler;
use std::sync::Arc;

pub mod domain;
pub mod jobs;
pub mod repo;
pub mod routes;
pub mod scan;
pub mod service;
pub mod state;
pub mod storage;

pub use state::MediaState;

/// Routes exposées par le module. Le montage est décidé par l'API d'après
/// `platform.modules` : le module ne teste pas son propre état.
///
/// Le préfixe `/media` n'appartient qu'à lui : aucun autre module n'y dépose, il
/// n'y a donc rien à composer côté API. Le back-office, lui, vit sous `/admin`,
/// **préfixe partagé** — voir plus bas pourquoi ses routes y sont plates.
pub fn routes(cfg: &mut ServiceConfig) {
    use actix_web::web;

    // **Chemins littéraux avant chemins paramétrés.** `/assets/precheck` et
    // `/assets/{id}` ont le même nombre de segments et la même méthode n'est pas
    // en cause — mais l'ordre est tenu par la structure plutôt que par la
    // vigilance, comme dans les modules livrés.
    cfg.service(
        web::scope("/media")
            .configure(routes::uploads::configurer)
            .configure(routes::assets::configurer)
            .configure(routes::roles::configurer)
            .configure(routes::attachments::configurer),
    );

    // Le back-office. **Trois routes plates, jamais un `web::scope("/admin")`** :
    // le préfixe d'administration est partagé — `/admin/planner` vient de B5,
    // `/admin/reminder-rules` de B6 —, et deux scopes du même préfixe ne se
    // complètent pas. Un scope ici rendrait muettes les routes des autres
    // modules, exactement le défaut qui a coûté trois routes en B2.
    routes::admin::configurer(cfg);
}

/// Travaux différés du module, montés par le worker sans qu'il les connaisse.
///
/// **Le traitement n'est pas mis en file ici** : `media.tg_enqueue_processing()`
/// l'enfile, dans la file « media » — que le worker n'écoute que parce que le
/// gestionnaire la déclare. Les deux récurrents, eux, n'ont aucun déclencheur :
/// ils posent leur propre occurrence suivante, et le démarrage du worker réarme
/// la chaîne.
pub fn job_handlers(db: Db, config: &Config) -> Vec<Arc<dyn JobHandler>> {
    let storage = crate::storage::build(&config.media);
    let scanner = crate::scan::build(&config.media);

    vec![
        Arc::new(jobs::process::ProcessAsset::new(
            db.clone(),
            storage.clone(),
            scanner,
        )),
        // Les deux récurrents. Ils déclarent la MÊME file que le traitement :
        // le worker n'écoute que celles que ses gestionnaires nomment, et une
        // file de plus serait une file de plus à ouvrir pour deux travaux par
        // jour.
        Arc::new(jobs::purge::PurgeAssets::new(
            db.clone(),
            storage,
            config.media.purge_interval,
        )),
        Arc::new(jobs::reconcile::ReconcileQuotas::new(
            db,
            config.media.reconcile_interval,
        )),
    ]
}
