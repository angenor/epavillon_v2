//! Le poste de direct, les cibles offertes au choix de portée, et l'écran
//! entier.

use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use super::incident::ManagedIncident;

/// Une cible offerte au choix de portée, déjà résolue pour l'affichage.
///
/// **`hint` est un TEXTE, `starts_at` est un INSTANT.** Le premier porte le
/// sigle d'une organisation ou la date civile d'une journée ; le second n'est
/// affichable que par l'interface, dans le fuseau de l'édition et jamais dans
/// celui du navigateur. Les mélanger avait fait apparaître un
/// `2027-11-13T09:30:00-03:00` brut dans une liste déroulante.
#[derive(Debug, Clone, Serialize)]
pub struct IncidentTargetOption {
    pub id: Uuid,
    pub label: String,
    pub hint: Option<String>,
    /// Début du créneau, pour une activité. Formaté à l'affichage.
    #[serde(with = "time::serde::rfc3339::option")]
    pub starts_at: Option<OffsetDateTime>,
}

/// **Les cibles de l'édition en cours, et rien d'autre** — règle métier n° 8.
///
/// Un administrateur détaché sur la COP31 ne doit pas pouvoir viser une journée
/// d'une autre édition, y compris en forgeant une requête. Les organisations
/// offertes sont celles qui **animent** au moins une activité de l'édition :
/// c'est le même critère que la portée `organization` de
/// `live.event_incidents()`.
#[derive(Debug, Clone, Serialize)]
pub struct IncidentTargets {
    pub event: IncidentTargetOption,
    pub days: Vec<IncidentTargetOption>,
    pub sessions: Vec<IncidentTargetOption>,
    pub organizations: Vec<IncidentTargetOption>,
}

/// Une activité que le poste de direct surveille — `programme.sessions`, plus
/// l'état temporel que calcule `programme.v_public_schedule`.
///
/// La lecture porte sur la **table** et non sur la vue publique : une activité
/// non publiée peut parfaitement tomber en panne, et le poste de direct est un
/// écran de back-office.
#[derive(Debug, Clone, Serialize)]
pub struct LiveDeskSession {
    pub session_id: Uuid,
    pub title: Value,
    #[serde(with = "time::serde::rfc3339")]
    pub starts_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub ends_at: OffsetDateTime,
    pub room_name: Option<Value>,
    /// Une activité non diffusée n'a pas d'incident de diffusion à signaler.
    pub is_streamed: bool,
    pub status: String,
    pub temporal_state: String,
    /// Messages **déjà actifs de portée `session`** visant cette activité : ne
    /// pas publier deux fois la même panne.
    pub active_incident_count: i64,
}

/// Ce que le poste montre, et **de quel jour il parle**.
///
/// `day` est le jour de l'ÉDITION — `(now() AT TIME ZONE events.timezone)::date`,
/// calculé en base —, jamais la date du serveur.
///
/// `is_fallback` est vrai quand l'édition n'a **aucune** activité ce jour-là et
/// que le poste montre les quatre prochaines à la place. `day` reste alors
/// aujourd'hui : « rien aujourd'hui » et « voici la suite » ne sont pas la même
/// information, et les confondre ferait croire à un direct en cours hors
/// période.
#[derive(Debug, Clone, Serialize)]
pub struct LiveDesk {
    pub day: Date,
    pub sessions: Vec<LiveDeskSession>,
    pub is_fallback: bool,
}

/// Compteurs de la barre d'états — établis **avant tout filtrage**, comme
/// partout dans le projet. Les cinq états sont posés à zéro d'abord : un état
/// absent de la réponse ferait afficher un tiret là où l'écran attend un
/// décompte.
pub type IncidentStateCounts = BTreeMap<String, i64>;

/// Les cinq états, dans l'ordre où le modèle les nomme.
pub const ETATS: [&str; 5] = ["active", "scheduled", "draft", "expired", "unpublished"];

/// Un terme de `reference.taxonomy_terms`, tel quel.
///
/// La forme entière, et non un extrait : le contrat du site annonce
/// `TaxonomyTerm[]`, et servir un sous-ensemble obligerait l'écran à deux
/// lectures du même vocabulaire selon l'endroit d'où il l'a reçu.
#[derive(Debug, Clone, Serialize)]
pub struct TaxonomyTerm {
    pub id: Uuid,
    pub taxonomy_code: String,
    pub parent_id: Option<Uuid>,
    pub code: String,
    pub label: Value,
    pub description: Option<Value>,
    pub color_hex: Option<String>,
    pub icon: Option<String>,
    pub sort_order: i16,
    pub is_active: bool,
    pub superseded_by: Option<Uuid>,
    pub metadata: Value,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

/// **Tout l'écran en une réponse.**
///
/// `timezone` est celui de l'ÉDITION : une fenêtre d'affichage se lit dans le
/// fuseau où l'incident a lieu, jamais dans celui du navigateur de qui publie.
/// `zone_label` est la **ville** — « heure de Belém », et non « heure de
/// America/Belem ».
#[derive(Debug, Clone, Serialize)]
pub struct IncidentListScreen {
    pub event_id: Uuid,
    pub event_title: Value,
    pub timezone: String,
    pub zone_label: Option<String>,
    pub rows: Vec<ManagedIncident>,
    pub desk: LiveDesk,
    pub counts: IncidentStateCounts,
    pub kinds: Vec<TaxonomyTerm>,
    pub targets: IncidentTargets,
}

/// Ce que le raccourci « Signaler un débordement » du planificateur a besoin de
/// savoir.
///
/// **`title` est ici RÉSOLU et non brut**, à la différence du reste de l'écran :
/// c'est une valeur de pré-remplissage de champ, pas une donnée à afficher — le
/// site la pose telle quelle dans le formulaire.
#[derive(Debug, Clone, Serialize)]
pub struct OverrunTemplate {
    pub session_id: Uuid,
    pub title: String,
    #[serde(with = "time::serde::rfc3339")]
    pub starts_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub ends_at: OffsetDateTime,
    pub event_id: Uuid,
}
