//! La page publique d'un fil, vue du back-office : ce que l'équipe règle au-delà
//! du fil lui-même (060 § 4 ter). Les noms des formes sont ceux de
//! `frontend/app/types/event/track-page.ts`.
//!
//! Une écriture rend toujours les réglages **entiers**, recalculés, comme les
//! onglets d'une édition : retirer une partie remet ses moments et ses
//! activités hors partie, et l'écran doit le voir sans recharger.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

/// Une partie du déroulé : `TrackPart`.
#[derive(Debug, Clone, Serialize)]
pub struct TrackPart {
    pub id: Uuid,
    pub track_id: Uuid,
    /// Nom court (« Matin », « S'inspirer ») ; nul, l'écran écrit « Partie N ».
    pub label: Option<Value>,
    /// Intitulé long.
    pub title: Option<Value>,
    /// Version d'une ligne, pour le sommaire de la page.
    pub summary: Option<Value>,
    pub sort_order: i16,
}

/// Un moment sans activité : `TrackMoment`.
#[derive(Debug, Clone, Serialize)]
pub struct TrackMoment {
    pub id: Uuid,
    pub track_id: Uuid,
    pub part_id: Option<Uuid>,
    #[serde(with = "time::serde::rfc3339")]
    pub starts_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub ends_at: Option<OffsetDateTime>,
    pub label: Value,
    pub description: Option<Value>,
}

/// Un partenaire du fil : `TrackPartner`. Nom, sigle et logo résolus : l'écran
/// n'affiche pas un identifiant.
#[derive(Debug, Clone, Serialize)]
pub struct TrackPartner {
    pub organization_id: Uuid,
    pub name: String,
    pub acronym: Option<String>,
    /// `AttachedImage | null`.
    pub logo: Option<Value>,
    pub role_code: String,
    /// Libellé du rôle, donnée de `reference.taxonomy_terms`.
    pub role_label: Value,
    pub sort_order: i16,
}

/// Une activité rattachée au fil, telle que l'éditeur de la page la place :
/// `TrackCompositionEntry`.
#[derive(Debug, Clone, Serialize)]
pub struct TrackCompositionEntry {
    pub session_id: Uuid,
    pub slug: String,
    pub title: Value,
    #[serde(with = "time::serde::rfc3339")]
    pub starts_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub ends_at: OffsetDateTime,
    pub status: String,
    /// Une activité non publiée reste dans la composition mais n'apparaît pas
    /// sur la page publique.
    pub is_published: bool,
    pub part_id: Option<Uuid>,
    pub is_highlight: bool,
    pub sort_order: i16,
}

/// Les réglages de la page d'un fil : `TrackPageSettings`.
#[derive(Debug, Clone, Serialize)]
pub struct TrackPageSettings {
    pub track_id: Uuid,
    pub event_id: Uuid,
    /// Nul : le pavillon de l'édition.
    pub venue_id: Option<Uuid>,
    pub tagline: Option<Value>,
    pub target_audiences: Vec<Value>,
    pub support_note: Option<Value>,
    pub feature_kicker: Option<Value>,
    pub feature_title: Option<Value>,
    pub feature_body: Option<Value>,
    pub feature_link_label: Option<Value>,
    pub feature_link_url: Option<String>,
    pub parts: Vec<TrackPart>,
    pub moments: Vec<TrackMoment>,
    pub partners: Vec<TrackPartner>,
    pub sessions: Vec<TrackCompositionEntry>,
}

/// Un partenaire envoyé par l'éditeur, élément de `TrackPagePayload.partners`.
/// L'ordre de la liste est l'ordre d'affichage.
#[derive(Debug, Clone, Deserialize)]
pub struct TrackPartnerPayload {
    pub organization_id: Uuid,
    /// Code de la taxonomie `track_partner_role` ; `partner` par défaut.
    #[serde(default = "role_par_defaut")]
    pub role_code: String,
}

fn role_par_defaut() -> String {
    "partner".to_owned()
}

/// Ce que l'éditeur de la page écrit sur le fil : `TrackPagePayload`.
///
/// **Écriture totale** de ces champs et de la liste des partenaires ; les
/// parties et les moments ont leurs propres routes, parce qu'ils ont une
/// identité que d'autres lignes désignent.
#[derive(Debug, Clone, Deserialize)]
pub struct TrackPagePayload {
    #[serde(default)]
    pub venue_id: Option<Uuid>,
    #[serde(default)]
    pub tagline: Option<Value>,
    #[serde(default)]
    pub target_audiences: Vec<Value>,
    #[serde(default)]
    pub support_note: Option<Value>,
    #[serde(default)]
    pub feature_kicker: Option<Value>,
    #[serde(default)]
    pub feature_title: Option<Value>,
    #[serde(default)]
    pub feature_body: Option<Value>,
    #[serde(default)]
    pub feature_link_label: Option<Value>,
    #[serde(default)]
    pub feature_link_url: Option<String>,
    #[serde(default)]
    pub partners: Vec<TrackPartnerPayload>,
}

/// Une partie : `TrackPartPayload`.
#[derive(Debug, Clone, Deserialize)]
pub struct TrackPartPayload {
    #[serde(default)]
    pub label: Option<Value>,
    #[serde(default)]
    pub title: Option<Value>,
    #[serde(default)]
    pub summary: Option<Value>,
    #[serde(default)]
    pub sort_order: i16,
}

/// Un moment : `TrackMomentPayload`.
#[derive(Debug, Clone, Deserialize)]
pub struct TrackMomentPayload {
    #[serde(default)]
    pub part_id: Option<Uuid>,
    #[serde(with = "time::serde::rfc3339")]
    pub starts_at: OffsetDateTime,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub ends_at: Option<OffsetDateTime>,
    pub label: Value,
    #[serde(default)]
    pub description: Option<Value>,
}
