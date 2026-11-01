//! Les séances communes : plusieurs dossiers retenus réunis dans un créneau,
//! sous un titre commun (`programme.session_proposals`, 075 § 2 quater).
//!
//! Les écritures rendent l'erreur brute de PostgreSQL : le service traduit.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use uuid::Uuid;

/// Ce que le service vérifie d'un dossier avant de le réunir.
#[derive(Debug, Clone)]
pub struct DossierAReunir {
    pub id: Uuid,
    pub event_id: Uuid,
    pub status: String,
    pub deja_reuni: bool,
    /// Séances non annulées du dossier.
    pub seances: i64,
    /// Séances en cours ou terminées : elles ont eu lieu, on ne les retire pas.
    pub seances_closes: i64,
    /// Inscriptions vivantes (tout sauf `cancelled`) sur ses séances.
    pub inscrits: i64,
}

/// Les dossiers demandés, **verrouillés** : deux réunions simultanées du même
/// dossier se sérialisent, et la seconde voit `deja_reuni`.
pub async fn dossiers(conn: &mut PgConnection, ids: &[Uuid]) -> Result<Vec<DossierAReunir>> {
    let lignes = sqlx::query!(
        r#"SELECT p.id, p.event_id, p.status::text AS "status!",
                  EXISTS (SELECT 1 FROM programme.session_proposals sp
                           WHERE sp.proposal_id = p.id) AS "deja_reuni!",
                  (SELECT count(*) FROM programme.sessions s
                    WHERE s.proposal_id = p.id AND s.status <> 'cancelled') AS "seances!",
                  (SELECT count(*) FROM programme.sessions s
                    WHERE s.proposal_id = p.id AND s.status IN ('live', 'completed'))
                      AS "seances_closes!",
                  (SELECT count(*) FROM programme.registrations r
                     JOIN programme.sessions s ON s.id = r.session_id
                    WHERE s.proposal_id = p.id AND s.status <> 'cancelled'
                      AND r.status <> 'cancelled') AS "inscrits!"
             FROM programme.proposals p
            WHERE p.id = ANY($1) AND p.deleted_at IS NULL
            FOR UPDATE OF p"#,
        ids
    )
    .fetch_all(conn)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| DossierAReunir {
            id: l.id,
            event_id: l.event_id,
            status: l.status,
            deja_reuni: l.deja_reuni,
            seances: l.seances,
            seances_closes: l.seances_closes,
            inscrits: l.inscrits,
        })
        .collect())
}

/// La séance du dossier dont la séance commune reprend le créneau : la
/// première non annulée.
pub async fn seance_modele(conn: &mut PgConnection, proposal_id: Uuid) -> Result<Option<Uuid>> {
    let id = sqlx::query_scalar!(
        "SELECT id FROM programme.sessions
          WHERE proposal_id = $1 AND status <> 'cancelled'
          ORDER BY sequence_number, starts_at
          LIMIT 1",
        proposal_id
    )
    .fetch_optional(conn)
    .await?;

    Ok(id)
}

/// Créer la séance commune **à l'image de la séance modèle** : créneau, salle,
/// format, inscription, diffusion, publication. Sans dossier propre, au titre
/// commun, portée par l'organisation du premier dossier.
pub async fn creer(
    conn: &mut PgConnection,
    modele: Uuid,
    titre: &serde_json::Value,
    slug: &str,
    acteur: Option<Uuid>,
) -> std::result::Result<Option<Uuid>, sqlx::Error> {
    sqlx::query_scalar!(
        "INSERT INTO programme.sessions
             (event_id, proposal_id, event_day_id, organization_id, title, slug,
              status, format, starts_at, ends_at, timezone, room_id, location_note,
              registration_required, registration_opens_at, registration_closes_at,
              capacity, waitlist_enabled, registration_form_id, is_streamed,
              broadcast_channel_id, is_recorded, allows_questions, published_at,
              created_by)
         SELECT s.event_id, NULL, s.event_day_id, s.organization_id, $2,
                $3::text::platform.slug,
                s.status, s.format, s.starts_at, s.ends_at, s.timezone, s.room_id,
                s.location_note, s.registration_required, s.registration_opens_at,
                s.registration_closes_at, s.capacity, s.waitlist_enabled,
                s.registration_form_id, s.is_streamed, s.broadcast_channel_id,
                s.is_recorded, s.allows_questions, s.published_at, $4
           FROM programme.sessions s
          WHERE s.id = $1
         RETURNING id",
        modele,
        titre,
        slug,
        acteur
    )
    .fetch_optional(conn)
    .await
}

/// Rattacher un dossier à la séance commune, ses textes recopiés.
pub async fn attacher(
    conn: &mut PgConnection,
    session_id: Uuid,
    proposal_id: Uuid,
    rang: i16,
    acteur: Option<Uuid>,
) -> std::result::Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO programme.session_proposals
             (session_id, proposal_id, sort_order, title, description, added_by)
         SELECT $1, p.id, $3, p.title, p.detailed_presentation, $4
           FROM programme.proposals p
          WHERE p.id = $2",
        session_id,
        proposal_id,
        rang,
        acteur
    )
    .execute(&mut *conn)
    .await?;

    // Le porteur du dossier, rangé sous lui. Le déclencheur a posé celui de la
    // séance (le premier dossier) sans dossier : on le range. Une organisation
    // déjà présente sous un autre dossier y reste.
    sqlx::query!(
        "INSERT INTO programme.session_organizations
             (session_id, organization_id, role, sort_order, proposal_id)
         SELECT $1, p.organization_id, 'lead', 0, p.id
           FROM programme.proposals p
          WHERE p.id = $2
         ON CONFLICT (session_id, organization_id) DO UPDATE
            SET role = 'lead', proposal_id = EXCLUDED.proposal_id
          WHERE programme.session_organizations.proposal_id IS NULL",
        session_id,
        proposal_id
    )
    .execute(conn)
    .await?;

    Ok(())
}

