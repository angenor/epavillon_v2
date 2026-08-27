//! **Ce qu'un découplage aurait à couper**, et rien d'autre.
//!
//! Quatre fichiers, quatre schémas métier lus en lecture seule : `event`,
//! `programme`, `org`, `live`. C'est la lecture hors schéma la plus large du
//! dépôt, et elle est bornée à ces quatre-là.
//!
//! **`live.rs` lit une FONCTION SQL, jamais le crate `live`** : les deux
//! modules ne partagent aucune ligne de Rust, et `cargo tree` ne porte aucune
//! arête entre eux.
//!
//! **`platform` et `reference` n'y sont pas** — noyau partagé, principe III.
//! Ils vivent dans `repo/settings.rs` et `repo/reference.rs`.

pub mod event;
pub mod live;
pub mod org;
pub mod programme;
