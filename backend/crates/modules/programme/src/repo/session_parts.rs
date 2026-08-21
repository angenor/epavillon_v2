//! Ce qui accompagne une séance — intervenants, organisations, journées
//! spéciales.
//!
//! # La ligne du porteur n'est JAMAIS écrite ici
//!
//! `tg_sessions_sync_lead_organization()` la pose et la déplace, exactement
//! comme sur les propositions. Ce fichier ne recopie que les **co-organisations**
//! — co-organisateurs, partenaires, soutiens : écrire la ligne du porteur à son
//! tour ne produirait rien de plus, et ferait croire à la relecture que le
//! service en décide.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use sqlx::PgExecutor;
use uuid::Uuid;

use crate::domain::ids::SessionId;

/// Recopier les intervenants du dossier sur la séance.
///
/// **Recopiés, puis modifiables** : les intervenants annoncés dans le dossier ne
/// sont pas toujours ceux du jour, et c'est ce que le modèle prévoit. La
/// modification appartient à un écran qui n'existe pas encore ; la recopie, elle,
/// est ce qui donne au planificateur son décompte d'intervenants.
pub async fn recopier_les_intervenants(
    conn: &mut PgConnection,
    proposal_id: Uuid,
    session_id: Uuid,
) -> Result<u64> {
    let poses = sqlx::query!(
        "INSERT INTO programme.session_speakers
             (session_id, person_id, role, job_title_snapshot,
              organization_snapshot, bio, confirmed_at, sort_order)
         SELECT $2, s.person_id, s.role, s.job_title_snapshot,
                s.organization_snapshot, s.bio, s.confirmed_at, s.sort_order
           FROM programme.proposal_speakers s
          WHERE s.proposal_id = $1
         ON CONFLICT (session_id, person_id, role) DO NOTHING",
        proposal_id,
        session_id
    )
    .execute(conn)
    .await?
    .rows_affected();

    Ok(poses)
}

/// Recopier les co-organisations — **tout sauf le porteur**.
pub async fn recopier_les_coorganisations(
    conn: &mut PgConnection,
    proposal_id: Uuid,
    session_id: Uuid,
) -> Result<u64> {
    let poses = sqlx::query!(
        "INSERT INTO programme.session_organizations
             (session_id, organization_id, role, sort_order)
         SELECT $2, o.organization_id, o.role, o.sort_order
           FROM programme.proposal_organizations o
          WHERE o.proposal_id = $1 AND o.role <> 'lead'
         ON CONFLICT (session_id, organization_id) DO NOTHING",
        proposal_id,
        session_id
    )
    .execute(conn)
    .await?
    .rows_affected();

    Ok(poses)
}

// -----------------------------------------------------------------------------
// Les lectures de la fiche d'une séance
// -----------------------------------------------------------------------------

/// Les intervenants d'une séance — `SessionSpeaker[]`.
pub async fn intervenants<'e>(
    executor: impl PgExecutor<'e>,
    session_id: SessionId,
) -> Result<Vec<serde_json::Value>> {
    let lignes = sqlx::query_scalar!(
        r#"SELECT to_jsonb(s) AS "ligne!"
             FROM programme.session_speakers s
            WHERE s.session_id = $1
            ORDER BY s.sort_order, s.created_at"#,
        session_id.as_uuid()
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes)
}

/// Les organisations d'une séance — `SessionOrganization[]`, porteur compris.
pub async fn organisations<'e>(
    executor: impl PgExecutor<'e>,
    session_id: SessionId,
) -> Result<Vec<serde_json::Value>> {
    let lignes = sqlx::query_scalar!(
        r#"SELECT to_jsonb(o) AS "ligne!"
             FROM programme.session_organizations o
            WHERE o.session_id = $1
            ORDER BY o.role = 'lead' DESC, o.sort_order"#,
        session_id.as_uuid()
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes)
}

/// Les rattachements d'une séance — `SessionTrack[]`.
pub async fn fils<'e>(
    executor: impl PgExecutor<'e>,
    session_id: SessionId,
) -> Result<Vec<serde_json::Value>> {
    let lignes = sqlx::query_scalar!(
        r#"SELECT to_jsonb(t) AS "ligne!"
             FROM programme.session_tracks t
            WHERE t.session_id = $1
            ORDER BY t.sort_order, t.added_at"#,
        session_id.as_uuid()
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes)
}

/// **Remplacer** la liste des rattachements d'une séance.
///
/// Le geste est un remplacement et non un ajout : l'écran envoie la liste
/// entière, et un fil retiré doit disparaître. L'acteur vient de la session
/// appelante — le contrat du front envoie encore un identifiant de personne, et
/// il est ignoré comme partout ailleurs.
///
/// Le refus d'un fil d'une autre édition appartient au déclencheur
/// `tg_check_session_track_event()` : le service le **traduit**, il ne le rejoue
/// pas.
pub async fn remplacer_les_fils(
    conn: &mut PgConnection,
    session_id: SessionId,
    fils: &[Uuid],
    acteur: Option<Uuid>,
) -> std::result::Result<(), sqlx::Error> {
    sqlx::query!(
        "DELETE FROM programme.session_tracks
          WHERE session_id = $1 AND NOT (track_id = ANY($2))",
        session_id.as_uuid(),
        fils
    )
    .execute(&mut *conn)
    .await?;

    if fils.is_empty() {
        return Ok(());
    }

    sqlx::query!(
        "INSERT INTO programme.session_tracks (session_id, track_id, sort_order, added_by)
         SELECT $1, f.track_id, f.rang, $3
           FROM unnest($2::uuid[]) WITH ORDINALITY AS f(track_id, rang)
         ON CONFLICT (session_id, track_id)
         DO UPDATE SET sort_order = EXCLUDED.sort_order",
        session_id.as_uuid(),
        fils,
        acteur
    )
    .execute(conn)
    .await?;

    Ok(())
}

/// Les identifiants des fils d'une séance — ce que `PlannerSession.track_ids`
/// porte.
pub async fn identifiants_de_fils<'e>(
    executor: impl PgExecutor<'e>,
    session_id: SessionId,
) -> Result<Vec<Uuid>> {
    let ids = sqlx::query_scalar!(
        "SELECT track_id FROM programme.session_tracks
          WHERE session_id = $1 ORDER BY sort_order",
        session_id.as_uuid()
    )
    .fetch_all(executor)
    .await?;

    Ok(ids)
}
