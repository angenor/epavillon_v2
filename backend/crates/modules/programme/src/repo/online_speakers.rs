//! Les intervenants en ligne (08/11) : la ligne d'intervenant que pose le lien
//! d'inscription ou la régie. Les lectures hors schéma sont dans
//! `cross::intervenants`.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use uuid::Uuid;

use crate::domain::online_speakers::IntervenantSaisi;

/// L'intervenant de la séance pour cette personne : la ligne qu'elle a déjà
/// (venue du dossier ou d'une inscription précédente), sinon une ligne neuve.
/// Une ligne du dossier garde ce que l'organisation y a écrit ; celle d'une
/// inscription précédente prend la saisie la plus récente. `afficher` ne fait
/// qu'allumer l'affichage, jamais l'éteindre.
pub async fn poser(
    conn: &mut PgConnection,
    session_id: Uuid,
    person_id: Uuid,
    saisie: &IntervenantSaisi,
    source: &str,
    afficher: bool,
) -> Result<Uuid> {
    let existant = sqlx::query!(
        "SELECT id, source FROM programme.session_speakers
          WHERE session_id = $1 AND person_id = $2
          ORDER BY sort_order, created_at LIMIT 1
          FOR UPDATE",
        session_id,
        person_id
    )
    .fetch_optional(&mut *conn)
    .await?;

    if let Some(ligne) = existant {
        sqlx::query!(
            "UPDATE programme.session_speakers
                SET is_listed = is_listed OR $2,
                    job_title_snapshot = CASE WHEN source = 'dossier' THEN job_title_snapshot ELSE $3 END,
                    organization_snapshot = CASE WHEN source = 'dossier' THEN organization_snapshot ELSE $4 END,
                    bio = CASE WHEN source = 'dossier' OR $5::text IS NULL THEN bio
                               ELSE jsonb_build_object('fr', $5::text) END,
                    country_id = COALESCE((SELECT c.id FROM reference.countries c WHERE c.iso2 = $6), country_id)
              WHERE id = $1",
            ligne.id,
            afficher,
            saisie.job_title,
            saisie.organization,
            saisie.bio,
            saisie.country_iso2,
        )
        .execute(&mut *conn)
        .await?;
        return Ok(ligne.id);
    }

    let id = sqlx::query_scalar!(
        "INSERT INTO programme.session_speakers
             (session_id, person_id, role, job_title_snapshot, organization_snapshot, bio,
              sort_order, source, is_listed, country_id)
         VALUES ($1, $2, 'speaker', $3, $4,
                 CASE WHEN $5::text IS NULL THEN NULL ELSE jsonb_build_object('fr', $5::text) END,
                 (SELECT COALESCE(max(sort_order), 0) + 1 FROM programme.session_speakers WHERE session_id = $1),
                 $6, $7, (SELECT c.id FROM reference.countries c WHERE c.iso2 = $8))
         RETURNING id",
        session_id,
        person_id,
        saisie.job_title,
        saisie.organization,
        saisie.bio,
        source,
        afficher,
        saisie.country_iso2,
    )
    .fetch_one(conn)
    .await?;
    Ok(id)
}

/// « Afficher sur la page » : faux si l'intervenant n'est pas de cette séance.
pub async fn afficher(conn: &mut PgConnection, session_id: Uuid, speaker_id: Uuid) -> Result<bool> {
    let fait = sqlx::query!(
        "UPDATE programme.session_speakers SET is_listed = true
          WHERE id = $1 AND session_id = $2",
        speaker_id,
        session_id
    )
    .execute(conn)
    .await?
    .rows_affected();
    Ok(fait == 1)
}
