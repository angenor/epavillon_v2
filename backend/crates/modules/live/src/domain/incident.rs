//! La ligne de gestion d'un message d'incident, et l'issue d'une écriture.
//!
//! Les noms de champs sont **exactement** ceux de
//! `frontend/app/types/admin-incidents.ts` : c'est le contrat du site, et il ne
//! se renégocie pas.

use serde::Serialize;
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

/// L'état d'un message, **calculé par `live.event_incidents()`** et jamais
/// recomposé ici.
///
/// `expired` et `unpublished` sont deux fins distinctes : la première est venue
/// seule, à l'heure prévue — c'est la correction de la v1, dont les bandeaux
/// restaient en ligne des mois ; la seconde est une décision, tracée avec son
/// auteur et son motif.
pub type IncidentState = String;

/// Ligne de `live.event_incidents(event_id, at)` — `080_live.sql` § 6.
///
/// **`unpublished_by_name` ne vient pas de la fonction** : elle rend
/// `unpublished_at` et `unpublish_reason`, mais pas le nom de qui a retiré. Le
/// dépôt le complète par une jointure, sans quoi l'historique afficherait
/// « retiré par — » alors que la colonne porte l'identifiant.
#[derive(Debug, Clone, Serialize)]
pub struct ManagedIncident {
    pub incident_id: Uuid,
    pub scope: String,
    pub severity: String,
    /// Code de la taxonomie `incident_kind` — vocabulaire ouvert, pas un ENUM.
    pub kind_code: String,
    pub title: Option<Value>,
    pub message: Value,
    pub action_url: Option<String>,
    pub is_dismissible: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub display_from: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub display_until: Option<OffsetDateTime>,
    /// Cible de la portée : séance, journée, organisation ou édition. Nulle si
    /// la portée est globale.
    pub target_id: Option<Uuid>,
    /// Cible **résolue par la fonction** — « Atelier de négociation », jamais un
    /// identifiant. Une journée sans titre est désignée par sa date.
    pub target_label: Option<String>,
    pub state: IncidentState,
    #[serde(with = "time::serde::rfc3339::option")]
    pub published_at: Option<OffsetDateTime>,
    pub published_by: Option<Uuid>,
    pub published_by_name: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub unpublished_at: Option<OffsetDateTime>,
    pub unpublished_by_name: Option<String>,
    pub unpublish_reason: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

/// **Les dix issues d'une écriture, sous un seul discriminant.**
///
/// `forbidden` et `not_found` en font partie, et c'est délibéré : le contrat du
/// site les nomme, et l'écran les traduit une par une sous le champ concerné
/// (`admin.incident.form.error.<statut>`). Répondre 403 ou 404 à ces deux-là
/// ferait **lever le client** là où il attend un message posé dans son
/// formulaire.
///
/// Ce qui reste en HTTP, c'est le **périmètre** : il ne figure pas au contrat du
/// site et ne doit rien révéler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IncidentWriteStatus {
    Created,
    Updated,
    Published,
    Unpublished,
    MissingTarget,
    MissingMessage,
    InvalidWindow,
    NotPublished,
    NotFound,
    Forbidden,
}

/// L'issue d'une écriture. **Toujours en 200.**
///
/// Sur un refus, `incident` vaut `null`. Sur un succès, il porte la ligne de
/// gestion **relue par `live.event_incidents()`** : l'état affiché est celui que
/// la base calcule, jamais un état recomposé côté service.
#[derive(Debug, Clone, Serialize)]
pub struct IncidentWriteResult {
    pub status: IncidentWriteStatus,
    pub incident: Option<ManagedIncident>,
}

impl IncidentWriteResult {
    pub fn refuse(status: IncidentWriteStatus) -> Self {
        Self {
            status,
            incident: None,
        }
    }

    pub fn abouti(status: IncidentWriteStatus, incident: Option<ManagedIncident>) -> Self {
        Self { status, incident }
    }
}
