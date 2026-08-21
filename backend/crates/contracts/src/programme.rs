//! Événements du module `programme`.
//!
//! # Le piège de ce module, énoncé là où l'on serait tenté d'en ajouter
//!
//! **`programme.tg_guard_proposal_status()` émet DÉJÀ.** À chaque transition
//! acceptée, le déclencheur appelle `platform.emit_event()` dans la
//! transaction, avec le numéro de dossier, l'édition, l'organisation, les deux
//! états et le motif. Huit types, un par état d'arrivée :
//!
//! `programme.proposal.draft` (à la création) · `.submitted` · `.under_review`
//! · `.changes_requested` · `.accepted` · `.rejected` · `.withdrawn` ·
//! `.cancelled`
//!
//! **C'est l'inverse de B3**, où aucun déclencheur du module n'émettait rien,
//! et c'est le retour du piège de B1 (`anonymize_person()`) et de B2
//! (`merge_organizations()`).
//!
//! **Conséquence, et elle est absolue : le service n'émet AUCUN événement de
//! changement d'état.** Émettre à son tour produirait deux événements par
//! transition — donc deux courriels, deux notifications, deux incréments —, et
//! le doublon ne se verrait qu'en production. Aucune charge utile de
//! changement d'état ne figure donc dans ce fichier : **son absence est la
//! décision**, pas un oubli.
//!
//! # Les trois que le service émet
//!
//! Ils décrivent des faits que la base n'annonce pas, et chacun a son
//! consommateur identifié en B6. Un événement **par dossier** dans une action
//! groupée, jamais un pour le lot : un consommateur qui reçoit un lot doit le
//! déplier lui-même, et son échec porte alors sur douze effets au lieu d'un.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

pub const AGGREGATE_SCHEMA: &str = "programme";
pub const AGGREGATE_PROPOSAL: &str = "proposal";
pub const AGGREGATE_COMMENT: &str = "comment";
pub const AGGREGATE_REVIEW: &str = "review";

pub const COORGANIZATION_REQUESTED: &str = "programme.coorganization.requested";
pub const COMMENT_SHARED: &str = "programme.comment.shared";
pub const REVIEW_ASSIGNED: &str = "programme.review.assigned";

/// Une co-organisation annoncée **engage un tiers** : le front dit déjà
/// « sera invitée à confirmer sa participation ». B6 enverra la demande.
///
/// Émis **par organisation ajoutée**, et seulement pour un rôle autre que
/// porteur : la ligne du porteur est posée par déclencheur, jamais par le
/// service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoorganizationRequested {
    pub proposal_id: Uuid,
    pub reference_code: String,
    pub event_id: Uuid,
    /// L'organisation invitée à confirmer.
    pub organization_id: Uuid,
    /// `programme.organization_role`, jamais `lead`.
    pub role: String,
    /// L'organisation porteuse, celle qui répond du dossier.
    pub lead_organization_id: Uuid,
}

/// **Le seul écrit du comité qui sorte du comité.** Sans annonce,
/// l'organisation découvre une demande de correction en revenant sur son
/// espace.
///
/// `is_change_request` voyage parce que l'avis n'est pas le même : « un
/// message vous attend » et « votre dossier demande une correction » ne se
/// rédigent pas pareil, et le consommateur ne doit pas relire le message pour
/// le savoir.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentShared {
    pub proposal_id: Uuid,
    pub reference_code: String,
    pub comment_id: Uuid,
    pub author_id: Uuid,
    /// L'organisation porteuse : c'est elle qu'on prévient.
    pub organization_id: Uuid,
    pub is_change_request: bool,
}

/// Ce qui déclenche le rappel de revue de B6.
///
/// L'échéance est celle de l'affectation (`review_assignments.due_at`), qui
/// peut être nulle : un comité peut confier sans dater.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewAssigned {
    pub proposal_id: Uuid,
    pub reference_code: String,
    pub event_id: Uuid,
    pub reviewer_id: Uuid,
    #[serde(with = "time::serde::rfc3339::option")]
    pub due_at: Option<OffsetDateTime>,
}
