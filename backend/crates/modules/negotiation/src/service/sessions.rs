//! Les sessions officielles d'une édition. Coupé, la liste est vide quel que
//! soit l'âge des lignes en base : le client efface alors sa copie (FR-039).

use kernel::error::{ApiError, ErrorCode, Result};
use time::OffsetDateTime;

use crate::domain::sessions::{EditionServie, EtatServi, MotifDeCoupure, OfficialSessions};
use crate::repo::sessions;
use crate::state::NegotiationState;

pub async fn de_ledition(state: &NegotiationState, slug: &str) -> Result<OfficialSessions> {
    let mut conn = state.pool().acquire().await?;
    let edition = sessions::edition(&mut conn, slug)
        .await?
        .ok_or_else(|| ApiError::new(ErrorCode::NegotiationEditionUnknown).field("edition"))?;

    let (etat, motif, lignes) = if edition.serving {
        let lignes =
            sessions::sessions(&mut conn, edition.event_id, edition.last_success_at).await?;
        (EtatServi::Serving, None, lignes)
    } else if !edition.is_enabled || edition.last_success_at.is_none() {
        (EtatServi::Cut, Some(MotifDeCoupure::Disabled), Vec::new())
    } else {
        (
            EtatServi::Cut,
            Some(MotifDeCoupure::Unreachable),
            Vec::new(),
        )
    };

    Ok(OfficialSessions {
        edition: EditionServie {
            slug: edition.slug,
            timezone: edition.timezone,
            city: edition.city,
        },
        official_programme_url: edition.official_programme_url,
        state: etat,
        cut_reason: motif,
        failing_since: edition.failing_since,
        read_at: edition.last_success_at,
        server_time: OffsetDateTime::now_utc(),
        sessions: lignes,
    })
}
