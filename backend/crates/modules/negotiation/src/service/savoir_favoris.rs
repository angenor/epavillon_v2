//! Les termes favoris de la personne connectée.

use kernel::context::RequestContext;
use kernel::error::{ApiError, ErrorCode, Result};
use uuid::Uuid;

use crate::domain::savoir::MyGlossaryFavorites;
use crate::repo::savoir_favoris as repo;
use crate::state::NegotiationState;

pub async fn favoris(
    state: &NegotiationState,
    personne: Uuid,
) -> Result<(MyGlossaryFavorites, String)> {
    let mut conn = state.pool().acquire().await?;
    let entry_ids = repo::lister(&mut conn, personne).await?;
    let empreinte = kernel::empreinte::de(
        &entry_ids
            .iter()
            .map(Uuid::to_string)
            .collect::<Vec<_>>()
            .join("\n"),
    );
    Ok((MyGlossaryFavorites { entry_ids }, empreinte))
}

/// Idempotent. Un brouillon n'existe pas pour le lecteur : `404`.
pub async fn poser_un_favori(
    state: &NegotiationState,
    ctx: &RequestContext,
    personne: Uuid,
    id: Uuid,
) -> Result<()> {
    let mut tx = state.db().write(ctx).await?;
    if !repo::servie(&mut tx, id).await? {
        return Err(ApiError::new(ErrorCode::NegotiationGlossaryNotFound));
    }
    repo::poser(&mut tx, personne, id).await?;
    tx.commit().await?;
    Ok(())
}

pub async fn retirer_un_favori(
    state: &NegotiationState,
    ctx: &RequestContext,
    personne: Uuid,
    id: Uuid,
) -> Result<()> {
    let mut tx = state.db().write(ctx).await?;
    repo::retirer(&mut tx, personne, id).await?;
    tx.commit().await?;
    Ok(())
}
