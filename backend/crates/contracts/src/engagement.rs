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

/// Le courriel de confirmation d'une inscription à une journée (02/11).
///
/// **Un travail, pas un événement** : sans compte, il porte le jeton en clair du
/// lien de gestion, que la base ne connaît que haché et que l'outbox ne doit
/// jamais voir. Mis en file par le module Programmation dans la transaction de
/// l'inscription ; servi par le module Engagement, qui le déclare secret.
pub const SEND_DAY_REGISTRATION: &str = "engagement.send_day_registration";

/// La file des courriels, celle que le module Engagement écoute.
pub const EMAIL_QUEUE: &str = "email";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DayRegistrationEmail {
    pub track_registration_id: uuid::Uuid,
    /// Jeton en clair du lien de gestion, sans compte seulement.
    #[serde(default)]
    pub token: Option<String>,
    /// Vrai : l'inscription existait déjà, on renvoie le lien.
    #[serde(default)]
    pub resent: bool,
}

/// Le lien Zoom personnel d'un intervenant en ligne (08/11). Mis en file par le
/// module Live dès que la place existe chez Zoom, sous la clé
/// `<place>:<instant de la demande>`.
pub const SEND_SPEAKER_ZOOM_LINK: &str = "engagement.send_speaker_zoom_link";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeakerZoomLinkEmail {
    /// Nul seulement pour un aperçu.
    pub participant_id: Option<uuid::Uuid>,
    pub session_id: uuid::Uuid,
    /// Aperçu pour validation : à cette seule adresse, sans copie, avec un lien
    /// et un nom d'exemple. Posé à la main, jamais par le code.
    #[serde(default)]
    pub preview_to: Option<String>,
}

/// Le lien d'inscription des intervenants en ligne, au porteur de l'activité
/// (08/11). Mis en file par la régie (module Programmation) ou par le balayage
/// de l'avant-veille.
pub const SEND_SPEAKER_SIGNUP_LINK: &str = "engagement.send_speaker_signup_link";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeakerSignupLinkEmail {
    pub session_id: uuid::Uuid,
    /// Aperçu pour validation : à cette seule adresse, sans copie.
    #[serde(default)]
    pub preview_to: Option<String>,
}
