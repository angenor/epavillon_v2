//! Le rattachement séance–fil vu depuis la page d'un fil : ce qu'elle publie,
//! et le placement d'une activité dans une partie (30/10) ou sous un moment
//! qui la regroupe (03/11).

use sqlx::postgres::PgConnection;
use sqlx::PgExecutor;
use uuid::Uuid;

use crate::domain::track_page::{TrackSessionPlacement, TrackSessionPlacementPayload};
use kernel::error::Result;

/// Une activité **publiée** rattachée au fil.
pub struct RattachementPublie {
    pub session_id: Uuid,
    pub part_id: Option<Uuid>,
    pub moment_id: Option<Uuid>,
    pub is_highlight: bool,
    pub sort_order: i16,
}

pub async fn rattachements_publies<'e>(
    executor: impl PgExecutor<'e>,
    track_id: Uuid,
) -> Result<Vec<RattachementPublie>> {
    let lignes = sqlx::query_as!(
        RattachementPublie,
        "SELECT st.session_id, st.part_id, st.moment_id, st.is_highlight, st.sort_order
           FROM programme.session_tracks st
           JOIN programme.sessions s ON s.id = st.session_id
          WHERE st.track_id = $1 AND s.published_at IS NOT NULL",
        track_id
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes)
}

/// Placer une activité déjà rattachée : partie, moment qui la regroupe, temps
/// fort, rang. `None` : la séance n'est pas rattachée à ce fil. Une partie ou
/// un moment d'un autre fil est refusé par les clés composites
/// `xmod_fk_session_tracks_part` et `xmod_fk_session_tracks_moment`.
pub async fn placer(
    conn: &mut PgConnection,
    session_id: Uuid,
    track_id: Uuid,
    p: &TrackSessionPlacementPayload,
) -> std::result::Result<Option<TrackSessionPlacement>, sqlx::Error> {
    sqlx::query_as!(
        TrackSessionPlacement,
        "UPDATE programme.session_tracks
            SET part_id = $3, moment_id = $4, is_highlight = $5, sort_order = $6
          WHERE session_id = $1 AND track_id = $2
      RETURNING session_id, track_id, part_id, moment_id, is_highlight, sort_order",
        session_id,
        track_id,
        p.part_id,
        p.moment_id,
        p.is_highlight,
        p.sort_order
    )
    .fetch_optional(conn)
    .await
}
