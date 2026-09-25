//! Les sessions officielles d'une édition, telles que l'application les lit —
//! toute la COP en une réponse (R9). Coupé : aucune session, jamais une liste
//! périmée (principe XII).

use serde::Serialize;
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

/// `OfficialSessions` — ce que `GET /negotiation/sessions?edition=` rend.
#[derive(Debug, Clone, Serialize)]
pub struct OfficialSessions {
    pub edition: EditionServie,
    pub official_programme_url: String,
    pub state: EtatServi,
    pub cut_reason: Option<MotifDeCoupure>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub failing_since: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub read_at: Option<OffsetDateTime>,
    /// Hors de l'empreinte : il change à chaque lecture.
    #[serde(with = "time::serde::rfc3339")]
    pub server_time: OffsetDateTime,
    pub sessions: Vec<OfficialSession>,
}

impl OfficialSessions {
    /// L'empreinte du corps, heure du serveur exclue.
    pub fn empreinte(&self) -> String {
        let mut corps = serde_json::to_value(self).unwrap_or(Value::Null);
        if let Some(objet) = corps.as_object_mut() {
            objet.remove("server_time");
        }
        kernel::empreinte::de(&corps.to_string())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct EditionServie {
    pub slug: String,
    pub timezone: String,
    pub city: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EtatServi {
    Serving,
    Cut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MotifDeCoupure {
    /// Import éteint, ou jamais lu.
    Disabled,
    Unreachable,
}

/// `OfficialSession`.
#[derive(Debug, Clone, Serialize)]
pub struct OfficialSession {
    pub id: Uuid,
    pub title_en: String,
    /// Traduction automatique ; nulle = anglais seul.
    pub title_fr: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub start_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub end_at: Option<OffsetDateTime>,
    pub venue: Option<String>,
    pub previous: Option<Precedent>,
    #[serde(rename = "type")]
    pub meeting_type: Option<TypeDeSession>,
    pub group: Option<Groupe>,
    pub theme: Option<String>,
    pub agenda_item: Option<PointDuJour>,
    pub open_access: Option<bool>,
    pub status: StatutDeSession,
    pub cancelled: Option<Annulation>,
    pub source_url: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub read_at: OffsetDateTime,
}

/// « Déplacée » : la valeur d'avant, champ par champ.
#[derive(Debug, Clone, Serialize)]
pub struct Precedent {
    #[serde(with = "time::serde::rfc3339")]
    pub start_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub end_at: Option<OffsetDateTime>,
    pub venue: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub changed_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize)]
pub struct TypeDeSession {
    pub code: String,
    pub label: Value,
    pub term_en: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Groupe {
    pub code: String,
    pub label: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct PointDuJour {
    pub code: String,
    pub title: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StatutDeSession {
    Scheduled,
    Cancelled,
}

#[derive(Debug, Clone, Serialize)]
pub struct Annulation {
    #[serde(with = "time::serde::rfc3339")]
    pub at: OffsetDateTime,
    /// `source`, `postponed` ou `removed`.
    pub reason: String,
}
