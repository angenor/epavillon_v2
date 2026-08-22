//! Accès à la base. Une requête par question, jamais de SQL composé.
//!
//! [`cross`] est le **seul** fichier du module qui lise hors de `engagement`, et
//! il porte la liste exhaustive de ce qu'il lit. Aucun fichier de ce dossier
//! n'écrit hors du schéma du module.

pub mod cross;
pub mod delivery;
pub mod notifications;
pub mod preferences;
pub mod reminders;
pub mod rules;
pub mod suppressions;
pub mod templates;
