//! Ce que « Mon accès », le verrou et le parcours d'entrée lisent — **une seule
//! forme, une seule route**.
//!
//! # L'ÉTAT NE VIT DANS AUCUNE COLONNE
//!
//! `state` se dérive du RBAC : `identity.role_assignments` fait foi, et lui
//! seul. Aucune des tables de ce module n'en porte la moindre copie — ni
//! `invitation_code_uses`, ni `network_memberships` — parce que deux vérités
//! divergent toujours un jour, et que ce jour-là c'est l'écran qui mentira.
//!
//! # RIEN ICI NE NOMME NI NE SUPPOSE UN GENRE
//!
//! SC-006. L'appartenance au réseau vient du code utilisé, et de rien d'autre :
//! aucun champ de personne ne la commande, aucune réponse ne la déduit.

use kernel::auth::ScopeType;
use serde::Serialize;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::admission::AdmissionMode;

/// Les cinq états, tels que l'écran les dit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AccessState {
    Visitor,
    Pending,
    Granted,
    Rejected,
    /// Accès **retiré** par un administrateur. Jamais une demande : une demande
    /// se dit « annulée » (FR-026).
    Revoked,
}

/// Ce que l'accès ouvre : une COP nommée, ou Guide Négo en entier.
#[derive(Debug, Clone, Serialize)]
pub struct AccessScopeView {
    #[serde(rename = "type")]
    pub kind: ScopeType,
    pub id: Option<Uuid>,
    /// Le nom de l'espace, **résolu par `platform.t()`** dans la requête.
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GrantedAccess {
    pub scope: AccessScopeView,
    #[serde(with = "time::serde::rfc3339")]
    pub granted_at: OffsetDateTime,
    /// Libellé du code par lequel l'accès est venu. Nul quand un administrateur
    /// l'a attribué directement.
    pub source_code_label: Option<String>,
}

/// Une appartenance de réseau. **Elle n'ouvre aucun droit** à cette étape
/// (FR-014) : elle sert les canaux réservés à venir et les chiffres des
/// bailleurs.
#[derive(Debug, Clone, Serialize)]
pub struct NetworkView {
    pub code: String,
    pub label: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestStatus {
    Pending,
    Approved,
    Rejected,
    Cancelled,
}

impl RequestStatus {
    pub fn from_db(valeur: &str) -> Option<Self> {
        match valeur {
            "pending" => Some(Self::Pending),
            "approved" => Some(Self::Approved),
            "rejected" => Some(Self::Rejected),
            "cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AccessRequestView {
    pub id: Uuid,
    pub status: RequestStatus,
    #[serde(with = "time::serde::rfc3339")]
    pub submitted_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub decided_at: Option<OffsetDateTime>,
    pub decision_reason: Option<String>,
}

/// `AccessStateView` — ce que rend `GET /api/negotiation/me/access` : un écran,
/// une lecture.
#[derive(Debug, Clone, Serialize)]
pub struct MyAccess {
    pub admission_mode: &'static str,
    pub state: AccessState,
    /// Renseigné pour le seul état `granted`.
    pub granted: Option<GrantedAccess>,
    pub networks: Vec<NetworkView>,
    /// La demande la plus récente, quelle que soit son issue : l'écran
    /// d'attente s'y retrouve, et « Mon accès » y lit le motif d'un refus.
    pub request: Option<AccessRequestView>,
    /// `negotiation.report.validate`, portée globale (R6).
    pub can_validate_reports: bool,
    /// Signalements en attente, toutes éditions ; nul sans la permission.
    pub reports_to_review: Option<i64>,
}

impl MyAccess {
    /// L'ordre de priorité, et pourquoi il est celui-là.
    ///
    /// Un accès en cours l'emporte sur tout : c'est un fait, pas une intention.
    /// Vient ensuite une demande en attente, qui dit à la personne d'attendre
    /// plutôt que de ressaisir un code. Puis un accès **retiré**, fait plus
    /// récent et plus dur qu'une demande refusée. Le refus ne se lit qu'en
    /// dernier, avec son motif.
    pub fn composer(
        mode: AdmissionMode,
        accorde: Option<GrantedAccess>,
        acces_retire: bool,
        networks: Vec<NetworkView>,
        request: Option<AccessRequestView>,
    ) -> Self {
        let statut = request.as_ref().map(|r| r.status);
        let state = if accorde.is_some() {
            AccessState::Granted
        } else if statut == Some(RequestStatus::Pending) {
            AccessState::Pending
        } else if acces_retire {
            AccessState::Revoked
        } else if statut == Some(RequestStatus::Rejected) {
            AccessState::Rejected
        } else {
            AccessState::Visitor
        };

        Self {
            admission_mode: mode.as_db(),
            state,
            granted: accorde,
            networks,
            request,
            can_validate_reports: false,
            reports_to_review: None,
        }
    }
}
