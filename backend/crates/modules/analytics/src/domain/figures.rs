//! Zone 2 du tableau de bord — **les chiffres**.
//!
//! De la consultation : entonnoir, courbes, répartitions. On y revient pour
//! comprendre une tendance, pas pour agir dans la minute.

use serde::Serialize;
use serde_json::Value;
use time::{Date, OffsetDateTime};

/// Un point de courbe : un jour, une valeur, un cumul.
///
/// **Le jour est une DATE CIVILE** (`AAAA-MM-JJ`), découpée en UTC par la
/// projection. Ce n'est pas un instant : le convertir puis le reformater dans un
/// fuseau le décalerait d'un jour la moitié de l'année.
#[derive(Debug, Clone, Serialize)]
pub struct TrendPoint {
    pub jour: Date,
    pub valeur: i64,
    pub cumul: i64,
    /// Moyenne mobile sur sept jours, portée par les deux projections
    /// quotidiennes du modèle. **Nulle sur les premiers jours de la série**, où
    /// la fenêtre n'est pas complète : présenter la moyenne de trois jours comme
    /// une moyenne de sept ferait croire à un démarrage lent là où il n'y a
    /// qu'un début de série.
    pub moyenne_7j: Option<f64>,
}

/// Une part de répartition — un pays, une thématique.
///
/// `label` est un texte **multilingue venu de la base** : il se résout à
/// l'affichage et ne passe jamais par un fichier i18n. `color` vient de
/// `reference.taxonomy_terms.color_hex` — figer ces couleurs dans la feuille de
/// style est le défaut n° 1 de la v1.
#[derive(Debug, Clone, Serialize)]
pub struct BreakdownSlice {
    /// Clé stable : code ISO du pays, code du terme de taxonomie.
    pub key: String,
    pub label: Value,
    pub color: Option<String>,
    pub count: i64,
    /// Part du total, entre 0 et 1. **Calculée à la source** pour ne pas
    /// diverger de la somme affichée à côté.
    pub share: f64,
}

/// Les six indicateurs de tête.
///
/// Chacun se trace à **une colonne du modèle**, sans exception. Aucun n'est
/// calculé à l'écran : un chiffre dérivé côté site finit par ne plus
/// correspondre au graphique d'à côté.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DashboardKpiKey {
    Submissions,
    Deadline,
    ReviewProgress,
    AcceptanceRate,
    Scheduled,
    Registrations,
}

/// Couleur d'un indicateur — **un état, jamais une décoration**.
///
/// `warning` pour ce qui demande attention, `danger` pour ce qui est dépassé,
/// `success` pour ce qui est fait. `neutral` est le cas ordinaire : la plupart
/// des chiffres ne sont ni bons ni mauvais, et les colorer tous revient à n'en
/// signaler aucun.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DashboardKpiTone {
    Neutral,
    Accent,
    Success,
    Warning,
    Danger,
}

/// **`value`, `out_of` et `delta` sont des `Option` : `null` n'est jamais zéro.**
///
/// C'est la distinction qui coûte le plus cher ici. Un taux d'acceptation nul
/// signifie qu'aucun dossier n'a été tranché ; affiché « 0 % », il ferait passer
/// un comité qui n'a pas commencé pour un comité qui a tout refusé.
#[derive(Debug, Clone, Serialize)]
pub struct DashboardKpi {
    pub key: DashboardKpiKey,
    pub value: Option<f64>,
    /// Second membre d'un rapport (« 18 sur 24 »). Nul quand il n'y en a pas.
    pub out_of: Option<f64>,
    /// Variation des sept derniers jours face aux sept précédents, en unités.
    /// **Nulle sous quatorze jours de série** : une comparaison sur une semaine
    /// tronquée est un artefact, pas une tendance.
    pub delta: Option<f64>,
    /// Instant associé : l'échéance, pour la carte qui la décompte.
    #[serde(with = "time::serde::rfc3339::option")]
    pub at: Option<OffsetDateTime>,
    /// Série courte de l'étincelle — vingt et un derniers jours de la projection
    /// concernée. Vide quand l'indicateur n'est pas une série.
    pub spark: Vec<i64>,
    pub tone: DashboardKpiTone,
}

/// La ligne de `analytics.mv_proposal_funnel` de l'appel de l'édition.
///
/// **Les colonnes de la projection, telles quelles.** Les renommer en anglais
/// aurait produit deux vocabulaires pour une même donnée.
#[derive(Debug, Clone, Serialize)]
pub struct ProposalFunnelRow {
    pub event_id: uuid::Uuid,
    pub cle_appel: uuid::Uuid,
    pub call_id: Option<uuid::Uuid>,
    pub evenement: Option<String>,
    pub edition_year: i16,
    pub statut_evenement: String,
    pub code_appel: Option<String>,
    pub appel: Option<String>,
    pub statut_appel: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub appel_ouvre_le: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub appel_ferme_le: Option<OffsetDateTime>,
    pub required_reviews: Option<i16>,
    pub total: i64,
    pub brouillons: i64,
    pub deposees: i64,
    pub en_attente_affectation: i64,
    pub en_revue: i64,
    pub modifications_demandees: i64,
    pub acceptees: i64,
    pub rejetees: i64,
    pub retirees: i64,
    pub annulees: i64,
    pub decidees: i64,
    pub en_instance: i64,
    pub taux_acceptation: Option<f64>,
    pub taux_acceptation_sur_depots: Option<f64>,
    pub organisations_distinctes: i64,
    pub note_moyenne: Option<f64>,
    pub delai_median_decision_heures: Option<f64>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub premier_depot: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub dernier_depot: Option<OffsetDateTime>,
    pub sessions_programmees: i64,
}

/// Les chiffres de l'édition sélectionnée.
///
/// `funnel` peut être **nul** : une édition sans appel ni dépôt n'a pas
/// d'entonnoir, et un entonnoir à zéro partout serait un graphique qui ment sur
/// sa propre existence.
#[derive(Debug, Clone, Serialize)]
pub struct DashboardFigures {
    pub kpis: Vec<DashboardKpi>,
    pub funnel: Option<ProposalFunnelRow>,
    pub submissions: Vec<TrendPoint>,
    pub registrations: Vec<TrendPoint>,
    /// **L'échéance qui fait foi** — `event.effective_deadline()`, donc la
    /// prolongation si elle existe. Sans ce repère sur la courbe des dépôts,
    /// l'effet de dernière minute (60 % des dépôts sur les 48 dernières heures,
    /// mesuré en v1) est illisible : on voit un pic sans savoir devant quoi.
    #[serde(with = "time::serde::rfc3339::option")]
    pub deadline: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub call_opens_at: Option<OffsetDateTime>,
    pub by_country: Vec<BreakdownSlice>,
    pub by_theme: Vec<BreakdownSlice>,
    /// Âge des projections matérialisées — `max(finished_at)` sur les
    /// rafraîchissements **réussis**. Nul quand aucun n'a jamais abouti. Affiché
    /// sans détour : un chiffre matérialisé présenté comme instantané est un
    /// chiffre faux.
    #[serde(with = "time::serde::rfc3339::option")]
    pub refreshed_at: Option<OffsetDateTime>,
}
