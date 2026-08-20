//! Les six onglets d'une édition : ce que leurs formulaires envoient, et la
//! **réponse commune** de toutes leurs écritures.
//!
//! **Aucun nom de champ n'est renégocié** : leur source unique est
//! `frontend/app/types/admin-events.ts`, § 3.
//!
//! Toutes ces écritures rendent la **composition entière recalculée** (FR-024).
//! La contrepartie est assumée : un enregistrement dans un onglet rafraîchit les
//! cinq autres, ce qui garantit que leurs décomptes restent justes. Retirer une
//! salle change le décompte de l'onglet des journées ; ne rendre que la salle
//! laisserait l'écran mentir jusqu'au prochain rechargement.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::Date;
use uuid::Uuid;

use super::detail::EditionDetail;

/// Ce qu'une écriture d'onglet peut refuser — `EditionTabResult.error_code`.
///
/// **`Deactivated` n'est pas un refus.** Il accompagne `ok: true` et dit qu'un
/// canal a été désactivé plutôt que supprimé, parce qu'il a servi (research.md
/// § R7). C'est le seul endroit du module où ce champ ne signale pas une erreur.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TabErrorCode {
    /// Objet inexistant **ou hors périmètre** — indiscernables.
    NotFound,
    /// Champ obligatoire manquant, ou mal formé.
    Required,
    /// `ck_programme_tracks_period`.
    Period,
    /// `ux_programme_tracks_code`, `ux_rooms_code`, `ux_broadcast_channels_code`.
    CodeTaken,
    /// `ux_programme_tracks_slug`, `ux_event_days_slug`.
    SlugTaken,
    /// `rooms_capacity_check`.
    Capacity,
    /// Modification d'un canal **général de la plateforme** depuis une édition.
    PlatformChannel,
    /// **Succès** : le canal a servi, il est désactivé et non supprimé.
    Deactivated,
}

/// La réponse commune des écritures d'onglet — `EditionTabResult`.
#[derive(Debug, Clone, Serialize)]
pub struct EditionTabResult {
    pub ok: bool,
    /// **La composition entière**, recalculée. `null` sur un refus.
    pub detail: Option<EditionDetail>,
    /// Séances détachées par l'écriture — salle retirée, journée supprimée,
    /// canal désactivé. Ces clés sont `ON DELETE SET NULL` : la séance survit et
    /// perd son rattachement. **Compté AVANT** l'ordre de suppression, sans quoi
    /// le chiffre serait toujours zéro (research.md § R8).
    pub sessions_detached: i64,
    pub error_code: Option<TabErrorCode>,
}

impl EditionTabResult {
    pub fn refuse(code: TabErrorCode) -> Self {
        Self {
            ok: false,
            detail: None,
            sessions_detached: 0,
            error_code: Some(code),
        }
    }

    pub fn reussite(detail: EditionDetail, sessions_detached: i64) -> Self {
        Self {
            ok: true,
            detail: Some(detail),
            sessions_detached,
            error_code: None,
        }
    }

    /// Le canal désactivé : **un succès**, avec son mot pour le dire.
    pub fn desactive(detail: EditionDetail, sessions_detached: i64) -> Self {
        Self {
            error_code: Some(TabErrorCode::Deactivated),
            ..Self::reussite(detail, sessions_detached)
        }
    }
}

// -----------------------------------------------------------------------------
// Journées du calendrier
// -----------------------------------------------------------------------------

/// L'habillage **éditorial** d'une journée — `EditionDayPayload`.
///
/// **La date n'y est pas, et c'est délibéré** : une journée du calendrier tient
/// sa date de la période de l'édition, et la déplacer ferait un doublon ou un
/// trou. La génération crée les dates ; ce formulaire les habille.
#[derive(Debug, Clone, Deserialize)]
pub struct EditionDayPayload {
    /// **Ignoré** : l'identifiant fait foi dans l'adresse, jamais dans le corps.
    #[serde(default)]
    pub id: Option<Uuid>,
    #[serde(default)]
    pub title: Option<Value>,
    #[serde(default)]
    pub slug: Option<String>,
    #[serde(default)]
    pub description: Option<Value>,
    pub is_featured: bool,
    #[serde(default)]
    pub color_hex: Option<String>,
}

