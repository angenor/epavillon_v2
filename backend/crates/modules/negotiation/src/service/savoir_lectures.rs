//! Une entrée de FAQ ouverte sur un téléphone : la matière des « plus lues ».

use kernel::context::RequestContext;
use kernel::error::Result;
use uuid::Uuid;

use crate::repo::savoir_lectures as repo;
use crate::state::NegotiationState;

pub async fn compter_une_lecture(
    state: &NegotiationState,
    ctx: &RequestContext,
    id: Uuid,
) -> Result<()> {
    let mut tx = state.db().write(ctx).await?;
    repo::compter(&mut tx, id).await?;
    tx.commit().await?;
    Ok(())
}
