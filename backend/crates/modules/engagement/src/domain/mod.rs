//! Types métier purs : ils se testent sans base.
//!
//! Deux fichiers portent chacun un piège que le reste du module ne rattraperait
//! pas : [`sanitize`], où une politique d'URL mal réglée détruit la variable
//! d'un lien de courriel — un défaut qui ne se voit qu'à la réception —, et
//! [`reminder`], dont la consolidation d'état est le miroir Rust de la règle
//! écrite en SQL.

pub mod ids;
pub mod notification;
pub mod offsets;
pub mod reminder;
pub mod render;
pub mod sanitize;
pub mod template;