/// Ce que la génération **ferait**, sans rien écrire — `DayGenerationPlan`.
#[derive(Debug, Clone, Serialize)]
pub struct DayGenerationPlan {
    /// Dates de la période qui n'ont pas encore de journée.
    pub to_create: Vec<Date>,
    /// Journées hors période, **avec ce qu'un retrait détacherait**.
    pub to_review: Vec<DayToReview>,
    /// Journées déjà en place et dans la période : rien à faire.
    pub unchanged: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct DayToReview {
    pub id: Uuid,
    pub day_date: Date,
    pub session_count: i64,
}

/// Ce que la génération demande, et **rien d'autre**.
///
/// Le plan n'y figure pas : il est **recalculé dans la transaction d'écriture**
/// (research.md § R4). Entre l'affichage du plan et le clic, quelqu'un peut
/// avoir modifié la période — écrire d'après le plan renvoyé, c'est supprimer
/// une journée qui vient d'y entrer, avec les séances qu'elle porte.
#[derive(Debug, Clone, Deserialize)]
pub struct DayGenerationRequest {
    #[serde(default)]
    pub remove_outside_period: bool,
}

// -----------------------------------------------------------------------------
// Fils de programmation
// -----------------------------------------------------------------------------

/// Un fil — `EditionTrackPayload`.
#[derive(Debug, Clone, Deserialize)]
pub struct EditionTrackPayload {
    #[serde(default)]
    pub id: Option<Uuid>,
    /// **Vérifié à la création, ignoré à la modification** : l'édition d'un fil
    /// existant vient de son ascendance en base (research.md § R2).
    pub event_id: Uuid,
    pub code: String,
    pub slug: String,
    pub kind: String,
    pub title: Value,
    #[serde(default)]
    pub subtitle: Option<Value>,
    #[serde(default)]
    pub description: Option<Value>,
    #[serde(default)]
    pub starts_on: Option<Date>,
    #[serde(default)]
    pub ends_on: Option<Date>,
    #[serde(default)]
    pub color_hex: Option<String>,
    #[serde(default)]
    pub curated_by: Option<Uuid>,
    /// Ouvrir ou refermer la **page publique** du fil : c'est le même
    /// enregistrement, jamais un geste séparé.
    pub is_published: bool,
    pub sort_order: i16,
    /// Thématiques du fil, par leur **code de taxonomie**. Ce sont des données,
    /// pas des traductions : elles vivent dans `reference.taxonomy_terms` et se
    /// modifient au back-office.
    #[serde(default)]
    pub theme_codes: Vec<String>,
}

// -----------------------------------------------------------------------------
// Lieux et salles
// -----------------------------------------------------------------------------

/// Un lieu — `EditionVenuePayload`.
#[derive(Debug, Clone, Deserialize)]
pub struct EditionVenuePayload {
    #[serde(default)]
    pub id: Option<Uuid>,
    pub event_id: Uuid,
    pub name: Value,
    pub kind: String,
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default)]
    pub map_url: Option<String>,
}

/// Une salle — `EditionRoomPayload`.
///
/// **`is_virtual` est écrit tel quel, jamais déduit du mode de participation.**
/// Une salle virtuelle accepte les créneaux simultanés, et la détection de
/// conflits n'y signale aucune double réservation : la déduire du mode de
/// l'édition ferait taire, sur une COP hybride, le conflit de gravité haute que
/// l'équipe doit absolument voir. C'est le **lieu** qui dit l'occupation.
#[derive(Debug, Clone, Deserialize)]
pub struct EditionRoomPayload {
    #[serde(default)]
    pub id: Option<Uuid>,
    pub venue_id: Uuid,
    pub name: Value,
    pub code: String,
    #[serde(default)]
    pub capacity: Option<i16>,
    pub is_virtual: bool,
    pub has_streaming: bool,
    #[serde(default)]
    pub equipment: Vec<String>,
    pub sort_order: i16,
}

// -----------------------------------------------------------------------------
// Canaux de diffusion
// -----------------------------------------------------------------------------

/// Un canal — `EditionChannelPayload`.
#[derive(Debug, Clone, Deserialize)]
pub struct EditionChannelPayload {
    #[serde(default)]
    pub id: Option<Uuid>,
    pub event_id: Uuid,
    pub code: String,
    pub name: Value,
    pub provider: String,
    #[serde(default)]
    pub channel_ref: Option<String>,
    #[serde(default)]
    pub locale: Option<String>,
    /// Poser le défaut **retire le précédent, dans la même transaction**
    /// (research.md § R6) : l'index n'est pas différable, et l'ordre inverse
    /// échoue.
    pub is_default: bool,
    pub is_active: bool,
}

// -----------------------------------------------------------------------------
// Comité de sélection
// -----------------------------------------------------------------------------

/// Un siège tel que le formulaire l'envoie.
#[derive(Debug, Clone, Deserialize)]
pub struct CommitteeSeat {
    pub person_id: Uuid,
    pub is_lead: bool,
    /// Plafond **indicatif** : rien ne l'applique, et le module ne l'applique
    /// pas non plus.
    #[serde(default)]
    pub workload_cap: Option<i16>,
}

/// La composition entière — `CommitteePayload`.
///
/// **Ajouts, retraits et plafonds d'un seul geste** : l'écran envoie la liste
/// complète, et ce qui n'y figure plus est retiré. Un ajout et un retrait
/// séparés laisseraient exister un comité intermédiaire que personne n'a voulu.
#[derive(Debug, Clone, Deserialize)]
pub struct CommitteePayload {
    /// **Ignoré** : l'identifiant de l'appel fait foi dans l'adresse.
    #[serde(default)]
    pub call_id: Option<Uuid>,
    #[serde(default)]
    pub members: Vec<CommitteeSeat>,
}

/// Un membre retiré qui portait encore des dossiers — `removed_with_assignments`.
///
/// Le retrait **n'annule aucune revue déjà rendue** : elles restent au dossier,
/// comme le veut l'historique opposable du modèle. Mais quelqu'un doit reprendre
/// le reste, et un retrait silencieux laisse des dossiers sans lecteur à trois
/// jours de la décision.
#[derive(Debug, Clone, Serialize)]
pub struct RemovedWithAssignments {
    pub full_name: String,
    pub assigned_count: i64,
}

/// La réponse de l'enregistrement du comité — `CommitteeSaveResult`.
#[derive(Debug, Clone, Serialize)]
pub struct CommitteeSaveResult {
    pub ok: bool,
    pub members: Vec<super::detail::EditionCommitteeMember>,
    pub removed_with_assignments: Vec<RemovedWithAssignments>,
}
