//! Les réunions de la Francophonie d'une édition — lecture publique.

use kernel::error::{ApiError, ErrorCode, Result};
use time::OffsetDateTime;

use crate::domain::meetings::FrancophoneMeetings;
use crate::domain::sessions::EditionServie;
use crate::repo::meetings;
use crate::state::NegotiationState;

pub async fn de_ledition(state: &NegotiationState, slug: &str) -> Result<FrancophoneMeetings> {
    let mut conn = state.pool().acquire().await?;
    let edition = meetings::edition(&mut conn, slug.trim())
        .await?
        .ok_or_else(|| ApiError::new(ErrorCode::NegotiationEditionUnknown).field("edition"))?;
    let reunions = meetings::publiees(&mut conn, edition.event_id).await?;
    let maintenant = OffsetDateTime::now_utc();
    Ok(FrancophoneMeetings {
        edition: EditionServie {
            slug: edition.slug,
            timezone: edition.timezone,
            city: edition.city,
        },
        read_at: maintenant,
        server_time: maintenant,
        meetings: reunions,
    })
}
