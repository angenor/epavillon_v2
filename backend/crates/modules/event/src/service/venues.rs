//! Le stand et ses salles — ce qui donne à un conflit de créneaux un **sujet
//! nommable**.
//!
//! Sans salle en base, la détection ne peut dire que « deux activités à 14 h ».
//! Avec elle, elle dit « salle Baobab, réservée deux fois ». **Nommer n'est pas
//! interdire** : le chevauchement reste parfaitement écrivable, il est signalé à
//! l'équipe, pas refusé par la base (règle métier n° 2).
//!
//! Deux choses se jouent ici, et aucune n'est un invariant de la base.
//!
//! 1. **`is_virtual` est écrit tel quel, jamais déduit du mode de
//!    participation.** Une salle virtuelle accepte les créneaux simultanés ;
//!    la déduire du mode de l'édition ferait taire, sur une COP hybride, le
//!    conflit de gravité haute que l'équipe doit absolument voir.
//! 2. **Le décompte de détachement se prend AVANT la suppression** (research.md
//!    § R8). Après l'ordre, le lien n'existe plus : le chiffre rendrait zéro, et
//!    l'écran annoncerait sereinement qu'il n'a rien déplacé.

use kernel::context::RequestContext;
use kernel::error::Result;

use crate::domain::ids::{EventId, RoomId, VenueId};
use crate::domain::tabs::{
    EditionRoomPayload, EditionTabResult, EditionVenuePayload, TabErrorCode,
};
use crate::repo::{cross, venues};
use crate::state::EventState;

use super::tabs;

/// Créer ou modifier un lieu.
pub async fn enregistrer_lieu(
    state: &EventState,
    ctx: &RequestContext,
    event_id: EventId,
    existant: Option<VenueId>,
    payload: EditionVenuePayload,
) -> Result<EditionTabResult> {
    let mut tx = state.db().write(ctx).await?;

    let ecriture = match existant {
        None => venues::creer_lieu(&mut tx, event_id, &payload)
            .await
            .map(|_| true),
        Some(id) => venues::modifier_lieu(&mut tx, id, &payload).await,
    };

    match ecriture {
        Err(e) => return tabs::refus_de_base(e),
        Ok(false) => return Ok(EditionTabResult::refuse(TabErrorCode::NotFound)),
        Ok(true) => {}
    }

    tx.commit().await?;

    tabs::reussite(state.pool(), event_id, 0).await
}

/// Retirer un lieu — **et ses salles avec lui**.
///
/// `sessions_detached` compte les séances de **toutes** ses salles, mesurées
/// avant l'ordre : c'est ce que l'écran annonce, et le chiffre doit tomber
/// juste (SC-017).
pub async fn supprimer_lieu(
    state: &EventState,
    ctx: &RequestContext,
    event_id: EventId,
    id: VenueId,
) -> Result<EditionTabResult> {
    let mut tx = state.db().write(ctx).await?;

    let detachees = cross::seances_du_lieu(&mut *tx, id).await?;

    if !venues::supprimer_lieu(&mut tx, id).await? {
        return Ok(EditionTabResult::refuse(TabErrorCode::NotFound));
    }

    tx.commit().await?;

    tabs::reussite(state.pool(), event_id, detachees).await
}

/// Créer ou modifier une salle.
///
/// **Le lieu visé est vérifié dans la transaction**, et pas seulement par le
/// périmètre : sans cela, une salle pourrait être posée dans le lieu d'une autre
/// édition, que l'appelant administre peut-être aussi. C'est la même règle que
/// l'ascendance, appliquée à la charge utile.
pub async fn enregistrer_salle(
    state: &EventState,
    ctx: &RequestContext,
    event_id: EventId,
    existante: Option<RoomId>,
    payload: EditionRoomPayload,
) -> Result<EditionTabResult> {
    let mut tx = state.db().write(ctx).await?;

    let lieu = venues::edition_du_lieu(&mut tx, VenueId::from(payload.venue_id)).await?;
    if lieu != Some(event_id) {
        return Ok(EditionTabResult::refuse(TabErrorCode::NotFound));
    }

    let ecriture = match existante {
        None => venues::creer_salle(&mut tx, &payload).await.map(|_| true),
        Some(id) => venues::modifier_salle(&mut tx, id, &payload).await,
    };

    match ecriture {
        Err(e) => return tabs::refus_de_base(e),
        Ok(false) => return Ok(EditionTabResult::refuse(TabErrorCode::NotFound)),
        Ok(true) => {}
    }

    tx.commit().await?;

    tabs::reussite(state.pool(), event_id, 0).await
}

/// Retirer une salle. Les séances qui s'y tenaient retournent au panneau « à
/// placer » — `ON DELETE SET NULL` —, et leur nombre est **compté avant**.
pub async fn supprimer_salle(
    state: &EventState,
    ctx: &RequestContext,
    event_id: EventId,
    id: RoomId,
) -> Result<EditionTabResult> {
    let mut tx = state.db().write(ctx).await?;

    let detachees = cross::seances_de_la_salle(&mut *tx, id).await?;

    if !venues::supprimer_salle(&mut tx, id).await? {
        return Ok(EditionTabResult::refuse(TabErrorCode::NotFound));
    }

    tx.commit().await?;

    tabs::reussite(state.pool(), event_id, detachees).await
}
