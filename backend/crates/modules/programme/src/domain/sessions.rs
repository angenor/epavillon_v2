//! Les formes d'une séance — planificateur, écran public, espace organisation.
//!
//! Aucune requête ici : ce fichier ne porte que ce que les trois écrans lisent,
//! et l'état qu'une séance peut prendre. Les lectures vivent dans `repo/`.
//!
//! # Une séance porte tout ce qu'un bloc affiche, déjà joint
//!
//! C'est la règle de `v_public_schedule`, transposée au planificateur : sans
//! elle, chaque bloc du calendrier coûterait une requête pour son organisation,
//! une pour sa note et une pour ses thématiques. Le contrat du front la nomme
//! en toutes lettres, et l'écran est livré depuis le 18/08.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

/// Les six états, tels que `programme.session_status` les nomme.
///
/// L'énumération traverse la frontière SQL en `text`, patron des quatre modules
/// livrés. **`planned` et `scheduled` ne sont pas synonymes** : le modèle
/// commente le premier « créneau pressenti, non public » et le second
/// « programmé et publié ». C'est la publication qui fait passer de l'un à
/// l'autre (research.md § R12).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Planned,
    Scheduled,
    Live,
    Completed,
    Postponed,
    Cancelled,
}

impl SessionStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::Scheduled => "scheduled",
            Self::Live => "live",
            Self::Completed => "completed",
            Self::Postponed => "postponed",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn from_db(valeur: &str) -> Option<Self> {
        Some(match valeur {
            "planned" => Self::Planned,
            "scheduled" => Self::Scheduled,
            "live" => Self::Live,
            "completed" => Self::Completed,
            "postponed" => Self::Postponed,
            "cancelled" => Self::Cancelled,
            _ => return None,
        })
    }
}

/// Une séance telle que le planificateur la manipule — `PlannerSession`.
///
/// **`room_id` nul range la séance au panneau « à placer »**, et c'est la seule
/// chose qui l'y range : une séance sans salle existe, elle n'est simplement
/// pas encore installée.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PlannerSession {
    pub id: Uuid,
    pub event_id: Uuid,
    /// Nul quand l'IFDD programme directement, sans passer par l'appel.
    pub proposal_id: Option<Uuid>,
    pub event_day_id: Option<Uuid>,
    pub title: serde_json::Value,
    pub slug: String,
    pub summary: Option<serde_json::Value>,
    pub status: String,
    pub format: String,
    #[serde(with = "time::serde::rfc3339")]
    pub starts_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub ends_at: OffsetDateTime,
    pub timezone: String,

    pub room_id: Option<Uuid>,
    pub room_name: Option<serde_json::Value>,
    /// Dérivée de `event.rooms.is_virtual` : une salle virtuelle n'occupe pas
    /// le stand. **Jamais saisissable** — voir `domain/derived.rs`.
    pub enforce_room_exclusivity: bool,
    pub location_note: Option<serde_json::Value>,

    pub organization_id: Option<Uuid>,
    pub organization_name: Option<String>,
    pub organization_acronym: Option<String>,
    /// Code ISO 3166-1 alpha-2 : il situe l'organisation aussi sûrement que son
    /// nom.
    pub organization_country_code: Option<String>,

    /// Numéro lisible du dossier d'origine, nul sans dossier.
    pub reference_code: Option<String>,
    /// Note consolidée du dossier — c'est par elle que le panneau se trie.
    pub average_score: Option<f64>,
    pub requested_duration_minutes: Option<i32>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub preferred_start_at: Option<OffsetDateTime>,
    pub scheduling_constraints: Option<String>,

    pub is_streamed: bool,
    pub broadcast_channel_id: Option<Uuid>,

    /// Journées spéciales auxquelles l'équipe l'a **rattachée** — jamais déduit
    /// des dates (règle métier n° 7).
    pub track_ids: Vec<Uuid>,
    /// Thématiques du dossier, libellé et couleur venus de la base.
    pub themes: serde_json::Value,
    pub speaker_count: i64,
    #[serde(with = "time::serde::rfc3339::option")]
    pub published_at: Option<OffsetDateTime>,
}

/// Salle offerte au placement — `PlannerRoom`.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PlannerRoom {
    pub id: Uuid,
    pub name: serde_json::Value,
    pub code: String,
    pub capacity: Option<i32>,
    /// Une salle virtuelle accepte les créneaux simultanés, sans conflit.
    pub is_virtual: bool,
    pub has_streaming: bool,
    pub sort_order: i16,
}

/// Journée du calendrier — `PlannerDay`.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PlannerDay {
    pub id: Uuid,
    pub day_date: time::Date,
    pub title: Option<serde_json::Value>,
    pub is_featured: bool,
    pub color_hex: Option<String>,
}

/// Journée spéciale offerte au rattachement — `PlannerTrack`.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PlannerTrack {
    pub id: Uuid,
    pub title: serde_json::Value,
    pub kind: String,
    pub color_hex: Option<String>,
    pub starts_on: Option<time::Date>,
    pub ends_on: Option<time::Date>,
}

/// Canal de diffusion — `PlannerChannel`. Ressource réservable : un seul direct
/// à la fois, tous événements confondus (règle métier n° 4).
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PlannerChannel {
    pub id: Uuid,
    pub name: serde_json::Value,
    pub provider: String,
    pub is_default: bool,
}

