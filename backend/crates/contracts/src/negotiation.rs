//! Événements du module `negotiation`.
//!
//! Six types, trois segments chacun — la forme qu'impose
//! `ck_outbox_event_type_format`. Aucune charge utile ne porte de jeton, de
//! code d'invitation ni d'adresse électronique : `platform.outbox_events` est
//! durable et faite pour être relue.
//!
//! **Trois de ces types sont émis par la BASE**, dans la transaction du
//! changement d'état : `tg_access_request_event()` écrit lui-même les trois
//! événements de demande. Leurs charges utiles sont déclarées ici pour être
//! relues par un consommateur, jamais sérialisées par le service — c'est le
//! piège de `org.organization.merged`, répété à l'identique.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const AGGREGATE_SCHEMA: &str = "negotiation";
pub const AGGREGATE_ACCESS_REQUEST: &str = "access_request";
pub const AGGREGATE_SPACE_ACCESS: &str = "space_access";
pub const AGGREGATE_INVITATION_CODE: &str = "invitation_code";

/// Émis par la base, à l'insertion de la demande.
pub const ACCESS_REQUEST_SUBMITTED: &str = "negotiation.access_request.submitted";
/// Émis par la base, à la décision. C'est ce qui garantit que le courriel ne
/// part qu'une fois la décision enregistrée.
pub const ACCESS_REQUEST_APPROVED: &str = "negotiation.access_request.approved";
/// Émis par la base, à la décision.
pub const ACCESS_REQUEST_REJECTED: &str = "negotiation.access_request.rejected";

/// Émis par le service, dans la transaction qui attribue le rôle.
pub const SPACE_ACCESS_GRANTED: &str = "negotiation.space_access.granted";
/// Émis par le service, dans la transaction qui révoque l'attribution.
pub const SPACE_ACCESS_REVOKED: &str = "negotiation.space_access.revoked";
/// Émis par le service. **Ne dit rien des accès déjà accordés** : révoquer un
/// code n'en retire aucun (ADR-006), le retrait est un second geste.
pub const INVITATION_CODE_REVOKED: &str = "negotiation.invitation_code.revoked";

/// Charge utile écrite **par la base**. Déclarée ici pour être relue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessRequestSubmitted {
    pub person_id: Uuid,
    pub scope_type: String,
    pub space_id: Option<Uuid>,
    /// Mode « les deux » : le code a été reconnu, il n'a pas suffi à ouvrir.
    pub invitation_code_id: Option<Uuid>,
}

/// Charge utile écrite **par la base**, commune à l'admission et au refus : la
/// décision se lit dans le type de l'événement, pas dans sa charge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessRequestDecided {
    pub person_id: Uuid,
    pub scope_type: String,
    pub space_id: Option<Uuid>,
    pub decided_by: Option<Uuid>,
    /// Motif facultatif, repris tel quel dans le courriel de décision.
    pub reason: Option<String>,
}

/// Par où l'accès est venu. Les trois ne se lisent pas pareil : un code
/// s'impute à une diffusion, une décision à un administrateur, une attribution
/// directe à un geste du back-office.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AccessOrigin {
    InvitationCode,
    AccessRequest,
    DirectGrant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpaceAccessGranted {
    pub person_id: Uuid,
    pub scope_type: String,
    pub space_id: Option<Uuid>,
    pub origin: AccessOrigin,
    /// Identifiant du code, jamais le code lui-même : l'outbox se relit.
    pub invitation_code_id: Option<Uuid>,
    /// Réseau rejoint du même geste, s'il y en a un.
    pub network_term_id: Option<Uuid>,
}

/// Pourquoi un accès est tombé. Un retrait vient d'un administrateur ; un
/// retrait en bloc suit un code compromis.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RevocationCause {
    Removed,
    CodeCompromised,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpaceAccessRevoked {
    pub person_id: Uuid,
    pub scope_type: String,
    pub space_id: Option<Uuid>,
    pub cause: RevocationCause,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvitationCodeRevoked {
    pub invitation_code_id: Uuid,
    pub scope_type: String,
    pub space_id: Option<Uuid>,
    pub reason: Option<String>,
    /// Accès déjà accordés par ce code, et qui **restent ouverts** : le nombre
    /// dit à l'administrateur ce qu'il lui reste éventuellement à retirer.
    pub granted_uses: i32,
}
