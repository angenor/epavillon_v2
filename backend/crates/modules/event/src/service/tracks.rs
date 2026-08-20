//! Les **journées spéciales** — qui ne sont pas des jours du calendrier.
//!
//! « Journée finance durable », « Journée jeunesse et climat » : leur
//! composition est une **décision éditoriale**, prise au planificateur parmi les
//! activités retenues (règle métier n° 7). Cet écran crée le fil, l'habille et
//! ouvre sa page publique ; **il ne lui rattache aucune séance**.
//!
//! Trois choses se jouent ici.
//!
//! 1. **Le fil, ses thématiques et sa page publique dans le même geste.** Le
//!    contrat porte `is_published` et les codes de thématique dans la même
//!    charge utile ; les écrire en deux temps laisserait exister un fil publié
//!    sans ses pastilles, le temps d'un aller-retour.
//! 2. **L'unicité du code et de l'adresse porte sur l'ÉDITION**, pas sur la
//!    plateforme : deux COP peuvent chacune avoir leur `journee_finance`. C'est
//!    la base qui le tient (`ux_programme_tracks_code`, `_slug`) ; on traduit.
//! 3. **Supprimer un fil ne supprime aucune séance** : ce qui disparaît, ce sont
//!    les rattachements — du travail éditorial, chiffré avant de confirmer.

use kernel::context::RequestContext;
use kernel::error::Result;

use crate::domain::ids::{EventId, TrackId};
use crate::domain::tabs::{EditionTabResult, EditionTrackPayload, TabErrorCode};
use crate::repo::{cross, themes, tracks};
use crate::state::EventState;

use super::tabs;

/// Créer ou modifier un fil, **thématiques comprises**.
pub async fn enregistrer(
    state: &EventState,
    ctx: &RequestContext,
    event_id: EventId,
    existant: Option<TrackId>,
    payload: EditionTrackPayload,
) -> Result<EditionTabResult> {
    let mut tx = state.db().write(ctx).await?;

    let id = match existant {
        None => match tracks::creer(&mut tx, event_id, &payload).await {
            Ok(id) => id,
            Err(e) => return tabs::refus_de_base(e),
        },
        Some(id) => match tracks::modifier(&mut tx, id, &payload).await {
            Err(e) => return tabs::refus_de_base(e),
            Ok(false) => return Ok(EditionTabResult::refuse(TabErrorCode::NotFound)),
            Ok(true) => id,
        },
    };

    // **Dans la même transaction que le fil.** Un fil enregistré sans ses
    // pastilles s'afficherait nu le temps d'un rechargement, et une thématique
    // retirée resterait visible si l'écriture échouait après coup.
    themes::poser(&mut tx, id.as_uuid(), &payload.theme_codes).await?;

    tx.commit().await?;

    tabs::reussite(state.pool(), event_id, 0).await
}

/// Supprimer un fil — **en chiffrant les rattachements perdus**.
///
/// Aucune séance n'est supprimée : `programme.session_tracks` cascade, les
/// séances restent. Ce que l'équipe perd, c'est la composition — et c'est
/// précisément ce qu'elle doit voir avant de confirmer (research.md § R8).
pub async fn supprimer(
    state: &EventState,
    ctx: &RequestContext,
    event_id: EventId,
    id: TrackId,
) -> Result<EditionTabResult> {
    let mut tx = state.db().write(ctx).await?;

    let rattachements = cross::rattachements_du_fil(&mut *tx, id).await?;

    if !tracks::supprimer(&mut tx, id).await? {
        return Ok(EditionTabResult::refuse(TabErrorCode::NotFound));
    }

    tx.commit().await?;

    tabs::reussite(state.pool(), event_id, rattachements).await
}
