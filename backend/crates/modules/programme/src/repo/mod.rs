//! Requêtes SQLx, un fichier par agrégat. Aucune règle métier ici : le dépôt
//! lit et écrit, le service décide.
//!
//! **Deux régimes, quatre fichiers, et la distinction compte** : `cross/` ne
//! porte que des LECTURES hors schéma — geste ordinaire ; `themes.rs`,
//! `people.rs` et `consents.rs` portent les **trois** ÉCRITURES hors schéma —
//! trois dérogations bornées, justifiées en « Complexity Tracking » du plan.
//! Il n'y en a pas une quatrième, et `T150` le vérifie mécaniquement.

pub mod assignments;
pub mod comments;
pub mod conflicts;
pub mod consents;
pub mod cross;
pub mod dashboard;
pub mod documents;
pub mod forms;
pub mod organizations;
pub mod people;
pub mod planner;
pub mod proposals;
pub mod public_schedule;
pub mod reads;
pub mod registrations;
pub mod reviews;
pub mod scores;
pub mod session_parts;
pub mod sessions;
pub mod speakers;
pub mod themes;
pub mod transitions;
