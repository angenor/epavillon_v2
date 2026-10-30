//! Les lectures hors schéma de l'éditeur de la page d'un fil (30/10), à part de
//! `repo/cross.rs` pour qu'il reste sous le plafond de mille lignes. Même règle :
//! **aucune ligne n'écrit**.
//!
//! | # | Lecture | Question de **ce** module |
//! |---|---|---|
//! | 10 | `org`, `reference`, `media` : nom, sigle, logo d'un partenaire, libellé de son rôle | « qui sont les partenaires de ce fil ? » |
//! | 11 | `programme` : rattachements séance–fil et leurs séances | « quelles activités ce fil compose-t-il, dans quelle partie ? » |

use kernel::error::Result;
use sqlx::PgExecutor;

use crate::domain::ids::TrackId;
use crate::domain::track_page::{TrackCompositionEntry, TrackPartner};

/// Les partenaires d'un fil, dans leur ordre, résolus pour l'affichage.
pub async fn partenaires<'e>(
    executor: impl PgExecutor<'e>,
    track: TrackId,
) -> Result<Vec<TrackPartner>> {
    let lignes = sqlx::query_as!(
        TrackPartner,
        r#"SELECT p.organization_id, o.legal_name AS name, o.acronym,
                  media.attached_image('org', 'organizations', o.id, 'logo') AS logo,
                  t.code AS role_code, t.label AS "role_label!: serde_json::Value",
                  p.sort_order
             FROM event.programme_track_partners p
             JOIN org.organizations o ON o.id = p.organization_id
             JOIN reference.taxonomy_terms t ON t.id = p.role_term_id
            WHERE p.track_id = $1
            ORDER BY p.sort_order, o.legal_name"#,
        track.as_uuid()
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes)
}

/// La composition d'un fil : chaque activité rattachée, publiée ou non, avec sa
/// partie, son temps fort et son rang.
pub async fn composition<'e>(
    executor: impl PgExecutor<'e>,
    track: TrackId,
) -> Result<Vec<TrackCompositionEntry>> {
    let lignes = sqlx::query_as!(
        TrackCompositionEntry,
        r#"SELECT s.id AS session_id, s.slug::text AS "slug!",
                  s.title AS "title!: serde_json::Value",
                  s.starts_at, s.ends_at, s.status::text AS "status!",
                  (s.published_at IS NOT NULL) AS "is_published!",
                  st.part_id, st.is_highlight, st.sort_order
             FROM programme.session_tracks st
             JOIN programme.sessions s ON s.id = st.session_id
            WHERE st.track_id = $1
            ORDER BY s.starts_at, st.sort_order, s.id"#,
        track.as_uuid()
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes)
}
