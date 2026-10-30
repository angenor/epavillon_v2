//! La page publique d'un fil (journée spéciale ou atelier préparatoire) et le
//! placement d'une activité dans le déroulé (30/10). Les noms des formes sont
//! ceux de `frontend/app/types/programme/track-page.ts`.
//!
//! **Le déroulé fusionne deux natures de ligne** : les activités publiées
//! rattachées au fil (`programme.session_tracks`) et les moments sans activité
//! (`event.programme_track_moments` : accueil, déjeuner, clôture). Les fondre
//! ici, triées par heure, évite à chaque écran de refaire la fusion.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use super::sessions::PublicScheduleRow;

/// L'édition du fil, ce que la page en affiche : `PublicTrackEdition`.
#[derive(Debug, Clone, Serialize)]
pub struct PublicTrackEdition {
    pub id: Uuid,
    pub slug: String,
    pub title: Value,
    pub acronym: Option<String>,
    /// Fuseau de référence : toute heure de la page s'y lit.
    pub timezone: String,
    #[serde(with = "time::serde::rfc3339")]
    pub starts_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub ends_at: OffsetDateTime,
    pub city: Option<String>,
    pub country_name: Option<Value>,
}

/// Le lieu du fil : `PublicTrackVenue`. Celui du fil, à défaut le pavillon de
/// l'édition.
#[derive(Debug, Clone, Serialize)]
pub struct PublicTrackVenue {
    pub id: Uuid,
    pub name: Value,
    pub kind: String,
    pub address: Option<String>,
    pub map_url: Option<String>,
    pub area: Option<Value>,
    pub access_note: Option<Value>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

/// L'encart mis en avant : `PublicTrackFeature`. Nul sans titre.
#[derive(Debug, Clone, Serialize)]
pub struct PublicTrackFeature {
    pub kicker: Option<Value>,
    pub title: Value,
    pub body: Option<Value>,
    pub link_label: Option<Value>,
    /// Adresse absolue, ou chemin du site (« /… »).
    pub link_url: Option<String>,
    /// `AttachedImage | null` : le visuel (couverture du Guide).
    pub visual: Option<Value>,
}

/// Un partenaire : `PublicTrackPartner`.
#[derive(Debug, Clone, Serialize)]
pub struct PublicTrackPartner {
    pub organization_id: Uuid,
    pub name: String,
    pub acronym: Option<String>,
    pub logo: Option<Value>,
    pub role_code: String,
    pub role_label: Value,
}

/// Une partie du déroulé : `PublicTrackPart`. Ses heures sont **lues sur son
/// contenu** (première heure, dernière fin) ; nulles quand elle est vide.
#[derive(Debug, Clone, Serialize)]
pub struct PublicTrackPart {
    pub id: Uuid,
    /// Rang à partir de 1 : « Partie 1 » quand `label` est nul.
    pub number: i32,
    pub label: Option<Value>,
    pub title: Option<Value>,
    pub summary: Option<Value>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub starts_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub ends_at: Option<OffsetDateTime>,
}

/// Une activité du déroulé : ce que porte une ligne `kind: "activity"`.
#[derive(Debug, Clone, Serialize)]
pub struct PublicTrackActivity {
    /// La ligne de la programmation publique, telle quelle : format, statut,
    /// état temporel, couverture, organisation porteuse, diffusion, rediffusion.
    /// Le lien vers sa page se compose de `session.slug` et du slug de l'édition.
    pub session: PublicScheduleRow,
    pub is_highlight: bool,
    /// `PublicSessionSpeaker[]`.
    pub speakers: Vec<Value>,
    /// `SessionOrganization[]`, porteur en tête, logo compris.
    pub organizations: Vec<Value>,
}

/// Un moment sans activité : ce que porte une ligne `kind: "moment"`.
#[derive(Debug, Clone, Serialize)]
pub struct PublicTrackMoment {
    pub id: Uuid,
    pub label: Value,
    pub description: Option<Value>,
}

/// Le contenu d'une ligne du déroulé.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PublicTrackEntryBody {
    Activity { activity: PublicTrackActivity },
    Moment { moment: PublicTrackMoment },
}

/// Une ligne du déroulé : `PublicTrackEntry`.
#[derive(Debug, Clone, Serialize)]
pub struct PublicTrackEntry {
    #[serde(with = "time::serde::rfc3339")]
    pub starts_at: OffsetDateTime,
    /// Nulle pour un moment sans fin annoncée (la clôture).
    #[serde(with = "time::serde::rfc3339::option")]
    pub ends_at: Option<OffsetDateTime>,
    pub part_id: Option<Uuid>,
    #[serde(flatten)]
    pub body: PublicTrackEntryBody,
}

