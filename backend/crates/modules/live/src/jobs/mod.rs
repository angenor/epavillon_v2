//! Les travaux de la file « live ». Le worker n'écoute cette file que
//! parce que ces gestionnaires la déclarent.

pub mod annonce;
pub mod annulation_differee;
pub mod cancel_meeting;
pub mod cancel_participant;
pub mod fenetre;
pub mod filet;
pub mod sync_meeting;
pub mod sync_participant;
pub mod webhook;

use uuid::Uuid;

use crate::zoom::ZoomError;

/// `Retry-After`, sinon trente secondes doublées à chaque essai (une heure au
/// plus), plus jusqu'à une minute propre à chaque ligne : trois cents
/// inscrits refusés ensemble ne reviennent pas ensemble.
pub(crate) fn delai_avant(e: &ZoomError, essais: i16, id: Uuid) -> i64 {
    let base = match e.reessayer_dans() {
        Some(s) => s as i64,
        None => (30i64 << (essais.clamp(1, 8) - 1)).min(3600),
    };
    base + i64::from(id.as_bytes()[15] % 60)
}
