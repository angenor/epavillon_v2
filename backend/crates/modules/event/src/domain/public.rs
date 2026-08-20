//! Ce que le public lit — les formes servies **sans session**.
//!
//! Elles sont celles des tables du modèle, telles que `frontend/app/types/event/`
//! les déclare : `EventEdition`, `EventSeries`, `EventDay`, `ProgrammeTrack`,
//! `Venue`, `Room`, `BroadcastChannel`, `CallForProposals`. **Aucun nom de champ
//! n'est renégocié.**
//!
//! L'édition publique en porte davantage, de façon **additive** : sa série et
//! son pays résolus, ses **trois déclinaisons d'image**, son état temporel, son
//! appel résolu et le volume de son programme publié. Tout cela vient de
//! `event.v_public_editions` et de `programme.v_edition_stats`, jointes par la
//! gauche — l'écart n° 25 se referme là, sans une ligne écrite pour lui
//! (research.md § R16).

use serde::Serialize;
use serde_json::Value;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

/// Une édition publique — `EventEdition`, augmentée de ce que la vue résout.
///
/// **Le critère de publicité vit dans la vue** : ni brouillon, ni annulée. Il
/// n'est pas recopié côté code (FR-084, écart n° 26). Une édition **annoncée**
/// dont le programme n'est pas publié en fait partie : sa page existe, elle
/// annonce ses échéances, et c'est précisément là qu'on dépose un dossier.
#[derive(Debug, Clone, Serialize)]
pub struct PublicEdition {
    pub id: Uuid,
    pub slug: String,
    pub title: Value,
    pub description: Value,
    pub acronym: Option<String>,
    pub edition_label: Option<String>,
    pub edition_year: i16,
    pub status: String,
    pub participation_mode: String,
    pub timezone: String,
    #[serde(with = "time::serde::rfc3339")]
    pub starts_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub ends_at: OffsetDateTime,
    pub has_pavilion: bool,
    #[serde(with = "time::serde::rfc3339::option")]
    pub programme_published_at: Option<OffsetDateTime>,
    pub highlights: Option<Value>,

    // -- Colonnes de la table que la vue ne porte pas, et dont la forme du
    //    contrat a besoin. Elles viennent de la même requête.
    pub country_id: Option<Uuid>,
    pub city: Option<String>,
    pub address: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub created_by: Option<Uuid>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,

    // -- Résolutions de la vue, additives ------------------------------------
    pub series_id: Option<Uuid>,
    pub series_kind: Option<String>,
    pub series_name: Option<Value>,
    pub series_slug: Option<String>,
    pub country_code: Option<String>,
    pub country_name: Option<Value>,

    /// **Les trois déclinaisons, et non une seule recadrée** — 32:9 pour le
    /// bandeau, 16:9 pour la carte, 1:1 pour la liste dense. Chacune est nulle
    /// tant que rien n'a été téléversé pour ce rôle, et c'est le cas courant :
    /// une édition sans image reste publique.
    pub banner: Option<Value>,
    pub cover: Option<Value>,
    pub thumbnail: Option<Value>,

    /// `upcoming`, `ongoing` ou `past` — **le vocabulaire de la vue**, le même
    /// que `programme.v_public_schedule`. Deux écrans qui parlent du même temps
    /// doivent employer les mêmes mots.
    pub temporal_state: String,

    pub call_id: Option<Uuid>,
    pub call_status: Option<String>,
    pub call_is_open: Option<bool>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub call_deadline: Option<OffsetDateTime>,

    pub theme_codes: Vec<String>,
    pub themes: Value,

    // -- Volume du programme publié — `programme.v_edition_stats`, jointe PAR
    //    LA GAUCHE : la vue ne porte que les éditions ayant au moins une séance
    //    publiée, et une jointure stricte ferait disparaître de l'historique
    //    toute édition annoncée. C'est la leçon de B2.
    pub published_session_count: i64,
    pub streamed_session_count: i64,
    pub organization_count: i64,
    #[serde(with = "time::serde::rfc3339::option")]
    pub programme_starts_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub programme_ends_at: Option<OffsetDateTime>,
}

