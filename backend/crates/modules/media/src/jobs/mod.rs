//! Les travaux différés du module.
//!
//! **Aucun n'est mis en file par ce module.** `media.tg_enqueue_processing()`
//! enfile le traitement dès l'insertion de l'objet, et émet l'annonce de dépôt
//! dans le même geste : ce dossier ne fait que **consommer** ce que la base a
//! posé. Un `kernel::jobs::enqueue` de plus produirait deux traitements par
//! fichier, et le doublon ne se verrait qu'en production.
//!
//! **Les deux travaux récurrents sont l'exception, et elle est bornée** : la
//! purge et la réconciliation n'ont aucun déclencheur pour les enfiler, et leur
//! seul `enqueue` est celui qui pose **leur propre occurrence suivante**. C'est
//! le patron de B1, écrit trois fois dans le dépôt avant celui-ci.

pub mod process;
pub mod purge;
pub mod reconcile;
