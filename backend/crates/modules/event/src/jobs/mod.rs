//! Travaux différés du module. Le worker les monte sans les connaître : il lit
//! le nom de la tâche dans `platform.jobs` et cherche son gestionnaire.
//!
//! **Un seul, et il se replanifie lui-même** (research.md § R15).
//!
//! **Aucun consommateur d'événement, et c'est un choix.** Ce module ne consomme
//! rien dans ce jalon : il produit `event.programme.published`, que B5
//! consommera, avec sa garde de rejeu.

pub mod autoclose;
