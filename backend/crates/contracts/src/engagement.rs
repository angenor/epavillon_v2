//! Événements du module `engagement`.
//!
//! # Un seul, et l'adresse n'y est pas en clair
//!
//! `engagement.reminders.scheduled` est émis par
//! `engagement.schedule_session_reminders()`, qui met aussi **un travail par
//! rappel** en file. Le service ne le redouble pas : il produirait deux
//! courriels par rappel, et le doublon ne se verrait qu'en production. Son
//! absence de ce fichier est la décision.
//!
//! Reste celui-ci. Une adresse qui sort du circuit est une information
//! d'exploitation : elle explique pourquoi une personne cesse de recevoir ses
//! avis. Mais **l'outbox est durable, indexée, relayée et tracée** — une adresse
//! électronique est une donnée personnelle, et elle n'y voyage donc que hachée.

use serde::{Deserialize, Serialize};

pub const AGGREGATE_SCHEMA: &str = "engagement";
pub const AGGREGATE_EMAIL_SUPPRESSION: &str = "email_suppression";

pub const EMAIL_SUPPRESSED: &str = "engagement.email.suppressed";

/// L'adresse est **hachée**, jamais en clair : qui détient déjà l'adresse peut
/// vérifier qu'elle est concernée, personne ne peut la lire dans l'outbox.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailSuppressed {
    /// SHA-256 de l'adresse en minuscules, en hexadécimal.
    pub email_hash: String,
    /// Valeur de `engagement.suppression_reason`, en texte.
    pub reason: String,
}
