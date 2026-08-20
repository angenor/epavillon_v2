//! Le canal du direct — **ressource réservable**, au même titre qu'une salle.
//!
//! Règle métier n° 4 : l'IFDD ne diffuse jamais deux activités en direct
//! simultanément. Une seule équipe technique, un seul flux. Le canal **par
//! défaut** est donc indispensable : sans lui, une séance marquée « diffusée »
//! n'occupe aucun canal et échappe à la détection. L'écran le pose ; il ne le
//! laisse pas deviner.
//!
//! Trois règles gouvernent ce fichier, et les trois viennent d'une asymétrie du
//! modèle.
//!
//! 1. **Poser le défaut retire le précédent d'abord**, dans la même transaction
//!    (research.md § R6). L'index n'est pas différable : l'ordre inverse échoue.
//! 2. **Retirer un canal qui a servi le désactive**, et c'est un **succès**
//!    (§ R7). Ce qu'une suppression perdrait, c'est la trace du canal sur lequel
//!    une activité passée a été diffusée.
//! 3. **Un canal général de la plateforme ne se modifie pas depuis une
//!    édition.** Il n'est ni introuvable ni hors périmètre : son refus est celui
//!    du contrat, `platform_channel`.

use kernel::context::RequestContext;
use kernel::error::Result;

use crate::domain::ids::{ChannelId, EventId};
use crate::domain::tabs::{EditionChannelPayload, EditionTabResult, TabErrorCode};
use crate::repo::{channels, cross};
use crate::state::EventState;

use super::tabs;

/// Créer ou modifier un canal d'édition.
pub async fn enregistrer(
    state: &EventState,
    ctx: &RequestContext,
    event_id: EventId,
    existant: Option<ChannelId>,
    payload: EditionChannelPayload,
) -> Result<EditionTabResult> {
    let mut tx = state.db().write(ctx).await?;

    // **Retirer AVANT de poser.** `ux_broadcast_channels_default` est un index
    // unique partiel et non différable : l'ordre inverse violerait l'unicité au
    // milieu de la transaction. Le groupe visé est celui de l'édition — le canal
    // général de la plateforme, qui forme son propre groupe, n'est pas délogé.
    if payload.is_default && payload.is_active {
        channels::retirer_le_defaut(&mut tx, event_id).await?;
    }

    let ecriture = match existant {
        None => channels::creer(&mut tx, event_id, &payload)
            .await
            .map(|_| true),
        Some(id) => channels::modifier(&mut tx, id, &payload).await,
    };

    match ecriture {
        Err(e) => return tabs::refus_de_base(e),
        Ok(false) => return Ok(EditionTabResult::refuse(TabErrorCode::NotFound)),
        Ok(true) => {}
    }

    tx.commit().await?;

    tabs::reussite(state.pool(), event_id, 0).await
}

/// **Le refus propre aux canaux généraux de la plateforme.**
///
/// Un canal sans édition sert plusieurs événements ; le modifier depuis l'un
/// d'eux le changerait pour tous. Ce n'est ni un introuvable ni un refus de
/// périmètre : c'est un refus **du contrat**, que l'écran sait expliquer.
pub fn refus_de_canal_de_plateforme() -> EditionTabResult {
    EditionTabResult::refuse(TabErrorCode::PlatformChannel)
}

/// Retirer un canal — **désactivé s'il a servi, supprimé sinon**.
///
/// Le décompte se prend **avant**, et il porte sur le canal et non sur
/// l'édition : un canal général sert plusieurs éditions, et ne compter que les
/// séances de celle-ci ferait annoncer « aucune séance » à qui s'apprête à le
/// retirer.
///
/// La désactivation est un **succès** : `ok: true`, `error_code: deactivated`.
/// C'est le seul endroit du module où ce champ ne signale pas une erreur.
pub async fn retirer(
    state: &EventState,
    ctx: &RequestContext,
    event_id: EventId,
    id: ChannelId,
) -> Result<EditionTabResult> {
    let mut tx = state.db().write(ctx).await?;

    let diffusees = cross::seances_du_canal(&mut *tx, id).await?;

    let a_servi = diffusees > 0;
    let touche = if a_servi {
        channels::desactiver(&mut tx, id).await?
    } else {
        channels::supprimer(&mut tx, id).await?
    };

    if !touche {
        return Ok(EditionTabResult::refuse(TabErrorCode::NotFound));
    }

    tx.commit().await?;

    if a_servi {
        tabs::desactive(state.pool(), event_id, diffusees).await
    } else {
        tabs::reussite(state.pool(), event_id, 0).await
    }
}
