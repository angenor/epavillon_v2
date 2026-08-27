//! Ce que le formulaire écrit — les colonnes de `live.incidents`, ni plus ni
//! moins, plus la décision de publier.
//!
//! **Aucun champ `granted`.** Le site l'envoyait pour rejouer l'autorisation sur
//! des données d'exemple ; l'API lit sa propre session, et un client qui déclare
//! ses droits n'est pas un contrôle d'accès.

use serde::Deserialize;
use serde_json::Value;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

/// Le corps commun aux deux écritures de contenu.
///
/// `publish` est **à part du reste** : enregistrer et publier sont deux actes
/// distincts en base — `live.publish_incident()` horodate, attribue et émet. Un
/// brouillon se relit avant de parler à toute une COP.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct IncidentPayload {
    pub scope: String,
    pub event_id: Option<Uuid>,
    pub event_day_id: Option<Uuid>,
    pub session_id: Option<Uuid>,
    pub organization_id: Option<Uuid>,
    pub incident_kind_code: String,
    pub severity: String,
    pub title: Option<Value>,
    pub message: Value,
    pub action_url: Option<String>,
    pub is_dismissible: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub display_from: OffsetDateTime,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub display_until: Option<OffsetDateTime>,
    pub publish: bool,
}

impl IncidentPayload {
    /// La cible désignée par la portée déclarée. `None` pour `global`, et pour
    /// une portée dont la cible manque — ce que la validation refusera.
    pub fn cible(&self) -> Option<Uuid> {
        match self.scope.as_str() {
            "event" => self.event_id,
            "event_day" => self.event_day_id,
            "session" => self.session_id,
            "organization" => self.organization_id,
            _ => None,
        }
    }
}

/// **`from_event_id` est le seul champ du corps qui ne soit pas une colonne** :
/// c'est l'édition **depuis laquelle** on agit, et donc l'ancre du contrôle de
/// périmètre.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateIncidentPayload {
    #[serde(flatten)]
    pub incident: IncidentPayload,
    pub from_event_id: Uuid,
}

/// Même corps, plus l'identifiant du message corrigé. Il est **redondant avec
/// le chemin** et le contrat du site le porte : c'est le chemin qui fait foi.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpdateIncidentPayload {
    #[serde(flatten)]
    pub incident: IncidentPayload,
    pub from_event_id: Uuid,
    #[serde(default)]
    pub incident_id: Option<Uuid>,
}

/// Le retrait, et son motif. **Ce n'est pas une suppression** : la ligne
/// demeure, avec son instant, son auteur et ce motif, et reparaît à
/// l'historique de la liste.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UnpublishIncidentPayload {
    #[serde(default)]
    pub incident_id: Option<Uuid>,
    #[serde(default)]
    pub reason: Option<String>,
}
