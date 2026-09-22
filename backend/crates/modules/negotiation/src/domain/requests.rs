//! Les formes de la demande d'accès : ce que la personne envoie, ce que la
//! file du back-office montre, ce qu'une décision porte.
//!
//! # « ANNULÉE » N'EST PAS « RÉVOQUÉE »
//!
//! FR-026. Une demande s'annule — c'est le fait de la personne, entrée par un
//! code entre-temps. Un accès se retire. Les deux mots désignent deux gestes,
//! deux auteurs et deux conséquences, et les confondre à l'écran ferait croire
//! à une sanction là où il n'y a qu'un doublon refermé.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::access::{AccessScopeView, RequestStatus};

/// `CreateAccessRequestPayload` — ce que l'écran de demande envoie.
///
/// `space_id` absent vaut une demande de portée globale : à cette étape
/// l'espace réservé n'en a qu'un, et l'écran n'a rien à choisir.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct CreateAccessRequestPayload {
    #[serde(default)]
    pub space_id: Option<Uuid>,
    #[serde(default)]
    pub message: Option<String>,
}

/// Une ligne de la file d'attente du back-office.
///
/// Le pays vient de la fiche de la personne, résolu dans sa langue : c'est ce
/// qui permet à l'IFDD de reconnaître une délégation qu'elle attend.
#[derive(Debug, Clone, Serialize)]
pub struct AccessRequestRow {
    pub id: Uuid,
    pub status: RequestStatus,
    pub person_id: Uuid,
    pub display_name: String,
    pub email: String,
    pub country: Option<String>,
    pub scope: AccessScopeView,
    /// Le code reconnu mais insuffisant à ouvrir, en mode « les deux ». C'est
    /// ce que l'administrateur lit d'abord pour trancher.
    pub invitation_code: Option<String>,
    pub invitation_code_label: Option<String>,
    pub message: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub submitted_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub decided_at: Option<OffsetDateTime>,
    pub decided_by_name: Option<String>,
    pub decision_reason: Option<String>,
}

/// `AccessRequestQueue` — la file, et de quoi composer son en-tête.
#[derive(Debug, Clone, Serialize)]
pub struct AccessRequestQueue {
    pub rows: Vec<AccessRequestRow>,
    pub total: i64,
    /// Demandes en attente, **tous filtres confondus** : c'est la pastille du
    /// menu, et elle ne doit pas changer quand on filtre sur « refusées ».
    pub pending: i64,
}

/// `DecideAccessRequestPayload` — le motif d'une décision, toujours facultatif.
///
/// Il est repris **tel quel** dans le courriel de refus : c'est la seule chose
/// que la personne lira pour comprendre.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct DecideAccessRequestPayload {
    #[serde(default)]
    pub reason: Option<String>,
}