/// Une série — `EventSeries`, augmentée de son **décompte d'éditions**.
#[derive(Debug, Clone, Serialize)]
pub struct PublicSeries {
    pub id: Uuid,
    pub code: String,
    pub kind: String,
    pub name: Value,
    pub description: Option<Value>,
    pub slug: String,
    pub track_code: Option<String>,
    pub organizer_organization_id: Option<Uuid>,
    pub is_active: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    /// Ce qui distingue une série vive d'une coquille.
    pub edition_count: i64,
}

/// Un jour du calendrier — `EventDay`.
#[derive(Debug, Clone, Serialize)]
pub struct PublicDay {
    pub id: Uuid,
    pub event_id: Uuid,
    pub day_date: Date,
    pub title: Option<Value>,
    pub slug: Option<String>,
    pub description: Option<Value>,
    pub is_featured: bool,
    pub color_hex: Option<String>,
    pub sort_order: i16,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

/// Un fil de programmation — `ProgrammeTrack`. **Publiés seulement** : un fil
/// sans page ouverte n'existe pas pour le public.
#[derive(Debug, Clone, Serialize)]
pub struct PublicTrack {
    pub id: Uuid,
    pub event_id: Uuid,
    pub code: String,
    pub slug: String,
    pub kind: String,
    pub title: Value,
    pub subtitle: Option<Value>,
    pub description: Option<Value>,
    pub starts_on: Option<Date>,
    pub ends_on: Option<Date>,
    pub color_hex: Option<String>,
    pub curated_by: Option<Uuid>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub published_at: Option<OffsetDateTime>,
    pub sort_order: i16,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

/// Un lieu — `Venue`.
#[derive(Debug, Clone, Serialize)]
pub struct PublicVenue {
    pub id: Uuid,
    pub event_id: Uuid,
    pub name: Value,
    pub kind: String,
    pub address: Option<String>,
    pub map_url: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

/// Une salle — `Room`.
#[derive(Debug, Clone, Serialize)]
pub struct PublicRoom {
    pub id: Uuid,
    pub venue_id: Uuid,
    pub name: Value,
    pub code: String,
    pub capacity: Option<i16>,
    pub is_virtual: bool,
    pub has_streaming: bool,
    pub equipment: Vec<String>,
    pub sort_order: i16,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

/// Un canal de diffusion — `BroadcastChannel`.
#[derive(Debug, Clone, Serialize)]
pub struct PublicChannel {
    pub id: Uuid,
    pub event_id: Option<Uuid>,
    pub code: String,
    pub name: Value,
    pub provider: String,
    pub channel_ref: Option<String>,
    pub locale: Option<String>,
    pub is_default: bool,
    pub is_active: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

/// L'appel d'une édition — `CallForProposals`. **Zéro ou un**, jamais un
/// tableau ; l'annulé est exclu.
#[derive(Debug, Clone, Serialize)]
pub struct PublicCall {
    pub id: Uuid,
    pub event_id: Uuid,
    pub code: String,
    pub title: Value,
    pub description: Option<Value>,
    pub status: String,
    #[serde(with = "time::serde::rfc3339")]
    pub opens_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub closes_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub extended_until: Option<OffsetDateTime>,
    pub results_expected_at: Option<Date>,
    pub max_proposals_per_organization: Option<i16>,
    pub requires_verified_organization: bool,
    pub min_speakers: i16,
    pub max_speakers: i16,
    pub default_duration_minutes: i16,
    pub min_duration_minutes: i16,
    pub max_duration_minutes: i16,
    pub daily_start_time: String,
    pub daily_end_time: String,
    pub allowed_formats: Vec<String>,
    pub required_reviews: i16,
    pub blind_review: bool,
    pub guidelines_url: Option<String>,
    pub created_by: Option<Uuid>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}
