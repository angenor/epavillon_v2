//! Travaux différés du module : les deux courriels de décision, et la purge des
//! essais de code.
//!
//! **Aucun consommateur d'événement ici**, et c'est un choix : les deux
//! courriels sont mis en file *dans la transaction qui les rend nécessaires*.
//! Si elle est annulée, le travail ne naît pas. Un consommateur ne se
//! justifierait que pour un effet appartenant à un **autre** module — il n'y en
//! a aucun à cette étape.

pub mod emails;
pub mod extract;
pub mod purge;
