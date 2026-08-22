//! Accès à la base. Une requête par question, jamais de SQL composé.
//!
//! [`cross`] est le **seul** fichier du module qui lise hors de `media`, et il
//! porte la liste exhaustive de ce qu'il lit. Aucun fichier de ce dossier
//! n'écrit hors du schéma du module — c'est la promesse la plus forte du jalon,
//! et un `grep` du quickstart la vérifie.

pub mod assets;
pub mod attachments;
pub mod cross;
pub mod quotas;
pub mod renditions;
