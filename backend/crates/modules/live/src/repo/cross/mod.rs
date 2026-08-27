//! **Ce qu'un découplage aurait à couper**, et rien d'autre.
//!
//! Trois fichiers, trois schémas métier lus en lecture seule : `event`,
//! `programme`, `org`. Le module y pose des questions sur **ses propres**
//! entités — dans quel fuseau se lit la fenêtre d'un message, que puis-je viser,
//! que se joue-t-il aujourd'hui — et n'y écrit jamais.
//!
//! **`platform` et `reference` n'y sont pas**, et c'est délibéré : le principe
//! III les nomme comme noyau partagé. Les y ranger ferait perdre au dossier son
//! sens, qui est de lister exactement les frontières à trancher le jour où le
//! module deviendrait un service autonome. Le vocabulaire des natures
//! d'incident vit donc dans `repo/kinds.rs`.

pub mod event;
pub mod org;
pub mod programme;
