//! Ce que le back-office de la vitrine échange — les formes de
//! `frontend/app/types/admin-showcase.ts`, **sans qu'aucun nom soit renégocié**.
//!
//! # L'état de diffusion n'est pas le statut
//!
//! `status` dit ce que l'éditeur a décidé ; `broadcast_state` dit ce que le
//! public voit **maintenant**. Une diapositive publiée dont la fenêtre s'ouvre
//! la semaine prochaine est `scheduled`, une autre dont la fenêtre est close est
//! `expired` : les deux sont pourtant `published`. Sans cette distinction, la
//! liste du back-office affiche « publié » à côté d'un contenu que personne ne
//! voit — c'est le défaut qui laissait survivre les annonces périmées en v1.
//!
//! # Le refus de validation est une RÉPONSE, pas une erreur HTTP
//!
//! `ShowcaseWriteResult { ok: false, errors }` sort en 200 : le contrat du site
//! le prévoit, et l'écran pose chaque erreur sur son champ. Les statuts d'erreur
//! restent pour ce que le contrat n'exprime pas — session absente, périmètre,
//! introuvable.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::showcase::ShowcaseRow;

/// État réel de diffusion — dérivé du statut ET de la fenêtre.
pub fn etat_de_diffusion(
    status: &str,
    starts_at: Option<OffsetDateTime>,
    ends_at: Option<OffsetDateTime>,
    maintenant: OffsetDateTime,
) -> &'static str {
    match status {
        "draft" => "draft",
        "archived" => "archived",
        _ => {
            if starts_at.is_some_and(|debut| debut > maintenant) {
                "scheduled"
            } else if ends_at.is_some_and(|fin| fin <= maintenant) {
                "expired"
            } else {
                "live"
            }
        }
    }
}

/// Une ligne du tableau dense de `/admin/vitrine`.
#[derive(Debug, Clone, Serialize)]
pub struct ShowcaseListRow {
    pub id: Uuid,
    pub placement: String,
    pub status: String,
    pub broadcast_state: String,
    pub sort_order: i16,

    pub nature_code: String,
    pub nature_label: Option<Value>,
    pub nature_color: Option<String>,
    pub nature_icon: Option<String>,

    pub title: Value,

    pub author_name: Option<String>,
    pub author_title: Option<Value>,
    pub organization_name: Option<String>,
    pub organization_acronym: Option<String>,
    pub country_name: Option<Value>,

    pub event_id: Option<Uuid>,
    pub event_title: Option<Value>,
    pub event_slug: Option<String>,
    pub session_id: Option<Uuid>,
    pub session_title: Option<Value>,

    pub thumbnail: Option<Value>,
    pub background_image: Option<Value>,
    /// Vrai quand un objet vidéo **prêt** est rattaché. Un objet en traitement
    /// ne compte pas : le bandeau se rabattrait sur l'image, et la liste doit
    /// dire ce que le public voit.
    pub has_video: bool,
    pub background_color_hex: Option<String>,

    #[serde(with = "time::serde::rfc3339::option")]
    pub starts_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub ends_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub published_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,

