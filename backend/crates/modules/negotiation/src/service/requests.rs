//! Demander l'accès, et retirer sa demande — **côté personne**.
//!
//! # L'UNICITÉ VIENT DE LA BASE, PAS D'UNE LECTURE PRÉALABLE
//!
//! « Une seule demande en attente par personne et par portée » est tenue par
//! `ux_access_requests_pending_space` et `ux_access_requests_pending_global`.
//! Deux appareils qui envoient ensemble ne produisent qu'une ligne, et le
//! conflit sort traduit en `NEGOTIATION_ACCESS_REQUEST_PENDING`. Un `SELECT`
//! préalable laisserait passer les deux (principe VIII).
//!
//! # ON NE DEMANDE PAS CE QU'ON A DÉJÀ
//!
//! Une personne qui détient l'accès n'ouvre pas de demande : elle recevrait un
//! courriel pour un droit acquis, et la file du back-office porterait une
//! décision sans objet.

use kernel::auth::{self, Scope};
use kernel::context::RequestContext;
use kernel::error::{ApiError, ErrorCode, Result};
use uuid::Uuid;

use crate::domain::access::AccessRequestView;
use crate::domain::permissions::SPACE_ACCESS;
use crate::domain::requests::CreateAccessRequestPayload;
use crate::repo::requests;
use crate::state::NegotiationState;

pub async fn demander(
    state: &NegotiationState,
    ctx: &RequestContext,
    person_id: Uuid,
    charge: &CreateAccessRequestPayload,
) -> Result<AccessRequestView> {
    let (scope_type, space_id) = match charge.space_id {
        Some(id) => ("negotiation_space", Some(id)),
        None => ("global", None),
    };

    let portee = match space_id {
        Some(id) => Scope::NegotiationSpace(id),
        None => Scope::Global,
    };

    if auth::has_permission(state.pool(), person_id, SPACE_ACCESS, portee).await? {
        return Err(ApiError::with_message(
            ErrorCode::Conflict,
            "Vous avez déjà l'accès aux modules réservés : il n'y a rien à demander.",
        ));
    }

    let message = charge
        .message
        .as_deref()
        .map(str::trim)
        .filter(|m| !m.is_empty());

    let mut tx = state.db().write(ctx).await?;
    let demande = requests::ouvrir(&mut tx, person_id, scope_type, space_id, None, message).await?;
    tx.commit().await?;

    Ok(demande)
}

/// La personne retire sa demande.
///
/// Une demande qui n'est pas la sienne se refuse **comme une demande
/// inexistante** : la forme de la réponse ne dit pas qu'elle existe ailleurs
/// (principe IX).
pub async fn annuler(
    state: &NegotiationState,
    ctx: &RequestContext,
    person_id: Uuid,
    request_id: Uuid,
) -> Result<()> {
    let mut tx = state.db().write(ctx).await?;

    if requests::annuler(&mut tx, request_id, person_id).await? {
        tx.commit().await?;
        return Ok(());
    }

    // Rien n'a bougé : soit la demande est déjà tranchée, soit elle n'est pas à
    // cette personne. Les deux réponses diffèrent, et c'est voulu — savoir que
    // sa propre demande a été tranchée est utile ; savoir qu'une autre existe
    // ne l'est pas.
    let sienne = requests::appartient_a(&mut tx, request_id, person_id).await?;
    tx.rollback().await?;

    Err(if sienne {
        ApiError::new(ErrorCode::NegotiationAccessRequestDecided)
    } else {
        ApiError::not_found()
    })
}
