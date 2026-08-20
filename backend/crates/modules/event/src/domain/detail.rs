//! Les six onglets d'une édition, et la composition qui les porte.
//!
//! **Aucun nom de champ n'est renégocié** (FR-003) : leur source unique est
//! `frontend/app/types/admin-events.ts`, § 3.
//!
//! **Une composition, pas douze lectures** (FR-023, research.md § R3). Ouvrir
//! l'onglet « Appel » ne doit pas attendre un aller-retour : l'équipe passe d'un
//! onglet à l'autre sans arrêt en préparant une COP. La contrepartie est
//! assumée — une écriture dans un onglet rafraîchit la composition entière, ce
//! qui garantit que les décomptes des cinq autres restent justes.
//!
//! Les décomptes de séances ne sont **aucune colonne** de ces tables : ils
//! viennent de `programme`, joints par `repo/cross.rs`, et disent ce qu'un
//! retrait déplacerait.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use super::edition::EditionListRow;

/// Une pastille thématique, telle que `reference.term_badges()` la rend —
/// `ScheduleThemeBadge`.
///
/// **Le libellé et la couleur viennent de la base**, jamais d'un fichier de
/// traduction : une thématique se modifie au back-office, ce n'est donc pas une
/// traduction mais une donnée. C'est le défaut n° 1 de la v1, qui les figeait
/// dans le frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeBadge {
    pub code: String,
    pub label: Value,
    pub color: Option<String>,
    pub icon: Option<String>,
}

// -----------------------------------------------------------------------------
// 1. Journées du calendrier
// -----------------------------------------------------------------------------

/// Un jour du calendrier — `EditionDay`, sur `event.event_days`.
#[derive(Debug, Clone, Serialize)]
pub struct EditionDay {
    pub id: Uuid,
    pub day_date: Date,
    pub title: Option<Value>,
    pub slug: Option<String>,
    pub description: Option<Value>,
    pub is_featured: bool,
    pub color_hex: Option<String>,
    pub sort_order: i16,
    pub session_count: i64,
    /// Vrai quand la date sort de la période de l'édition — une soirée
    /// d'ouverture la veille est un cas légitime. **On le signale ; on ne le
    /// supprime pas d'office** (FR-035).
    pub is_outside_period: bool,
}

// -----------------------------------------------------------------------------
// 2. Journées spéciales
// -----------------------------------------------------------------------------

/// Un fil de programmation — `EditionTrack`, sur `event.programme_tracks`.
///
/// **Sa composition ne se fait pas ici** : `session_count` est en lecture seule,
/// le rattachement d'une séance à un fil étant une décision éditoriale prise au
/// planificateur (règle métier n° 7).
#[derive(Debug, Clone, Serialize)]
pub struct EditionTrack {
    pub id: Uuid,
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
    /// Nom du responsable, résolu — l'écran n'affiche pas un identifiant.
    pub curator_name: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub published_at: Option<OffsetDateTime>,
    pub sort_order: i16,
    pub session_count: i64,
    pub themes: Vec<ThemeBadge>,
}

// -----------------------------------------------------------------------------
// 3. Lieux et salles
// -----------------------------------------------------------------------------

/// Une salle — `EditionRoom`, sur `event.rooms`.
///
/// `is_virtual` **n'est pas un détail** : une salle virtuelle accepte des
/// séances simultanées, et `programme.detect_conflicts()` n'y signale aucune
/// double réservation.
#[derive(Debug, Clone, Serialize)]
pub struct EditionRoom {
    pub id: Uuid,
    pub venue_id: Uuid,
    pub name: Value,
    pub code: String,
    pub capacity: Option<i16>,
    pub is_virtual: bool,
    pub has_streaming: bool,
    pub equipment: Vec<String>,
    pub sort_order: i16,
    pub session_count: i64,
}

/// Un lieu et ses salles — `EditionVenue`, sur `event.venues`.
#[derive(Debug, Clone, Serialize)]
pub struct EditionVenue {
    pub id: Uuid,
    pub name: Value,
    pub kind: String,
    pub address: Option<String>,
    pub map_url: Option<String>,
    pub rooms: Vec<EditionRoom>,
}

// -----------------------------------------------------------------------------
// 4. Canaux de diffusion
// -----------------------------------------------------------------------------

/// Un canal — `EditionChannel`, sur `event.broadcast_channels`.
///
/// `event_id` est **nul pour un canal général de la plateforme**, qui n'est pas
/// modifiable depuis une édition. L'onglet les affiche tous deux, comme le
/// front les compose déjà.
#[derive(Debug, Clone, Serialize)]
pub struct EditionChannel {
    pub id: Uuid,
    pub event_id: Option<Uuid>,
    pub code: String,
    pub name: Value,
    pub provider: String,
    pub channel_ref: Option<String>,
    pub locale: Option<String>,
    pub is_default: bool,
    pub is_active: bool,
    pub session_count: i64,
}

