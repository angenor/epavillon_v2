//! Accès à la base. Une requête par question, jamais de SQL composé.
//!
//! [`cross`] porte la liste de ce que le module lit hors de `engagement` ;
//! [`mailings`] y ajoute, pour les envois du back-office, le nom des personnes,
//! de leur organisation et de l'édition (28/10). Aucun fichier de ce dossier
//! n'écrit hors du schéma du module.

pub mod cross;
pub mod delivery;
pub mod mailings;
pub mod notifications;
pub mod preferences;
pub mod reminders;
pub mod rules;
pub mod suppressions;
pub mod templates;
