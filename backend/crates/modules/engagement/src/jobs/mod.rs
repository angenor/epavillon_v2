//! Les travaux différés du module.
//!
//! **Aucun n'est mis en file par ce module.**
//! `engagement.schedule_session_reminders()` enfile un travail par rappel,
//! passe les lignes à `queued` **et** émet son annonce, le tout dans le même
//! geste. Ce dossier ne fait que **consommer** ce que la base a posé : un
//! `kernel::jobs::enqueue` de plus produirait deux courriels par rappel, et le
//! doublon ne se verrait qu'en production.

//! **Le travail des partitions est l'exception, et elle est bornée** : rien ne
//! l'enfile, et son seul `enqueue` est celui qui pose **sa propre occurrence
//! suivante**. C'est le patron de B1, écrit trois fois dans le dépôt avant lui.

pub mod partitions;
pub mod send_reminder;
