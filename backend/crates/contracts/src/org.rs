//! Événements du module `org`.
//!
//! Six types, trois segments chacun — la forme qu'impose
//! `ck_outbox_event_type_format`. Aucune charge utile ne porte de jeton ni
//! d'adresse électronique : `platform.outbox_events` est durable et faite pour
//! être relue.
//!
//! **Deux de ces types portent le nom exact d'un type de notification déjà semé**
//! dans `110_engagement.sql` § 11 — `org.membership.requested` et
//! `org.membership.approved`. Ce n'est pas une coïncidence à défaire : B6 y
//! branchera ses modèles de message sans rien renommer.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const AGGREGATE_SCHEMA: &str = "org";
pub const AGGREGATE_ORGANIZATION: &str = "organization";
pub const AGGREGATE_MEMBERSHIP: &str = "membership";

pub const ORGANIZATION_CREATED: &str = "org.organization.created";
pub const ORGANIZATION_VERIFIED: &str = "org.organization.verified";
pub const ORGANIZATION_UNVERIFIED: &str = "org.organization.unverified";
/// **Émis par la base** : `org.merge_organizations()` appelle elle-même
/// `platform.emit_event()` avant de rendre la main, et marque elle-même la
/// paire de la file des doublons.
///
/// Le service de fusion n'émet donc rien et ne marque rien. C'est le piège n° 1
/// du module `identity` — `identity.anonymize_person()` — répété à l'identique :
/// un second appel écrirait deux lignes **sans qu'aucune erreur ne le signale**,
/// et un consommateur idempotent traiterait la première puis ignorerait la
/// mauvaise.
pub const ORGANIZATION_MERGED: &str = "org.organization.merged";
pub const MEMBERSHIP_REQUESTED: &str = "org.membership.requested";
pub const MEMBERSHIP_APPROVED: &str = "org.membership.approved";
pub const MEMBERSHIP_REVOKED: &str = "org.membership.revoked";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationCreated {
    pub organization_id: Uuid,
    pub status: String,
    pub country_id: Option<Uuid>,
    pub organization_type_code: String,
    /// Fiches proches montrées à la personne avant qu'elle n'insiste. Créer
    /// sans rien voir n'est pas la même chose que créer en sachant.
    pub acknowledged_matches: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationVerified {
    pub organization_id: Uuid,
    #[serde(with = "time::serde::rfc3339")]
    pub verified_at: time::OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationUnverified {
    pub organization_id: Uuid,
}

/// Charge utile écrite **par la base**. Elle n'est déclarée ici que pour être
/// relue par un consommateur — jamais sérialisée par le service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationMerged {
    pub source_id: Uuid,
    pub target_id: Uuid,
    pub rows: serde_json::Value,
}

/// La direction de l'attente voyage dans la charge utile : une invitation émise
/// n'a pas d'événement à elle, elle produit une adhésion en attente annoncée
/// ici. Ajouter un septième type pour un état déjà décrit ferait deux vérités.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MembershipDirection {
    /// La personne a demandé ; un référent doit trancher.
    Requested,
    /// L'organisation a invité ; la personne doit accepter.
    Invited,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MembershipRequested {
    pub membership_id: Uuid,
    pub organization_id: Uuid,
    pub person_id: Uuid,
    pub direction: MembershipDirection,
    /// Le domaine a-t-il rattaché la personne d'office ?
    pub auto_joined: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MembershipApproved {
    pub membership_id: Uuid,
    pub organization_id: Uuid,
    pub person_id: Uuid,
    pub role: String,
}

/// Pourquoi une adhésion est tombée. Les trois motifs ne se lisent pas pareil :
/// un refus vient d'un référent, un retrait d'un administrateur, un départ de
/// la personne elle-même.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RevocationCause {
    Declined,
    Removed,
    Left,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MembershipRevoked {
    pub membership_id: Uuid,
    pub organization_id: Uuid,
    pub person_id: Uuid,
    pub cause: RevocationCause,
}