/// Un autre fil publié de l'édition, pour relier les pages :
/// `PublicTrackSummary`.
#[derive(Debug, Clone, Serialize)]
pub struct PublicTrackSummary {
    pub slug: String,
    pub kind: String,
    pub title: Value,
    pub color_hex: Option<String>,
    pub starts_on: Option<Date>,
    pub ends_on: Option<Date>,
    /// Première et dernière heure de son déroulé publié ; nulles s'il est vide.
    #[serde(with = "time::serde::rfc3339::option")]
    pub starts_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub ends_at: Option<OffsetDateTime>,
    /// Nom du lieu du fil, à défaut celui du pavillon.
    pub venue_name: Option<Value>,
    pub is_off_site: bool,
    /// La page qu'on lit.
    pub is_current: bool,
}

/// Un moment sans activité, tel que `/programmations` l'affiche dans le cadre
/// d'une journée : `PublicScheduleTrackMoment`.
#[derive(Debug, Clone, Serialize)]
pub struct PublicScheduleTrackMoment {
    #[serde(with = "time::serde::rfc3339")]
    pub starts_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub ends_at: Option<OffsetDateTime>,
    pub label: Value,
}

/// Un fil publié de l'édition pour `/programmations` : `PublicScheduleTrack`,
/// le résumé (jamais `is_current`) et ses moments dans l'ordre de l'heure.
/// Les activités, elles, sont déjà dans la programmation.
#[derive(Debug, Clone, Serialize)]
pub struct PublicScheduleTrack {
    #[serde(flatten)]
    pub summary: PublicTrackSummary,
    pub moments: Vec<PublicScheduleTrackMoment>,
}

/// Le fil lui-même : `PublicTrackHeader`.
#[derive(Debug, Clone, Serialize)]
pub struct PublicTrackHeader {
    pub id: Uuid,
    pub slug: String,
    pub code: String,
    pub kind: String,
    pub title: Value,
    pub subtitle: Option<Value>,
    pub description: Option<Value>,
    pub tagline: Option<Value>,
    pub starts_on: Option<Date>,
    pub ends_on: Option<Date>,
    pub color_hex: Option<String>,
    /// `ScheduleThemeBadge[]`.
    pub themes: Value,
}

/// La page publique d'un fil, en une réponse : `PublicTrackPage`.
#[derive(Debug, Clone, Serialize)]
pub struct PublicTrackPage {
    pub edition: PublicTrackEdition,
    pub track: PublicTrackHeader,
    pub venue: Option<PublicTrackVenue>,
    /// Le fil se tient ailleurs qu'au pavillon (l'atelier, à l'hôtel).
    pub is_off_site: bool,
    /// `AttachedImage | null` : la photo du bandeau.
    pub cover: Option<Value>,
    /// `AttachedDocument[]` : programme en PDF, Guide des négociations.
    pub documents: Vec<Value>,
    pub feature: Option<PublicTrackFeature>,
    pub target_audiences: Vec<Value>,
    pub support_note: Option<Value>,
    pub partners: Vec<PublicTrackPartner>,
    pub parts: Vec<PublicTrackPart>,
    pub agenda: Vec<PublicTrackEntry>,
    /// Première heure d'activité (à défaut, première ligne du déroulé).
    #[serde(with = "time::serde::rfc3339::option")]
    pub starts_at: Option<OffsetDateTime>,
    /// Dernière fin, ou dernier début d'un moment sans fin.
    #[serde(with = "time::serde::rfc3339::option")]
    pub ends_at: Option<OffsetDateTime>,
    /// Premier moment s'il précède la première activité : « accueil dès 08:00 ».
    #[serde(with = "time::serde::rfc3339::option")]
    pub opens_at: Option<OffsetDateTime>,
    pub other_tracks: Vec<PublicTrackSummary>,
}

/// Placer une activité dans le déroulé d'un fil : `TrackSessionPlacementPayload`.
#[derive(Debug, Clone, Deserialize)]
pub struct TrackSessionPlacementPayload {
    #[serde(default)]
    pub part_id: Option<Uuid>,
    #[serde(default)]
    pub is_highlight: bool,
    #[serde(default)]
    pub sort_order: i16,
}

/// Le rattachement après écriture : `TrackSessionPlacement`.
#[derive(Debug, Clone, Serialize)]
pub struct TrackSessionPlacement {
    pub session_id: Uuid,
    pub track_id: Uuid,
    pub part_id: Option<Uuid>,
    pub is_highlight: bool,
    pub sort_order: i16,
}