    /// Première / dernière de son emplacement : les boutons d'ordre s'en
    /// désactivent, calculés là où l'ordre est connu.
    pub is_first: bool,
    pub is_last: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ShowcaseNatureOption {
    pub code: String,
    pub label: Value,
    pub color: Option<String>,
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ShowcaseEventOption {
    pub id: Uuid,
    pub title: Value,
    pub acronym: Option<String>,
    pub edition_year: i16,
    pub slug: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ShowcaseListScreen {
    pub rows: Vec<ShowcaseListRow>,
    /// Un compte par emplacement. Un seul depuis le 24/08, mais la forme reste
    /// indexée : l'écran la lit par clé, et un second emplacement s'ajouterait
    /// sans changer le contrat.
    pub counts: serde_json::Map<String, Value>,
    pub broadcast_counts: serde_json::Map<String, Value>,
    pub natures: Vec<ShowcaseNatureOption>,
    pub events: Vec<ShowcaseEventOption>,
    pub is_global_scope: bool,
}

/// Les valeurs saisissables — une par colonne éditable, plus les thématiques.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ShowcaseFormValues {
    pub id: Option<Uuid>,
    pub placement: String,
    pub status: String,
    pub nature_code: String,
    pub sort_order: i16,

    pub title: Value,
    pub quote: Option<Value>,
    pub body: Option<Value>,

    pub person_id: Option<Uuid>,
    pub author_name: Option<String>,
    pub author_title: Option<Value>,

    pub organization_id: Option<Uuid>,
    pub organization_label: Option<String>,

    pub country_id: Option<Uuid>,

    pub event_id: Option<Uuid>,
    pub session_id: Option<Uuid>,

    pub link_url: Option<String>,
    pub link_label: Option<Value>,

    pub background_color_hex: Option<String>,

    #[serde(with = "time::serde::rfc3339::option", default)]
    pub starts_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option", default)]
    pub ends_at: Option<OffsetDateTime>,

    #[serde(default)]
    pub theme_codes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ShowcaseSessionOption {
    pub id: Uuid,
    pub event_id: Uuid,
    pub title: Value,
    #[serde(with = "time::serde::rfc3339")]
    pub starts_at: OffsetDateTime,
    pub timezone: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ShowcaseOrganizationOption {
    pub id: Uuid,
    pub legal_name: String,
    pub acronym: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ShowcasePersonOption {
    pub id: Uuid,
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ShowcaseCountryOption {
    pub id: Uuid,
    pub iso2: String,
    pub name: Value,
}

/// Un emplacement de média : la contrainte du modèle, et ce qui y est rattaché.
#[derive(Debug, Clone, Serialize)]
pub struct ShowcaseMediaSlot {
    pub role: String,
    pub label: Value,
    pub allowed_mime_prefixes: Vec<String>,
    pub max_byte_size: i64,
    pub current: Option<Value>,
    /// Vrai quand un objet existe pour ce rôle mais n'est pas encore servi.
    /// `current` est alors nul, et l'écran doit dire « la vidéo arrive »
    /// plutôt que d'afficher un emplacement vide.
    pub is_pending: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ShowcaseFormScreen {
    pub values: ShowcaseFormValues,
    /// L'aperçu, dans le contrat **exact** du bandeau public : le même
    /// composant rend l'aperçu et la vitrine.
    pub preview: ShowcaseRow,
    pub natures: Vec<ShowcaseNatureOption>,
    pub events: Vec<ShowcaseEventOption>,
    pub sessions: Vec<ShowcaseSessionOption>,
    pub organizations: Vec<ShowcaseOrganizationOption>,
    pub people: Vec<ShowcasePersonOption>,
    pub countries: Vec<ShowcaseCountryOption>,
    pub available_themes: Value,
    pub media: Vec<ShowcaseMediaSlot>,
    pub is_global_scope: bool,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ShowcaseStatusPayload {
    pub id: Uuid,
    pub status: String,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ShowcaseReorderPayload {
    pub id: Uuid,
    pub direction: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ShowcaseValidationError {
    pub field: String,
    pub code: String,
}

impl ShowcaseValidationError {
    pub fn new(field: &str, code: &str) -> Self {
        Self {
            field: field.to_owned(),
            code: code.to_owned(),
        }
    }
}

/// Le retour de toute écriture.
///
/// `placement_rows` n'est renseigné que par ce qui touche à l'ORDRE — un
/// déplacement change au moins deux lignes, et rendre la seule ligne déplacée
/// laisserait sa voisine afficher un rang faux.
#[derive(Debug, Clone, Serialize)]
pub struct ShowcaseWriteResult {
    pub ok: bool,
    pub errors: Vec<ShowcaseValidationError>,
    pub row: Option<ShowcaseListRow>,
    pub placement_rows: Option<Vec<ShowcaseListRow>>,
}

impl ShowcaseWriteResult {
    pub fn refuse(errors: Vec<ShowcaseValidationError>) -> Self {
        Self {
            ok: false,
            errors,
            row: None,
            placement_rows: None,
        }
    }
}
