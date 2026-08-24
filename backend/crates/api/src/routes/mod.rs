//! Routes portées par l'API elle-même, et non par un module : la vivacité, la
//! santé d'exploitation, la documentation générée, le référentiel et les
//! drapeaux de fonctionnalité. Aucune n'appartient à un schéma métier, et
//! aucune ne doit disparaître quand un module est démonté.
//!
//! Les deux dernières sont arrivées à la bascule du site : `reference` et
//! `platform` sont des schémas TRANSVERSES — le dépôt d'un dossier y lit ses
//! thématiques, le rattachement à une organisation son type, le routage ses
//! drapeaux —, et les loger dans un crate de module obligerait les cinq autres
//! à en dépendre.

pub mod health;
pub mod platform;
pub mod reference;
