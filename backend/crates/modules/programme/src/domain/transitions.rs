//! Ce que le service fait des règles **lues** — jamais le graphe (R7).
//!
//! # La règle absolue de ce fichier
//!
//! `programme.proposal_transitions_allowed` porte quatorze lignes, et **aucune
//! n'est recopiée ici**. Le graphe est une donnée : ouvrir un chemin doit être
//! une ligne de plus en base, pas une relecture de ce code. Ce fichier ne porte
//! que ce que la table ne dit pas — **comment une règle lue devient une action
//! offerte**, et ce que le service en attend au retour.
//!
//! Deux usages, et deux seulement :
//!
//! - **offrir** — composer le menu d'un lecteur donné, à partir des règles que
//!   la requête a déjà croisées avec sa permission et sa qualité de porteur ;
//! - **exiger un motif** — savoir, avant de tenter, si la transition visée en
//!   réclame un, pour rendre `reason_required` sans aller-retour inutile.
//!
//! Le refus, lui, appartient au déclencheur. Le service **tente** ; il ne
//! rejoue pas la machine.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Les huit états, tels que `programme.proposal_status` les nomme.
///
/// L'énumération traverse la frontière SQL en `text` — patron des trois modules
/// livrés (data-model.md § 6). Elle est ici pour que le service ne compare pas
/// des chaînes nues, pas pour redire la machine à états.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProposalStatus {
    Draft,
    Submitted,
    UnderReview,
    ChangesRequested,
    Accepted,
    Rejected,
    Withdrawn,
    Cancelled,
}

impl ProposalStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Submitted => "submitted",
            Self::UnderReview => "under_review",
            Self::ChangesRequested => "changes_requested",
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
            Self::Withdrawn => "withdrawn",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn from_db(valeur: &str) -> Option<Self> {
        Some(match valeur {
            "draft" => Self::Draft,
            "submitted" => Self::Submitted,
            "under_review" => Self::UnderReview,
            "changes_requested" => Self::ChangesRequested,
            "accepted" => Self::Accepted,
            "rejected" => Self::Rejected,
            "withdrawn" => Self::Withdrawn,
            "cancelled" => Self::Cancelled,
            _ => return None,
        })
    }

    /// **Un dossier encore modifiable par son organisation.**
    ///
    /// C'est ce que `PROPOSAL_NOT_EDITABLE` refuse ailleurs. Ce n'est pas une
    /// règle de la machine à états — elle porte sur les transitions, pas sur la
    /// modification du contenu — et la base ne la tient nulle part.
    ///
    /// # 🔴 UN DOSSIER RETENU RESTE MODIFIABLE (écart n° 110)
    ///
    /// Ce point a été corrigé en écrivant US6 : les fondations fermaient la
    /// modification dès l'acceptation, ce que **ni le contrat d'erreurs ni
    /// l'arbitrage du commanditaire** ne demandent. `PROPOSAL_NOT_EDITABLE` est
    /// décrit comme « dossier **rejeté, retiré, annulé**, ou édition terminée »,
    /// et le commanditaire a tranché le 17/08 : « tant que l'événement n'est
    /// pas terminé, il peut modifier ».
    ///
    /// Le fermer à l'acceptation aurait un coût concret : une organisation
    /// retenue qui repère une coquille dans son intitulé trois jours avant sa
    /// séance n'aurait plus aucun moyen de la corriger.
    ///
    /// **Ce que cela n'autorise pas** : propager quoi que ce soit vers la
    /// séance programmée. Corriger la demande ne rejoue pas la décision — voir
    /// le commentaire de `service/draft_write.rs`, et FR-091.
    pub fn est_modifiable(self) -> bool {
        !matches!(self, Self::Rejected | Self::Withdrawn | Self::Cancelled)
    }
}

/// Une transition **offerte à ce lecteur**, telle que la requête l'a composée.
///
/// `requires_reason` vient de la table ; l'écran s'en sert pour demander le
/// motif **avant** d'envoyer, plutôt que de faire un aller-retour pour
/// l'apprendre.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AvailableTransition {
    pub to_status: ProposalStatus,
    pub requires_reason: bool,
}

/// La règle brute, rendue telle quelle par la route globale du contrat
/// (`GET /proposals/transitions`). Quatorze lignes, sans dossier ni lecteur.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProposalTransitionRule {
    pub from_status: ProposalStatus,
    pub to_status: ProposalStatus,
    pub required_permission: Option<String>,
    pub allowed_for_owner: bool,
    pub requires_reason: bool,
}

/// La transition visée exige-t-elle un motif, d'après ce que la base a rendu ?
///
/// Rendre `None` signifie **transition non déclarée** : le service ne tranche
/// pas pour autant — il tente, et le déclencheur refuse. C'est ce qui garde le
/// graphe à un seul endroit.
pub fn motif_exige(offertes: &[AvailableTransition], vise: ProposalStatus) -> Option<bool> {
    offertes
        .iter()
        .find(|t| t.to_status == vise)
        .map(|t| t.requires_reason)
}

/// Le motif reçu est-il utilisable ? Un motif d'espaces n'en est pas un — et
/// c'est exactement ce que le déclencheur vérifie, `btrim` compris.
pub fn motif_fourni(motif: Option<&str>) -> bool {
    motif.is_some_and(|m| !m.trim().is_empty())
}
