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
/// Message du comité adressé à des personnes nommées, réponse dans leur fil,
/// ou personnes ajoutées au fil. Charge `WithNotification`, courriel compris.
pub const COMMENT_ADDRESSED: &str = "programme.comment.addressed";
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

// -----------------------------------------------------------------------------
// Ce que la BASE émet, et que B6 consomme
//
// Ces quatorze noms n'étaient nulle part. Sans eux, chaque consommateur les
// écrirait en littéral dans son coin, et une faute de frappe donnerait un
// consommateur qui ne se réveille jamais — sans erreur, sans trace.
//
// **Aucune charge utile n'est déclarée ici.** Ces événements sont émis par
// `programme.tg_sessions_emit_events()` et `programme.tg_registrations_emit_events()`,
// pas par le service : leur forme appartient au SQL, et la figer dans une
// structure Rust ferait croire à un contrat que le déclencheur ne connaît pas.
// Le consommateur lit les champs dont il a besoin, un par un.
//
// LE PIÈGE, ET IL EST ÉCRIT LÀ OÙ ON LE CHERCHERAIT : il n'existe **aucun**
// `programme.registration.confirmed`. Le commentaire de
// `engagement.schedule_session_reminders()` le nomme, mais `registration_status`
// vaut `registered`, `waitlisted`, `cancelled`, `attended`, `no_show` — jamais
// `confirmed`. Et une inscription ordinaire naît **à l'état inscrit, par une
// création** : un consommateur qui n'écouterait que les changements d'état
// raterait la quasi-totalité des inscriptions. On branche donc sur le STATUT
// porté par la charge utile, jamais sur le type d'événement (écart n° 126).
// -----------------------------------------------------------------------------

pub const AGGREGATE_SESSION: &str = "session";
pub const AGGREGATE_REGISTRATION: &str = "registration";

/// Émis à l'insertion, avec le statut d'arrivée en charge utile — c'est le
/// chemin le plus courant, et celui qu'une lecture du modèle aurait cassé.
pub const REGISTRATION_CREATED: &str = "programme.registration.created";
/// Les cinq suivants portent le nom d'une valeur de `programme.registration_status`
/// et sont émis à chaque changement d'état.
pub const REGISTRATION_REGISTERED: &str = "programme.registration.registered";
pub const REGISTRATION_WAITLISTED: &str = "programme.registration.waitlisted";
pub const REGISTRATION_CANCELLED: &str = "programme.registration.cancelled";
pub const REGISTRATION_ATTENDED: &str = "programme.registration.attended";
pub const REGISTRATION_NO_SHOW: &str = "programme.registration.no_show";

/// Émis à l'insertion d'une séance.
pub const SESSION_CREATED: &str = "programme.session.created";
/// Les cinq suivants portent le nom d'une valeur de `programme.session_status`.
pub const SESSION_PLANNED: &str = "programme.session.planned";
pub const SESSION_SCHEDULED: &str = "programme.session.scheduled";
pub const SESSION_LIVE: &str = "programme.session.live";
pub const SESSION_COMPLETED: &str = "programme.session.completed";
pub const SESSION_POSTPONED: &str = "programme.session.postponed";
pub const SESSION_CANCELLED: &str = "programme.session.cancelled";
/// Le seul qui ne soit pas un état : émis quand le créneau change **sans** que
/// le statut bouge. La charge utile porte alors `previous_starts_at`.
pub const SESSION_RESCHEDULED: &str = "programme.session.rescheduled";

/// Régie du jour (02/11) : une activité annulée en régie, courriel aux
/// inscrits dû à `notify_at`. Celui qui envoie vérifie à l'heure dite que le
/// geste `action_id` n'a pas été défait : un retour en arrière dans les cinq
/// minutes ne laisse partir aucun courriel.
pub const REGIE_CANCELLATION_NOTICE: &str = "programme.regie.cancellation_notice";
/// L'annulation `action_id` est défaite : écrire « finalement maintenue » à
/// ceux qui ont reçu le courriel d'annulation, et à eux seuls.
pub const REGIE_RESTORATION_NOTICE: &str = "programme.regie.restoration_notice";

/// Le travail qui envoie (module `engagement`) et la clé des courriels d'une
/// annulation : la régie lit leur état dans `platform.jobs` pour dire « prévenus
/// à 08:45 », sans table de plus.
pub const REGIE_NOTICE_TASK: &str = "engagement.regie_notice";
pub const NOTIFICATION_EMAIL_TASK: &str = "engagement.notification_email";

pub fn regie_notice_key(kind: &str, action_id: Uuid) -> String {
    format!("{kind}:{action_id}")
}

/// Préfixe des courriels d'annulation d'un geste, un par personne.
pub fn regie_email_prefix(action_id: Uuid) -> String {
    format!("regie:{action_id}:")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegieNotice {
    pub action_id: Uuid,
    pub session_id: Uuid,
    #[serde(with = "time::serde::rfc3339")]
    pub notify_at: OffsetDateTime,
}