// -----------------------------------------------------------------------------
// 5. Appel à propositions et grille
// -----------------------------------------------------------------------------

/// Un critère de la grille — `EditionCriterion`, sur `event.review_criteria`.
#[derive(Debug, Clone, Serialize)]
pub struct EditionCriterion {
    pub id: Option<Uuid>,
    pub code: String,
    pub label: Value,
    pub description: Option<Value>,
    pub max_score: f64,
    pub weight: f64,
    pub is_knockout: bool,
    pub sort_order: i16,
    /// Notes déjà posées sur ce critère : ce qu'une modification de barème
    /// rendrait faux, et ce qui interdit son retrait (research.md § R9).
    pub score_count: i64,
}

/// L'appel de l'édition — `EditionCall`. **Zéro ou un**, jamais un tableau :
/// `ux_calls_one_per_event` tient la cardinalité, pas l'application.
#[derive(Debug, Clone, Serialize)]
pub struct EditionCall {
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
    /// `HH:MM:SS`, en heure **locale de l'édition**. Rendue en texte par la
    /// base : la mettre en forme ici inventerait une seconde écriture de
    /// l'heure.
    pub daily_start_time: String,
    pub daily_end_time: String,
    pub allowed_formats: Vec<String>,
    pub required_reviews: i16,
    pub blind_review: bool,
    pub guidelines_url: Option<String>,

    // -- Dérivés, jamais des colonnes : les trois fonctions du modèle sont
    //    APPELÉES, jamais recalculées ici.
    #[serde(with = "time::serde::rfc3339")]
    pub effective_deadline: OffsetDateTime,
    pub is_open: bool,
    pub max_weighted_score: f64,
    pub proposal_count: i64,
    pub criteria: Vec<EditionCriterion>,
}

// -----------------------------------------------------------------------------
// 6. Comité de sélection
// -----------------------------------------------------------------------------

/// Un membre du comité — `EditionCommitteeMember`, sur `event.call_reviewers`.
///
/// **Cette table dit la composition, pas le droit d'accès** : l'autorisation
/// reste portée par `identity.role_assignments`. `has_review_permission` se
/// contente de dire si la personne le détient vraiment.
#[derive(Debug, Clone, Serialize)]
pub struct EditionCommitteeMember {
    pub person_id: Uuid,
    pub full_name: String,
    pub email: String,
    pub organization_name: Option<String>,
    pub is_lead: bool,
    pub workload_cap: Option<i16>,
    #[serde(with = "time::serde::rfc3339")]
    pub added_at: OffsetDateTime,
    pub assigned_count: i64,
    pub submitted_count: i64,
    pub has_review_permission: bool,
}

/// Une personne que l'on peut désigner — `CommitteeCandidate`.
///
/// **Le critère est une permission, jamais un nom de rôle** : une liste de rôles
/// écrite en dur laisserait de côté le premier rôle ajouté au catalogue.
#[derive(Debug, Clone, Serialize)]
pub struct CommitteeCandidate {
    pub person_id: Uuid,
    pub full_name: String,
    pub email: String,
    pub organization_name: Option<String>,
    pub has_review_permission: bool,
}

// -----------------------------------------------------------------------------
// La composition
// -----------------------------------------------------------------------------

/// La période en dates civiles, **dans le fuseau de l'édition** : ce que les
/// onglets bornent (research.md § R5).
#[derive(Debug, Clone, Serialize)]
pub struct EditionPeriod {
    pub first_day: Date,
    pub last_day: Date,
}

/// Tout l'écran de détail en une réponse — `EditionDetail`.
#[derive(Debug, Clone, Serialize)]
pub struct EditionDetail {
    pub edition: EditionListRow,
    /// Les deux textes longs, portés **ici** et non sur la ligne de liste : un
    /// tableau à huit colonnes n'a pas à charger deux paragraphes par édition.
    pub description: Value,
    pub highlights: Option<Value>,
    pub period: EditionPeriod,
    /// Les trois déclinaisons, telles que `media.attached_image()` les rend
    /// pour les rôles `banner`, `cover` et `thumbnail`. **Les trois clés sont
    /// toujours présentes**, à `null` tant que rien n'a été téléversé : la
    /// boucle d'affichage n'a alors aucune garde à écrire.
    pub images: Value,
    pub days: Vec<EditionDay>,
    pub tracks: Vec<EditionTrack>,
    pub venues: Vec<EditionVenue>,
    pub channels: Vec<EditionChannel>,
    pub call: Option<EditionCall>,
    pub committee: Vec<EditionCommitteeMember>,
    pub curators: Vec<CommitteeCandidate>,
    pub committee_candidates: Vec<CommitteeCandidate>,
    pub available_themes: Vec<ThemeBadge>,
}
