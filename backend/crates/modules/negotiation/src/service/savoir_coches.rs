//! Les coches du parcours « Ma première COP » de la personne connectée.

use kernel::context::RequestContext;
use kernel::error::{ApiError, ErrorCode, Result};
use uuid::Uuid;

use crate::domain::savoir::MyPathway;
use crate::repo::savoir_coches as repo;
use crate::state::NegotiationState;

pub async fn coches(state: &NegotiationState, personne: Uuid) -> Result<(MyPathway, String)> {
    let mut conn = state.pool().acquire().await?;
    let step_ids = repo::lister(&mut conn, personne).await?;
    let empreinte = kernel::empreinte::de(
        &step_ids
            .iter()
            .map(Uuid::to_string)
            .collect::<Vec<_>>()
            .join("\n"),
    );
    Ok((MyPathway { step_ids }, empreinte))
}

/// Idempotent. Une étape inconnue ou non publiée : `404`.
pub async fn cocher(
    state: &NegotiationState,
    ctx: &RequestContext,
    personne: Uuid,
    etape: Uuid,
) -> Result<()> {
    let mut tx = state.db().write(ctx).await?;
    if !repo::publiee(&mut tx, etape).await? {
        return Err(ApiError::new(ErrorCode::NegotiationPathwayStepNotFound));
    }
    repo::cocher(&mut tx, personne, etape).await?;
    tx.commit().await?;
    Ok(())
}

/// Idempotent, même sur une étape retirée : décocher ne se refuse jamais.
pub async fn decocher(
    state: &NegotiationState,
    ctx: &RequestContext,
    personne: Uuid,
    etape: Uuid,
) -> Result<()> {
    let mut tx = state.db().write(ctx).await?;
    repo::decocher(&mut tx, personne, etape).await?;
    tx.commit().await?;
    Ok(())
}
