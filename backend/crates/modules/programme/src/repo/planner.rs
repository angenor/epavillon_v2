//! Les séances telles que le planificateur les manipule — **tout ce qu'un bloc
//! et une carte affichent, déjà joint**.
//!
//! # Pourquoi cette requête est large
//!
//! C'est la même règle que `v_public_schedule`, qui n'aide pas ici : elle ne
//! montre que le publié, et cet écran travaille d'abord sur ce qui ne l'est pas.
//! Sans les jointures qui suivent, chaque bloc du calendrier coûterait une
//! requête pour son organisation, une pour sa note et une pour ses thématiques —
//! quarante blocs, cent vingt requêtes.
//!
//! # Ce qui vient du dossier vient de `programme`
//!
//! Numéro, note consolidée, durée souhaitée, créneau souhaité, contraintes de
//! programmation : ce sont des colonnes de `programme.proposals`, **du même
//! schéma**. Rien ici ne franchit une frontière que `repo/cross/` ne déclare
//! déjà.

use kernel::error::Result;
use sqlx::PgExecutor;

use crate::domain::ids::EventId;
use crate::domain::sessions::PlannerSession;

/// Les séances d'une édition, **placées ou non**.
///
/// Le tri est chronologique : c'est l'ordre du calendrier, et le panneau « à
/// placer » le retriera par note côté écran — le contrat du front en porte les
/// quatre clés.
pub async fn seances_de_ledition<'e>(
    executor: impl PgExecutor<'e>,
    event_id: EventId,
) -> Result<Vec<PlannerSession>> {
    let lignes = sqlx::query!(
        r#"SELECT s.id, s.event_id, s.proposal_id, s.event_day_id,
                  s.title, s.slug::text AS "slug!", s.summary,
                  s.status::text AS "status!", s.format::text AS "format!",
                  s.starts_at, s.ends_at, s.timezone::text AS "timezone!",
                  -- Les quatre jointures sont EXTERNES : sans salle, sans
                  -- organisation, sans pays ou sans dossier, la colonne est
                  -- nulle. SQLx ne le déduit pas d'un LEFT JOIN — il lit la
                  -- nullité de la colonne d'origine —, d'où le `?`.
                  s.room_id, r.name AS "room_name?",
                  s.enforce_room_exclusivity, s.location_note,
                  s.organization_id, o.legal_name AS "organization_name?",
                  o.acronym AS "organization_acronym?",
                  c.iso2 AS "organization_country_code?",
                  p.reference_code AS "reference_code?",
                  p.average_score::float8 AS "average_score?",
                  p.duration_minutes::int4 AS "requested_duration_minutes?",
                  p.preferred_start_at,
                  p.scheduling_constraints,
                  s.is_streamed, s.broadcast_channel_id, s.published_at,
                  COALESCE((SELECT array_agg(st.track_id ORDER BY st.sort_order)
                              FROM programme.session_tracks st
                             WHERE st.session_id = s.id), '{}') AS "track_ids!",
                  reference.term_badges('programme', 'sessions', s.id, 'activity_theme')
                      AS "themes!",
                  (SELECT count(*) FROM programme.session_speakers sp
                    WHERE sp.session_id = s.id) AS "speaker_count!"
             FROM programme.sessions s
             LEFT JOIN event.rooms r         ON r.id = s.room_id
             LEFT JOIN org.organizations o   ON o.id = s.organization_id
             LEFT JOIN reference.countries c ON c.id = o.country_id
             LEFT JOIN programme.proposals p ON p.id = s.proposal_id
            WHERE s.event_id = $1
            ORDER BY s.starts_at, s.id"#,
        event_id.as_uuid()
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| PlannerSession {
            id: l.id,
            event_id: l.event_id,
            proposal_id: l.proposal_id,
            event_day_id: l.event_day_id,
            title: l.title,
            slug: l.slug,
            summary: l.summary,
            status: l.status,
            format: l.format,
            starts_at: l.starts_at,
            ends_at: l.ends_at,
            timezone: l.timezone,
            room_id: l.room_id,
            room_name: l.room_name,
            enforce_room_exclusivity: l.enforce_room_exclusivity,
            location_note: l.location_note,
            organization_id: l.organization_id,
            organization_name: l.organization_name,
            organization_acronym: l.organization_acronym,
            organization_country_code: l.organization_country_code,
            reference_code: l.reference_code,
            average_score: l.average_score,
            requested_duration_minutes: l.requested_duration_minutes,
            preferred_start_at: l.preferred_start_at,
            scheduling_constraints: l.scheduling_constraints,
            is_streamed: l.is_streamed,
            broadcast_channel_id: l.broadcast_channel_id,
            track_ids: l.track_ids,
            themes: l.themes,
            speaker_count: l.speaker_count,
            published_at: l.published_at,
        })
        .collect())
}

/// Une seule séance, dans la **même** forme — ce que rend une écriture du
/// planificateur.
///
/// La lecture passe par la liste de l'édition plutôt que par une seconde requête
/// : deux requêtes pour la même forme, c'est deux occasions de diverger sur ce
/// qu'un bloc affiche, et le contrat exige qu'elles soient identiques.
pub async fn seance_du_planificateur<'e>(
    executor: impl PgExecutor<'e>,
    event_id: EventId,
    session_id: uuid::Uuid,
) -> Result<Option<PlannerSession>> {
    Ok(seances_de_ledition(executor, event_id)
        .await?
        .into_iter()
        .find(|s| s.id == session_id))
}
