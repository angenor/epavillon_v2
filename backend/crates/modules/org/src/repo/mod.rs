//! Requêtes SQLx, un fichier par agrégat. Aucune règle métier ici : le dépôt
//! lit et écrit, le service décide.

pub mod admin_detail;
pub mod admin_list;
pub mod domains;
pub mod duplicates;
pub mod memberships;
pub mod merge;
pub mod merge_counts;
pub mod names;
pub mod organizations;
pub mod search;
