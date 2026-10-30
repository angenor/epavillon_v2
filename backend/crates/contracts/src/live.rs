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
