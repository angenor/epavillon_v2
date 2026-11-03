//! Présence : combien de personnes sont sur la plateforme en ce moment, et où.
//!
//! Chaque onglet visible du site envoie `POST /presence` toutes les 30 s et à
//! chaque changement de zone ; `GET /admin/presence` lit le compte. Ni table,
//! ni Valkey, ni service tiers : un magasin en mémoire, juste parce que l'API
//! tourne en une seule instance (`ops/docker-compose.prod.yml`). Une seconde
//! instance couperait le compte en deux.
//!
//! Pourquoi pas le pouls du temps réel : il n'est appelé que par les pages qui
//! suivent un sujet public, et une personne connectée passe par la ligne
//! ouverte, qui ne rappelle pas à chaque page. Ni l'un ni l'autre ne dit la
//! zone de chaque onglet visible.
//!
//! Aucune donnée personnelle : l'identifiant d'onglet est tiré au hasard par le
//! navigateur, jamais journalisé ; ni personne, ni adresse IP, ni historique.

pub mod routes;
pub mod store;

use actix_web::web;
use kernel::error::ApiError;

pub use store::{Presence, PresenceSnapshot, Zone};

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/presence")
            // Le corps tient en quelques dizaines d'octets.
            .app_data(
                web::JsonConfig::default()
                    .limit(512)
                    .error_handler(|erreur, _| ApiError::from(&erreur).into()),
            )
            .route(web::post().to(routes::signaler)),
    )
    .route("/admin/presence", web::get().to(routes::lire));
}
