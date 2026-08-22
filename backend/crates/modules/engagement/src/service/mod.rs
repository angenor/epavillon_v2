//! Les services : ce que les routes appellent, et ce que les tests éprouvent.
//!
//! [`rules`] arrive **avant** le calendrier et les envois, contre l'ordre des
//! priorités : rien ne sème de règle de rappel, et sans écriture ni l'un ni les
//! autres ne se démontrent autrement qu'en posant une ligne à la main en SQL.
//!
//! [`schedule`] lit ce qui va partir — **un nombre de destinataires, jamais un
//! nom** —, et sa garde est l'adhésion active, pas un périmètre : une
//! organisation n'administre rien.

pub mod compose;
pub mod deliverability;
pub mod notifications;
pub mod rules;
pub mod schedule;
pub mod templates;
