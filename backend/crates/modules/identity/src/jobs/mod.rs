//! Travaux différés du module. Le worker les monte sans les connaître : il lit
//! le nom de la tâche dans `platform.jobs` et cherche son gestionnaire.

pub mod emails;
pub mod purge;
