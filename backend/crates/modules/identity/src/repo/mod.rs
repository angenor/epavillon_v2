//! Requêtes SQLx, un fichier par agrégat. Aucune règle métier ici : le repo
//! lit et écrit, le service décide.

pub mod accounts;
pub mod admin_users;
pub mod people;
pub mod privacy;
pub mod rbac;
pub mod sessions;
