//! Les travaux différés du module.
//!
//! **Aucun n'est mis en file par ce module.**
//! `engagement.schedule_session_reminders()` enfile un travail par rappel,
//! passe les lignes à `queued` **et** émet son annonce, le tout dans le même
//! geste. Ce dossier ne fait que **consommer** ce que la base a posé : un
//! `kernel::jobs::enqueue` de plus produirait deux courriels par rappel, et le
//! doublon ne se verrait qu'en production.

//! **Deux exceptions, bornées.** Le courriel d'un avis générique
//! (`notification_email`) est enfilé par le consommateur des avis, dans la
//! transaction qui écrit l'avis, quand l'émetteur le demande. Le travail des
//! partitions, lui : rien ne
//! l'enfile, et son seul `enqueue` est celui qui pose **sa propre occurrence
//! suivante**. C'est le patron de B1, écrit trois fois dans le dépôt avant lui.

pub mod notification_email;
pub mod partitions;
pub mod send_mailing;
pub mod send_reminder;
