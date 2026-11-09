//! Les intervenants en ligne (08/11) : ce que le module Live lit hors schéma
//! pour leur donner une place Zoom. `programme.session_speakers` et
//! `programme.sessions` (jeton du lien, échéances), `identity.people` (nom,
//! adresse), `platform.settings` (envoi d'office). Lecture seule.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use time::OffsetDateTime;
use uuid::Uuid;

pub struct IntervenantJoignable {
    pub speaker_id: Uuid,
    pub person_id: Uuid,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub display_name: String,
}

/// Les intervenants d'une séance qui ont une adresse ; `ids` les restreint.
pub async fn joignables(
    conn: &mut PgConnection,
    session_id: Uuid,
    ids: Option<&[Uuid]>,
) -> Result<Vec<IntervenantJoignable>> {
    let lignes = sqlx::query_as!(
        IntervenantJoignable,
        r#"SELECT d.id AS speaker_id, p.id AS person_id, p.primary_email::text AS "email!",
                  p.first_name, p.last_name, p.display_name AS "display_name!"
             FROM programme.session_speakers d
             JOIN identity.people p ON p.id = d.person_id AND p.status <> 'anonymized'
            WHERE d.session_id = $1 AND ($2::uuid[] IS NULL OR d.id = ANY($2))
            ORDER BY d.sort_order, d.created_at"#,
        session_id,
        ids as Option<&[Uuid]>
    )
    .fetch_all(conn)
    .await?;
    Ok(lignes)
}

/// La séance du lien d'inscription, s'il sert encore (même règle que le module
/// Programmation : publiée, diffusée, pas annulée, pas finie).
pub async fn seance_du_lien(conn: &mut PgConnection, jeton: &str) -> Result<Option<Uuid>> {
    let id = sqlx::query_scalar!(
        "SELECT s.id FROM programme.sessions s
          WHERE s.speaker_link_token = $1
            AND s.published_at IS NOT NULL AND s.status <> 'cancelled' AND s.ends_at > now()
            AND s.is_streamed
            AND COALESCE((SELECT t.is_streamed FROM event.programme_tracks t
                           WHERE t.id = programme.day_track_of(s.id)), true)",
        jeton
    )
    .fetch_optional(conn)
    .await?;
    Ok(id)
}

/// Un intervenant de cette séance.
pub async fn est_de_la_seance(
    conn: &mut PgConnection,
    session_id: Uuid,
    speaker_id: Uuid,
) -> Result<bool> {
    let oui = sqlx::query_scalar!(
        r#"SELECT EXISTS (SELECT 1 FROM programme.session_speakers
                           WHERE id = $1 AND session_id = $2) AS "oui!""#,
        speaker_id,
        session_id
    )
    .fetch_one(conn)
    .await?;
    Ok(oui)
}

pub async fn envoi_doffice(conn: &mut PgConnection) -> Result<bool> {
    let oui = sqlx::query_scalar!(
        r#"SELECT COALESCE((SELECT value = 'true'::jsonb FROM platform.settings
                             WHERE key = 'live.speaker_links.auto'), false) AS "oui!""#
    )
    .fetch_one(conn)
    .await?;
    Ok(oui)
}

pub struct SeanceAEcheance {
    pub id: Uuid,
    pub has_owner: bool,
}

/// Les activités publiées et diffusées qui commencent avant `jusqua` et pas
/// encore commencées : l'avant-veille pour le porteur, la veille pour les
/// intervenants.
pub async fn seances_avant(
    conn: &mut PgConnection,
    jusqua: OffsetDateTime,
) -> Result<Vec<SeanceAEcheance>> {
    let lignes = sqlx::query_as!(
        SeanceAEcheance,
        r#"SELECT s.id,
                  (s.proposal_id IS NOT NULL OR EXISTS (
                       SELECT 1 FROM programme.session_proposals sp WHERE sp.session_id = s.id
                  )) AS "has_owner!"
             FROM programme.sessions s
            WHERE s.published_at IS NOT NULL AND s.status IN ('scheduled', 'live')
              AND s.starts_at > now() AND s.starts_at <= $1
              AND s.is_streamed
              AND COALESCE((SELECT t.is_streamed FROM event.programme_tracks t
                             WHERE t.id = programme.day_track_of(s.id)), true)"#,
        jusqua
    )
    .fetch_all(conn)
    .await?;
    Ok(lignes)
}
