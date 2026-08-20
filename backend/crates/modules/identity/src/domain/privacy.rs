//! La file RGPD, son échéance, et ce que rend son traitement.

use serde::{Deserialize, Serialize};

use super::admin_users::PrivacyRequestView;

/// Échéance réglementaire, en jours.
///
/// **La table en est la source** : `identity.privacy_requests.due_at` la porte
/// par sa valeur par défaut, et le service ne la calcule jamais. La constante
/// n'existe que pour l'annoncer à l'écran, et `effacement.rs` vérifie qu'elle
/// n'a pas divergé du modèle — c'est le seul garde-fou possible pour une valeur
/// qui vit dans un `DEFAULT`.
pub const DEADLINE_DAYS: i32 = 30;

/// L'acte demandé sur une demande.
///
/// `anonymize` n'est pas un état : c'est l'ACTE que réclame un effacement, et il
/// est irréversible. Le distinguer de la clôture administrative évite qu'on
/// l'exécute en croyant seulement classer un dossier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrivacyAction {
    Start,
    Complete,
    Reject,
    Anonymize,
}

#[derive(Debug, Clone, Serialize)]
pub struct PrivacyQueueScreen {
    pub requests: Vec<PrivacyRequestView>,
    pub open_count: usize,
    pub overdue_count: usize,
    pub deadline_days: i32,
}

/// Ce que rend un traitement. **Les quatre issues sortent en 200** : ce sont des
/// refus prévus par le contrat du site, pas des erreurs HTTP. Le refus
/// d'autorisation, lui, n'en fait pas partie — il sort en 403, avant d'arriver
/// ici.
///
/// Les quatre portent la file : l'appelant la lit déjà par la même permission,
/// la lui rendre ne divulgue rien et lui évite un aller-retour pour se recaler.
#[derive(Debug, Clone, Serialize)]
pub struct PrivacyWriteOutcome {
    pub status: &'static str,
    pub request: Option<PrivacyRequestView>,
    pub requests: Vec<PrivacyRequestView>,
}

impl PrivacyWriteOutcome {
    pub fn saved(request: PrivacyRequestView, requests: Vec<PrivacyRequestView>) -> Self {
        Self {
            status: "saved",
            request: Some(request),
            requests,
        }
    }

    /// L'identité est purgée, les comptes supprimés, les sessions révoquées —
    /// et les agrégats de participation intacts.
    pub fn anonymized(request: PrivacyRequestView, requests: Vec<PrivacyRequestView>) -> Self {
        Self {
            status: "anonymized",
            request: Some(request),
            requests,
        }
    }

    /// L'anonymisation ne répond qu'à une demande d'**effacement** (FR-060).
    /// Un export ou une rectification se traitent autrement — les anonymiser
    /// détruirait une identité que personne n'a demandé d'effacer.
    pub fn wrong_type(request: PrivacyRequestView, requests: Vec<PrivacyRequestView>) -> Self {
        Self {
            status: "wrong_type",
            request: Some(request),
            requests,
        }
    }

    pub fn not_found(requests: Vec<PrivacyRequestView>) -> Self {
        Self {
            status: "not_found",
            request: None,
            requests,
        }
    }
}
