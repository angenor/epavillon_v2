//! Requêtes SQLx, un fichier par agrégat. Aucune règle métier ici : le dépôt
//! lit et écrit, le service décide.

pub mod calls;
pub mod channels;
pub mod committee;
pub mod criteria;
pub mod cross;
pub mod cross_page;
pub mod days;
pub mod editions;
pub mod public;
pub mod themes;
pub mod track_page;
pub mod tracks;
pub mod venues;
