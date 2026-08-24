//! Module `content` — schéma PostgreSQL `content`.
//!
//! Ne dépend que de `kernel` et de `contracts` (principe II).
//!
//! # Les deux choses à savoir avant d'écrire une ligne ici
//!
//! **1. La vitrine ne sert QUE le bandeau d'ouverture.** `content.highlight_placement`
//! ne porte plus qu'une valeur depuis le 24/08 : la colonne « À venir » de
//! l'accueil s'alimente seule — événements à venir, puis frise des activités
//! retenues — et ne se compose plus à la main. Un emplacement sans rendu
//! n'existe pas, c'est la règle écrite dans `115_content.sql`.
//!
//! **2. La fenêtre de diffusion est appliquée PAR LA VUE**, jamais par ce code.
//! `content.v_showcase` écarte déjà ce qui n'est pas publié et ce qui est hors
//! fenêtre. Rejouer le filtre ici, ce serait la v1 : chaque appelant comparait
//! les dates à sa façon, et une annonce périmée survivait à l'endroit où
//! quelqu'un avait oublié la comparaison.

use actix_web::web::ServiceConfig;

pub mod domain;
pub mod repo;
pub mod routes;
pub mod service;
pub mod state;

pub use state::ContentState;

/// Routes exposées par le module. Le montage est décidé par l'API d'après
/// `platform.modules` : le module ne teste pas son propre état.
///
/// `/home` est un chemin plat et n'appartient qu'à ce module — aucun autre n'y
/// dépose, il n'y a donc rien à composer côté API.
pub fn routes(cfg: &mut ServiceConfig) {
    routes::public::configurer(cfg);
    // Le back-office. **Des routes plates, jamais un `web::scope("/admin")`** :
    // le préfixe d'administration est partagé avec d'autres modules, et deux
    // scopes du même préfixe ne se complètent pas — un scope ici rendrait
    // muettes leurs routes.
    routes::admin::configurer(cfg);
}