/// Un chevauchement recensé par `programme.detect_conflicts()` —
/// `ScheduleConflict`.
///
/// **Aucun ne refuse une écriture.** Ils alimentent le bandeau d'alerte, et le
/// seul garde-fou dur est la publication du programme.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ScheduleConflict {
    pub severity: String,
    pub conflict_kind: String,
    pub subject_id: Option<Uuid>,
    pub subject_label: Option<String>,
    pub session_a: Uuid,
    pub session_a_title: Option<String>,
    pub session_b: Uuid,
    pub session_b_title: Option<String>,
    /// Intersection des deux créneaux, dans la représentation textuelle de
    /// PostgreSQL — le contrat du front déclare une chaîne (R26).
    pub overlap: Option<String>,
}

/// Tout l'écran du planificateur, en une réponse — `PlannerScreen`.
///
/// **Les conflits en font partie et ne sont pas un second appel** : une grille
/// affichée avant de savoir ce qui s'y chevauche montre, pendant une seconde,
/// une programmation qui a l'air saine.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PlannerScreen {
    pub event_id: Uuid,
    pub event_title: serde_json::Value,
    /// Fuseau de l'**édition** : c'est lui qui place les blocs, jamais celui du
    /// poste.
    pub timezone: String,
    /// Nom de la ville hôte — « heure de Belém ».
    pub zone_label: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub programme_published_at: Option<OffsetDateTime>,

    pub days: Vec<PlannerDay>,
    pub rooms: Vec<PlannerRoom>,
    pub tracks: Vec<PlannerTrack>,
    pub channels: Vec<PlannerChannel>,

    /// Séances installées dans une salle : les blocs du calendrier.
    pub placed: Vec<PlannerSession>,
    /// Séances retenues sans salle : le panneau latéral, et son compteur.
    pub unplaced: Vec<PlannerSession>,

    pub conflicts: Vec<ScheduleConflict>,
}

/// La réponse commune aux trois écritures du planificateur —
/// `PlannerMutationResult`.
///
/// **La séance ET les conflits de toute l'édition, toujours** : déplacer un bloc
/// peut résoudre le conflit d'un autre bloc à l'autre bout de la semaine, et ne
/// rendre que la séance modifiée laisserait le bandeau afficher un conflit qui
/// n'existe plus.
///
/// Ce type ne porte **aucun discriminant de refus**, et c'est le contrat le plus
/// important du module : aucun chevauchement ne produit d'erreur, à aucun
/// statut.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PlannerMutationResult {
    pub session: PlannerSession,
    pub conflicts: Vec<ScheduleConflict>,
}

/// Une séance vue depuis l'espace de l'organisation — `TrackedSession`.
///
/// **Trois nombres, jamais un nom.** L'organisation sait combien de personnes
/// viendront ; qui elles sont ne la regarde pas (écart n° 36).
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TrackedSession {
    pub session: serde_json::Value,
    pub room: Option<serde_json::Value>,
    /// Inscriptions **confirmées** : `registered` et `attended`, exactement le
    /// prédicat de la vue publique et du contrôle de jauge. Trois définitions
    /// du même mot produiraient trois chiffres, et c'est l'organisation qui
    /// s'en apercevrait.
    pub registered_count: i64,
    pub waitlisted_count: i64,
    pub capacity: Option<i32>,
    /// `ReminderSlot[]` — une ligne par (décalage, canal), **un nombre de
    /// destinataires et jamais un nom**. Elle vient de
    /// `engagement.session_reminder_schedule()`, la même fonction que sert la
    /// lecture par séance : deux agrégations divergeraient en silence (B6,
    /// écart n° 108 refermé). Vide quand aucune règle ne s'applique — jamais
    /// absente.
    pub reminders: Vec<serde_json::Value>,
}

/// Une ligne de `programme.v_public_schedule` — `PublicScheduleRow`.
///
/// **Une ligne = un bloc du calendrier public.** La vue ne retient que les
/// séances dont `published_at` est renseigné, et elle porte déjà tout ce que la
/// carte affiche : salle, organisation et son pays, journées spéciales,
/// thématiques avec leur libellé et leur couleur, image de couverture, état
/// temporel calculé en base. Chaque colonne manquante coûterait une requête par
/// écran, ou un renoncement d'affichage — les deux se sont produits en v1.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PublicScheduleRow {
    pub id: Uuid,
    pub event_id: Uuid,
    pub event_day_id: Option<Uuid>,
    pub proposal_id: Option<Uuid>,
    pub slug: String,
    pub title: serde_json::Value,
    pub summary: Option<serde_json::Value>,
    #[serde(with = "time::serde::rfc3339")]
    pub starts_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub ends_at: OffsetDateTime,
    pub timezone: String,
    pub format: String,
    pub status: String,
    pub room_id: Option<Uuid>,
    pub room_name: Option<serde_json::Value>,
    pub organization_id: Option<Uuid>,
    pub organization_name: Option<String>,
    pub organization_acronym: Option<String>,
    pub organization_country_code: Option<String>,
    pub organization_country: Option<serde_json::Value>,
    pub is_streamed: bool,
    pub broadcast_channel_id: Option<Uuid>,
    pub capacity: Option<i32>,
    pub tracks: serde_json::Value,
    /// Couverture de la séance, **à défaut celle du dossier d'origine**. Le repli
    /// est la règle, pas une commodité : une organisation joint son image au
    /// dépôt, et personne ne revient en téléverser une seconde après
    /// l'acceptation. Résolu **en base** par `media.attached_image()`.
    pub cover: Option<serde_json::Value>,
    /// `upcoming`, `ongoing`, `past`, `cancelled`, `postponed` — calculé en base,
    /// une fois, plutôt que dans chaque composant.
    pub temporal_state: String,
    pub registered_count: i64,
    /// Codes de la taxonomie, **pour filtrer**.
    pub theme_codes: Vec<String>,
    /// Les mêmes thématiques **pour afficher** — libellé traduit et couleur.
    pub themes: serde_json::Value,
}