/// Reporter sur la séance commune les journées spéciales des séances qu'elle
/// remplace : la composition d'une journée est un choix de l'équipe, il ne se
/// perd pas en réunissant.
pub async fn reprendre_les_fils(
    conn: &mut PgConnection,
    session_id: Uuid,
    proposal_id: Uuid,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO programme.session_tracks
             (session_id, track_id, sort_order, is_highlight, added_by, part_id)
         SELECT $1, st.track_id, st.sort_order, st.is_highlight, st.added_by, st.part_id
           FROM programme.session_tracks st
           JOIN programme.sessions s ON s.id = st.session_id
          WHERE s.proposal_id = $2 AND s.status <> 'cancelled'
         ON CONFLICT (session_id, track_id) DO NOTHING",
        session_id,
        proposal_id
    )
    .execute(conn)
    .await?;

    Ok(())
}

/// Retirer les séances propres d'un dossier réuni. Le service a vérifié
/// qu'aucune n'a d'inscrit ni n'a eu lieu ; les annulées restent, elles sont
/// l'histoire du dossier. Les thématiques recopiées partent avec elles.
pub async fn retirer_les_seances(conn: &mut PgConnection, proposal_id: Uuid) -> Result<i64> {
    let retirees = sqlx::query_scalar!(
        r#"WITH retirees AS (
               DELETE FROM programme.sessions s
                WHERE s.proposal_id = $1 AND s.status <> 'cancelled'
            RETURNING s.id
           ), liens AS (
               DELETE FROM reference.entity_terms et
                USING retirees r
                WHERE et.entity_schema = 'programme'
                  AND et.entity_table = 'sessions'
                  AND et.entity_id = r.id
           )
           SELECT count(*) AS "n!" FROM retirees"#,
        proposal_id
    )
    .fetch_one(conn)
    .await?;

    Ok(retirees)
}

/// Ce que séparer doit savoir de la séance commune, verrouillée.
#[derive(Debug, Clone)]
pub struct SeanceCommune {
    pub status: String,
    pub inscrits: i64,
    pub room_id: Option<Uuid>,
    pub starts_at: time::OffsetDateTime,
    pub ends_at: time::OffsetDateTime,
    pub event_day_id: Option<Uuid>,
    /// Les dossiers réunis, dans l'ordre d'affichage. Vide : séance ordinaire.
    pub dossiers: Vec<Uuid>,
}

pub async fn seance_commune(
    conn: &mut PgConnection,
    session_id: Uuid,
) -> Result<Option<SeanceCommune>> {
    let ligne = sqlx::query!(
        r#"SELECT s.status::text AS "status!", s.room_id, s.starts_at, s.ends_at,
                  s.event_day_id,
                  (SELECT count(*) FROM programme.registrations r
                    WHERE r.session_id = s.id AND r.status <> 'cancelled') AS "inscrits!",
                  COALESCE((SELECT array_agg(sp.proposal_id ORDER BY sp.sort_order, sp.added_at)
                              FROM programme.session_proposals sp
                             WHERE sp.session_id = s.id), '{}') AS "dossiers!"
             FROM programme.sessions s
            WHERE s.id = $1
            FOR UPDATE OF s"#,
        session_id
    )
    .fetch_optional(conn)
    .await?;

    Ok(ligne.map(|l| SeanceCommune {
        status: l.status,
        inscrits: l.inscrits,
        room_id: l.room_id,
        starts_at: l.starts_at,
        ends_at: l.ends_at,
        event_day_id: l.event_day_id,
        dossiers: l.dossiers,
    }))
}

/// Supprimer la séance commune ; ses rattachements partent par cascade.
pub async fn supprimer(conn: &mut PgConnection, session_id: Uuid) -> Result<()> {
    sqlx::query!(
        "DELETE FROM reference.entity_terms
          WHERE entity_schema = 'programme' AND entity_table = 'sessions' AND entity_id = $1",
        session_id
    )
    .execute(&mut *conn)
    .await?;
    sqlx::query!("DELETE FROM programme.sessions WHERE id = $1", session_id)
        .execute(conn)
        .await?;
    Ok(())
}

/// Les dossiers réunis d'une séance, pour le planificateur et la page publique.
pub async fn parties<'e>(
    executor: impl sqlx::PgExecutor<'e>,
    session_id: Uuid,
) -> Result<Vec<(Uuid, Option<serde_json::Value>)>> {
    let lignes = sqlx::query!(
        "SELECT proposal_id, description FROM programme.session_proposals
          WHERE session_id = $1 ORDER BY sort_order, added_at",
        session_id
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| (l.proposal_id, l.description))
        .collect())
}
