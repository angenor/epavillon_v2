//! Événements du module `live`.
//!
//! **Émis par la base, jamais par le crate** : le déclencheur
//! `live.tg_meetings_follow_status()` et `live.announce_meeting_start()`
//! (`080_live.sql`, § 2 bis). Leur charge utile appartient au SQL, comme celle
//! des séances (`programme.rs`) : `session_id`, `event_id`, `occurrence`, et
//! `started_at` ou `ended_at`. Le consommateur lit les champs un par un.

pub const AGGREGATE_SCHEMA: &str = "live";
pub const AGGREGATE_MEETING: &str = "meeting";

/// La réunion tourne et sa séance commence dans moins d'un quart d'heure (ou a
/// commencé). Une répétition plus tôt n'est annoncée qu'à début − 15 min.
pub const MEETING_STARTED: &str = "live.meeting.started";
/// La réunion est terminée chez Zoom.
pub const MEETING_ENDED: &str = "live.meeting.ended";

/// Les places Zoom des intervenants d'une activité (08/11) : intervenants en
/// ligne, `075_programme_sessions.sql` § 2 bis.
///
/// **Un travail, pas un événement** : mis en file par le module Programmation
/// (inscription d'un intervenant, renvoi des liens par la régie) et par le
/// balayage du module Engagement (la veille) ; servi par le module Live, qui
/// crée la place de chaque intervenant qui a une adresse et demande le
/// courriel de son lien.
pub const SPEAKER_PLACES: &str = "live.speaker_places";

/// La file que le module Live écoute, servie seulement si Zoom est configuré.
pub const LIVE_QUEUE: &str = "live";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SpeakerPlaces {
    pub session_id: uuid::Uuid,
    /// Les intervenants visés ; `None` : tous ceux de l'activité.
    pub speaker_ids: Option<Vec<uuid::Uuid>>,
    /// Envoi d'office de la veille : un intervenant dont le lien a déjà été
    /// demandé n'en reçoit pas un second.
    pub auto: bool,
}
