//! Le profil de la personne connectée : elle seule l'écrit, l'audit garde qui
//! a changé quoi (`app.actor_id` posé par la transaction).

use kernel::context::RequestContext;
use kernel::error::{ApiError, Result};
use uuid::Uuid;

use crate::domain::ids::PersonId;
use crate::domain::person::{PersonView, ProfileChanges};
use crate::repo::people;
use crate::state::IdentityState;

pub async fn update(
    state: &IdentityState,
    ctx: &RequestContext,
    acteur: Uuid,
    cible: Uuid,
    changes: &ProfileChanges,
) -> Result<PersonView> {
    // Soi-même seulement : l'équipe modifie une fiche depuis le back-office.
    if cible != acteur {
        return Err(ApiError::forbidden());
    }

    let mut tx = state.db().write(&ctx.with_actor(acteur)).await?;
    if !people::update_profile(&mut tx, PersonId(acteur), changes).await? {
        tx.rollback().await?;
        return Err(ApiError::not_found());
    }
    tx.commit().await?;

    people::view(state.pool(), PersonId(acteur)).await?.ok_or_else(ApiError::not_found)
}
