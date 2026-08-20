//! La file des doublons présumés, et les deux fiches d'une paire.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use super::ids::{DuplicatePairId, OrganizationId, PersonId};

/// Une des deux fiches d'une paire, **réduite à ce qui permet de trancher** sans
/// ouvrir les deux fiches : qui l'a créée, quand, ce qu'elle porte déjà.
#[derive(Debug, Clone, Serialize)]
pub struct DuplicateSide {
    pub organization_id: OrganizationId,
    pub legal_name: String,
    pub acronym: Option<String>,
    pub slug: String,
    pub status: String,
    pub organization_type_code: String,
    pub organization_type_label: Option<Value>,
    pub country_id: Option<Uuid>,
    pub country_name: Option<Value>,
    pub city: Option<String>,
    pub website: Option<String>,
    pub contact_email: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub verified_at: Option<OffsetDateTime>,
    pub trust_score: i16,
    pub member_count: i64,
    pub proposal_count: i64,
    pub session_count: i64,
    /// Domaines déclarés, vérifiés ou non : c'est là que se lit le motif partagé.
    pub domains: Vec<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    pub created_by_name: Option<String>,
}

/// Une paire de `org.duplicate_candidates`.
///
/// `left_id` est **toujours** inférieur à `right_id`
/// (`ck_duplicate_candidates_ordered`). Cet ordre est technique et ne dit **rien**
/// de qui doit absorber qui : c'est la désignation de l'écran de fusion, et elle
/// n'a aucun rapport avec la place d'une fiche dans la paire.
#[derive(Debug, Clone, Serialize)]
pub struct DuplicatePair {
    pub id: DuplicatePairId,
    pub score: f64,
    pub reasons: Vec<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub detected_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub reviewed_at: Option<OffsetDateTime>,
    pub reviewed_by: Option<PersonId>,
    pub reviewed_by_name: Option<String>,
    pub decision: Option<String>,
    pub left: Box<DuplicateSide>,
    pub right: Box<DuplicateSide>,
}

/// **La file, et ce qui en est sorti.**
///
/// Les paires arbitrées ne disparaissent pas : « ce ne sont pas des doublons »
/// se reprend, et une paire écartée par erreur serait autrement introuvable.
#[derive(Debug, Clone, Serialize)]
pub struct DuplicateQueueScreen {
    /// Non arbitrées, triées par similarité **décroissante**.
    pub pending: Vec<DuplicatePair>,
    /// Déjà tranchées : fusionnées, écartées, reportées.
    pub settled: Vec<DuplicatePair>,
}

/// Décision portée sur une paire depuis la file. La fusion, elle, a son écran —
/// et c'est la fonction de base qui marque alors la paire.
#[derive(Debug, Clone, Deserialize)]
pub struct DuplicateDecision {
    #[serde(default)]
    pub pair_id: Option<Uuid>,
    /// `distinct` retire la paire de la file ; `deferred` la met de côté — ou l'y
    /// **ramène** si elle en était déjà sortie.
    pub decision: String,
    #[serde(default)]
    pub note: Option<String>,
}

impl DuplicateDecision {
    /// `merged` ne se pose jamais depuis la file : c'est
    /// `org.merge_organizations()` qui l'écrit, et elle seule.
    pub fn est_recevable(&self) -> bool {
        matches!(self.decision.as_str(), "distinct" | "deferred")
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum DuplicateDecisionOutcome {
    Recorded { pair: Box<DuplicatePair> },
    NotFound,
}
