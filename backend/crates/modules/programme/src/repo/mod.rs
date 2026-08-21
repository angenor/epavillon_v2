//! Requêtes SQLx, un fichier par agrégat. Aucune règle métier ici : le dépôt
//! lit et écrit, le service décide.
//!
//! **Trois régimes, trois fichiers, et la distinction compte** : `cross.rs` ne
//! porte que des LECTURES hors schéma — geste ordinaire ; `themes.rs` et
//! `people.rs` portent les deux ÉCRITURES hors schéma — deux dérogations
//! bornées, justifiées en « Complexity Tracking » du plan.

pub mod assignments;
pub mod comments;
pub mod cross;
pub mod dashboard;
pub mod documents;
pub mod organizations;
pub mod people;
pub mod proposals;
pub mod reads;
pub mod reviews;
pub mod scores;
pub mod speakers;
pub mod themes;
pub mod transitions;
