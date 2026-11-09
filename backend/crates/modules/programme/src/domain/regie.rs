//! Régie du jour (02/11) : l'écran de l'équipe pendant une CdP et ses gestes.
//! Chaque geste s'inscrit au journal avec de quoi le défaire.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::{Date, OffsetDateTime};
use utoipa::ToSchema;
use uuid::Uuid;

/// Délai avant le courriel d'annulation : « Défaire » d'ici là, personne ne
/// reçoit rien.
pub const DELAI_COURRIEL_MINUTES: i64 = 5;
/// Un direct terminé par erreur se reprend pendant ce délai, même réunion Zoom.
pub const REPRISE_MINUTES: i64 = 15;

pub const TAXONOMIE_RAISONS: &str = "session_cancellation_reason";
/// Raisons qui disent les organisateurs absents : la présence suit.
pub const RAISONS_ABSENCE: [&str; 2] = ["no_show", "notified_absence"];

/// Code d'un geste, `programme.control_room_actions.action_code`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RegieActionKind {
    PresenceConfirmed,
    SessionCancelled,
    LiveStarted,
    LiveEnded,
}

impl RegieActionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PresenceConfirmed => "presence_confirmed",
            Self::SessionCancelled => "session_cancelled",
            Self::LiveStarted => "live_started",
            Self::LiveEnded => "live_ended",
        }
    }

    pub fn parse(code: &str) -> Option<Self> {
        match code {
            "presence_confirmed" => Some(Self::PresenceConfirmed),
            "session_cancelled" => Some(Self::SessionCancelled),
            "live_started" => Some(Self::LiveStarted),
            "live_ended" => Some(Self::LiveEnded),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RegieScreen {
    pub event_id: Uuid,
    pub event_title: Value,
    pub event_acronym: Option<String>,
    pub timezone: String,
    pub zone_label: Option<String>,
    pub days: Vec<Date>,
    pub day: Date,
    pub today: Date,
    #[serde(with = "time::serde::rfc3339")]
    pub now: OffsetDateTime,
    /// Un direct qui tourne ailleurs, toutes éditions confondues (règle n° 4).
    pub live_elsewhere: Option<RegieLiveElsewhere>,
    pub sessions: Vec<RegieSession>,
    pub journal: Vec<RegieJournalEntry>,
    pub reasons: Vec<RegieReason>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RegieLiveElsewhere {
    pub session_id: Uuid,
    /// Nul quand la séance est hors du périmètre du lecteur.
    pub title: Option<Value>,
    pub same_event: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RegieReason {
    pub code: String,
    pub label: Value,
    pub description: Option<Value>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RegieSession {
    pub session_id: Uuid,
    pub title: Value,
    #[serde(with = "time::serde::rfc3339")]
    pub starts_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub ends_at: OffsetDateTime,
    /// `programme.session_status`.
    pub status: String,
    pub is_published: bool,
    pub is_streamed: bool,
    pub is_joint: bool,
    pub room_name: Option<Value>,
    pub cover: Option<Value>,
    pub organizations: Vec<RegieOrganization>,
    /// Inscriptions confirmées (`registered`, `attended`).
    pub registrants: i64,
    pub presence: RegiePresence,
    pub meeting: Option<RegieMeeting>,
    pub documents: Vec<RegieDocument>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub live_started_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub live_ended_at: Option<OffsetDateTime>,
    /// Fin de la reprise possible d'un direct terminé, nulle passé ce délai.
    #[serde(with = "time::serde::rfc3339::option")]
    pub resumable_until: Option<OffsetDateTime>,
    pub cancellation: Option<RegieCancellation>,
    /// Le dernier geste de la séance encore défaisable, pour « Défaire ».
    pub last_action_id: Option<Uuid>,
    /// Intervenants en ligne (08/11).
    pub speakers: crate::domain::online_speakers::RegieSpeakerSummary,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RegieOrganization {
    pub id: Uuid,
    pub name: String,
    pub acronym: Option<String>,
    pub logo: Option<Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PresenceState {
    Confirmed,
    Absent,
    Unknown,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RegiePresence {
    pub state: PresenceState,
    #[serde(with = "time::serde::rfc3339::option")]
    pub noted_at: Option<OffsetDateTime>,
    pub noted_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RegieMeeting {
    /// `live.meeting_status`.
    pub status: String,
    /// `live.sync_status`.
    pub sync_status: String,
    /// Créée chez Zoom et programmée ou démarrée : l'animateur peut l'ouvrir.
    pub ready: bool,
    /// Personnes dans la salle, réunion démarrée seulement.
    pub present_count: Option<i64>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RegieDocument {
    pub id: Uuid,
    pub proposal_id: Uuid,
    pub title: Value,
    pub filename: Option<String>,
    pub mime_type: Option<String>,
    pub byte_size: Option<i64>,
    /// Nulle tant que le fichier n'est pas servable (analyse, quarantaine).
    pub url: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub uploaded_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RegieCancellation {
    #[serde(with = "time::serde::rfc3339::option")]
    pub cancelled_at: Option<OffsetDateTime>,
    pub reason_code: Option<String>,
    pub reason_label: Option<Value>,
    /// Note interne, jamais publiée.
    pub note: Option<String>,
    /// Le geste de la régie qui l'a annulée ; nul pour une annulation venue du
    /// dossier, qui ne se rétablit pas d'ici.
    pub action_id: Option<Uuid>,
    pub notice: RegieNoticeStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum NoticeState {
    /// Aucun courriel demandé, ou personne à prévenir.
    None,
    /// Dû à `due_at` ; « Défaire » d'ici là le retire.
    Pending,
    /// Parti : on ne le reprend pas, on envoie le correctif.
    Sent,
    /// Retiré avant tout envoi.
    Withdrawn,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RegieNoticeStatus {
    pub state: NoticeState,
    #[serde(with = "time::serde::rfc3339::option")]
    pub due_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub sent_at: Option<OffsetDateTime>,
    /// Inscrits à prévenir tant que rien n'est parti, prévenus ensuite.
    pub recipients: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum JournalKind {
    PresenceConfirmed,
    SessionCancelled,
    LiveStarted,
    LiveEnded,
    IncidentPublished,
}

impl From<RegieActionKind> for JournalKind {
    fn from(k: RegieActionKind) -> Self {
        match k {
            RegieActionKind::PresenceConfirmed => Self::PresenceConfirmed,
            RegieActionKind::SessionCancelled => Self::SessionCancelled,
            RegieActionKind::LiveStarted => Self::LiveStarted,
            RegieActionKind::LiveEnded => Self::LiveEnded,
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RegieJournalEntry {
    /// Le geste de la régie, ou le message d'incident (`incident_published`).
    pub id: Uuid,
    pub kind: JournalKind,
    pub session_id: Option<Uuid>,
    pub session_title: Option<Value>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub session_starts_at: Option<OffsetDateTime>,
    /// Texte du message d'incident.
    pub incident_message: Option<Value>,
    pub incident_kind_code: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub acted_at: OffsetDateTime,
    pub actor_name: Option<String>,
    pub actor_is_me: bool,
    #[serde(with = "time::serde::rfc3339::option")]
    pub undone_at: Option<OffsetDateTime>,
    pub undone_by: Option<String>,
    /// Le geste se défait encore (pour un direct terminé : dans les quinze minutes).
    pub can_undo: bool,
    pub notice: Option<RegieNoticeStatus>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct RegieCancelPayload {
    pub reason_code: String,
    #[serde(default)]
    pub note: Option<String>,
    pub notify_registrants: bool,
}

#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct RegieUndoPayload {
    /// Rétablir une annulation : écrire « finalement maintenue » à ceux qui ont
    /// reçu le courriel d'annulation.
    #[serde(default)]
    pub notify_registrants: bool,
}

/// Ce que l'écran affiche dans le bandeau « Défaire » après un geste.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RegieGesture {
    pub action_id: Uuid,
    pub kind: RegieActionKind,
    pub session_id: Uuid,
    #[serde(with = "time::serde::rfc3339")]
    pub acted_at: OffsetDateTime,
    /// Annulation : heure du courriel aux inscrits, nulle sans envoi.
    #[serde(with = "time::serde::rfc3339::option")]
    pub notify_at: Option<OffsetDateTime>,
    pub registrants: i64,
    /// Direct terminé : fin de la reprise possible.
    #[serde(with = "time::serde::rfc3339::option")]
    pub resumable_until: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub undone_at: Option<OffsetDateTime>,
}
