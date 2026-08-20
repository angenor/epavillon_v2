//! Événements du module `event`.
//!
//! Six types, trois segments chacun — la forme qu'impose
//! `ck_outbox_event_type_format`. Aucune charge utile ne porte de secret :
//! `platform.outbox_events` est durable et faite pour être relue.
//!
//! **Aucun déclencheur de `060_events.sql` n'émet d'événement de domaine.** Le
//! fichier ne porte que deux déclencheurs d'audit — sur l'édition et sur
//! l'appel — et cinq horodatages. C'est l'inverse du piège rencontré en B1 et
//! B2, où `identity.anonymize_person()` et `org.merge_organizations()`
//! émettaient déjà et où un service zélé aurait produit un doublon. Ici la
//! conséquence est symétrique et vaut d'être dite à l'endroit où l'on serait
//! tenté de croire l'inverse : **le service émet tout lui-même, et rien n'émet
//! à sa place.** Un changement d'état non annoncé par le code n'est annoncé par
//! personne.
//!
//! Ce qui n'émet **rien**, et c'est une soustraction délibérée : les journées du
//! calendrier, les fils de programmation, les lieux, les salles, les canaux de
//! diffusion et le comité de sélection. Aucun autre module n'a à y réagir, et
//! émettre « pour plus tard » remplit la file de messages que personne ne lit.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const AGGREGATE_SCHEMA: &str = "event";
pub const AGGREGATE_EDITION: &str = "edition";
pub const AGGREGATE_CALL: &str = "call";
pub const AGGREGATE_PROGRAMME: &str = "programme";

pub const EDITION_CREATED: &str = "event.edition.created";
pub const EDITION_UPDATED: &str = "event.edition.updated";
pub const CALL_OPENED: &str = "event.call.opened";
pub const CALL_CLOSED: &str = "event.call.closed";
pub const CALL_DEADLINE_EXTENDED: &str = "event.call.deadline_extended";
pub const PROGRAMME_PUBLISHED: &str = "event.programme.published";

/// Personne ne la consomme aujourd'hui. Elle porte la trace de l'entité dont
/// B4, B5 et B6 dépendent tous.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditionCreated {
    pub event_id: Uuid,
    pub slug: String,
    pub series_id: Option<Uuid>,
    pub edition_year: i16,
    pub has_pavilion: bool,
}

/// Ce qui a changé, et rien d'autre. La période et le fuseau intéresseront B5,
/// dont les séances s'y rattachent ; les recopier tous à chaque écriture
/// obligerait le consommateur à comparer lui-même.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditionUpdated {
    pub event_id: Uuid,
    pub period_changed: bool,
    pub status_changed: bool,
    pub pavilion_changed: bool,
    pub timezone_changed: bool,
}

/// **C'est l'annonce qui ouvre le jalon.** L'échéance qui voyage est
/// l'échéance *effective* — `event.effective_deadline()`, prolongation
/// comprise —, jamais `closes_at` seule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallOpened {
    pub call_id: Uuid,
    pub event_id: Uuid,
    #[serde(with = "time::serde::rfc3339")]
    pub effective_deadline: time::OffsetDateTime,
}

/// Émis à la main **ou par la clôture automatique**, indifféremment : c'est le
/// changement d'état qui est annoncé, pas la façon dont il a été demandé.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallClosed {
    pub call_id: Uuid,
    pub event_id: Uuid,
    /// L'échéance qui a été appliquée pour clore.
    #[serde(with = "time::serde::rfc3339")]
    pub applied_deadline: time::OffsetDateTime,
}

/// **L'échéance initiale voyage avec la nouvelle** : c'est celle qui a été
/// annoncée aux organisations, et un rappel qui l'ignore dit une contre-vérité.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallDeadlineExtended {
    pub call_id: Uuid,
    pub event_id: Uuid,
    #[serde(with = "time::serde::rfc3339")]
    pub initial_deadline: time::OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub new_deadline: time::OffsetDateTime,
}

/// Le prédicat **exact** des séances que la publication désigne.
///
/// Il voyage plutôt que d'être redéduit par le consommateur : l'émetteur a
/// compté sous l'instantané de sa transaction, et un consommateur qui
/// recalculerait « les séances de l'édition » publierait autre chose que ce qui
/// a été annoncé.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSelection {
    pub event_id: Uuid,
    /// Statuts retenus — `planned` et `scheduled` aujourd'hui.
    pub statuses: Vec<String>,
    /// Seules les séances pas encore publiques sont visées.
    pub only_unpublished: bool,
}

/// Consommé par **B5**, qui pose la date de publication sur les séances
/// désignées. Son contrat est fixé ici pour que B5 n'ait rien à deviner :
/// garde de rejeu obligatoire par `platform.inbox_events (consumer, event_id)` ;
/// il publie **exactement** le prédicat porté par `selection` ; il n'écrit pas
/// `event.events.programme_published_at`, déjà posée par l'émetteur ; et une
/// seconde livraison ne publie rien de plus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgrammePublished {
    pub event_id: Uuid,
    #[serde(with = "time::serde::rfc3339")]
    pub published_at: time::OffsetDateTime,
    pub selection: SessionSelection,
    /// Ce que l'émetteur a compté au moment d'annoncer.
    pub published_count: i64,
}
