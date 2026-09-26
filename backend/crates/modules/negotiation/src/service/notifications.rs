//! L'accord « Notifications » : lire, basculer. Rejouer la même valeur
//! n'écrit rien ; seule une bascule laisse une preuve.

use kernel::context::RequestContext;
use kernel::error::Result;
use uuid::Uuid;

use crate::domain::notifications::NotificationSettings;
use crate::repo::notifications as depot;
use crate::state::NegotiationState;

fn version() -> String {
    kernel::legal::version("privacy").to_owned()
}

pub async fn reglage(state: &NegotiationState, person_id: Uuid) -> Result<NotificationSettings> {
    let mut conn = state.pool().acquire().await?;
    Ok(NotificationSettings {
        email: depot::accord(&mut conn, person_id).await?,
        version: version(),
    })
}

pub async fn regler(
    state: &NegotiationState,
    ctx: &RequestContext,
    person_id: Uuid,
    email: bool,
) -> Result<NotificationSettings> {
    let version = version();
    let mut tx = state.db().write(ctx).await?;
    depot::verrouiller(&mut tx, person_id).await?;
    if depot::accord(&mut tx, person_id).await? != email {
        depot::consigner(&mut tx, person_id, email, &version).await?;
    }
    tx.commit().await?;
    Ok(NotificationSettings { email, version })
}
