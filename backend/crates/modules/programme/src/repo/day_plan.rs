//! Le jour J d'un dossier retenu : points focaux, besoins techniques,
//! publications (070 § 4 bis).

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use sqlx::PgExecutor;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::day_plan::{FocalPoint, TechnicalNeeds};
use crate::domain::ids::ProposalId;

/// Ce que les gardes du jour J lisent du dossier et de son édition.
pub struct DossierDuJourJ {
    pub event_id: Uuid,
    pub organization_id: Uuid,
    pub submitted_by: Uuid,
    pub status: String,
    /// Fin passée, ou édition close (`completed`).
    pub edition_terminee: bool,
}

pub async fn dossier<'e>(
    executor: impl PgExecutor<'e>,
    dossier: ProposalId,
) -> Result<Option<DossierDuJourJ>> {
    let ligne = sqlx::query!(
        r#"SELECT p.event_id, p.organization_id, p.submitted_by,
                  p.status::text AS "status!",
                  (e.ends_at < now() OR e.status = 'completed') AS "edition_terminee!"
             FROM programme.proposals p
             JOIN event.events e ON e.id = p.event_id
            WHERE p.id = $1 AND p.deleted_at IS NULL"#,
        dossier.as_uuid()
    )
    .fetch_optional(executor)
    .await?;

    Ok(ligne.map(|l| DossierDuJourJ {
        event_id: l.event_id,
        organization_id: l.organization_id,
        submitted_by: l.submitted_by,
        status: l.status,
        edition_terminee: l.edition_terminee,
    }))
}

/// Les points focaux enregistrés, dans l'ordre.
pub async fn points_focaux<'e>(
    executor: impl PgExecutor<'e>,
    dossier: ProposalId,
) -> Result<Vec<FocalPoint>> {
    let lignes = sqlx::query!(
        "SELECT id, speaker_id, person_id, full_name, phone, whatsapp
           FROM programme.proposal_focal_points
          WHERE proposal_id = $1
          ORDER BY sort_order, created_at",
        dossier.as_uuid()
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| FocalPoint {
            id: Some(l.id),
            speaker_id: l.speaker_id,
            person_id: l.person_id,
            full_name: l.full_name,
            phone: l.phone,
            whatsapp: l.whatsapp,
            is_default: false,
        })
        .collect())
}

/// Le déposant, proposé par défaut tant que rien n'est enregistré.
pub async fn deposant<'e>(
    executor: impl PgExecutor<'e>,
    dossier: ProposalId,
) -> Result<Option<FocalPoint>> {
    let ligne = sqlx::query!(
        r#"SELECT pe.id, pe.display_name AS "display_name!", pe.phone,
                  (SELECT sp.id FROM programme.proposal_speakers sp
                    WHERE sp.proposal_id = p.id AND sp.person_id = pe.id
                    ORDER BY sp.sort_order, sp.created_at LIMIT 1) AS speaker_id
             FROM programme.proposals p
             JOIN identity.people pe ON pe.id = p.submitted_by
            WHERE p.id = $1"#,
        dossier.as_uuid()
    )
    .fetch_optional(executor)
    .await?;

    Ok(ligne.map(|l| FocalPoint {
        id: None,
        speaker_id: l.speaker_id,
        person_id: Some(l.id),
        full_name: l.display_name,
        phone: l.phone.unwrap_or_default(),
        whatsapp: String::new(),
        is_default: true,
    }))
}

/// Un intervenant du dossier : sa personne et le nom à afficher.
pub struct IntervenantDuDossier {
    pub person_id: Option<Uuid>,
    pub nom: String,
    pub phone: Option<String>,
}

pub async fn intervenant<'e>(
    executor: impl PgExecutor<'e>,
    dossier: ProposalId,
    speaker_id: Uuid,
) -> Result<Option<IntervenantDuDossier>> {
    let ligne = sqlx::query!(
        r#"SELECT sp.person_id,
                  COALESCE(pe.display_name, btrim(sp.first_name || ' ' || sp.last_name)) AS "nom!",
                  pe.phone
             FROM programme.proposal_speakers sp
             LEFT JOIN identity.people pe ON pe.id = sp.person_id
            WHERE sp.id = $1 AND sp.proposal_id = $2"#,
        speaker_id,
        dossier.as_uuid()
    )
    .fetch_optional(executor)
    .await?;

    Ok(ligne.map(|l| IntervenantDuDossier {
        person_id: l.person_id,
        nom: l.nom,
        phone: l.phone,
    }))
}

/// Une personne que le dossier connaît : son déposant, son contact, ou un
/// membre actif de l'organisation porteuse. Rien pour une autre.
pub async fn personne_connue<'e>(
    executor: impl PgExecutor<'e>,
    dossier: ProposalId,
    person_id: Uuid,
) -> Result<Option<(String, Option<String>)>> {
    let ligne = sqlx::query!(
        r#"SELECT pe.display_name AS "display_name!", pe.phone
             FROM programme.proposals p
             JOIN identity.people pe ON pe.id = $2
            WHERE p.id = $1
              AND (pe.id = p.submitted_by
                   OR pe.id = p.contact_person_id
                   OR EXISTS (SELECT 1 FROM org.memberships m
                               WHERE m.organization_id = p.organization_id
                                 AND m.person_id = pe.id AND m.status = 'active'))"#,
        dossier.as_uuid(),
        person_id
    )
    .fetch_optional(executor)
    .await?;

    Ok(ligne.map(|l| (l.display_name, l.phone)))
}

/// Un point focal prêt à écrire.
pub struct NouveauPointFocal {
    pub speaker_id: Option<Uuid>,
    pub person_id: Option<Uuid>,
    pub full_name: String,
    pub phone: String,
    pub whatsapp: String,
}

/// Remplacer la liste entière.
pub async fn remplacer_points_focaux(
    conn: &mut PgConnection,
    dossier: ProposalId,
    points: &[NouveauPointFocal],
) -> Result<()> {
    sqlx::query!(
        "DELETE FROM programme.proposal_focal_points WHERE proposal_id = $1",
        dossier.as_uuid()
    )
    .execute(&mut *conn)
    .await?;

    for (rang, point) in points.iter().enumerate() {
        sqlx::query!(
            "INSERT INTO programme.proposal_focal_points
                 (proposal_id, speaker_id, person_id, full_name, phone, whatsapp, sort_order)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
            dossier.as_uuid(),
            point.speaker_id,
            point.person_id,
            point.full_name,
            point.phone,
            point.whatsapp,
            i16::try_from(rang).unwrap_or(i16::MAX)
        )
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}

/// Les besoins déclarés, ou rien.
pub async fn besoins<'e>(
    executor: impl PgExecutor<'e>,
    dossier: ProposalId,
) -> Result<Option<TechnicalNeeds>> {
    let ligne = sqlx::query!(
        "SELECT video, remote, remote_details, accessibility, accessibility_details,
                other, updated_at
           FROM programme.proposal_technical_needs
          WHERE proposal_id = $1",
        dossier.as_uuid()
    )
    .fetch_optional(executor)
    .await?;

    Ok(ligne.map(|l| TechnicalNeeds {
        video: l.video,
        remote: l.remote,
        remote_details: l.remote_details,
        accessibility: l.accessibility,
        accessibility_details: l.accessibility_details,
        other: l.other,
        updated_at: Some(l.updated_at),
    }))
}

/// Déclarer les besoins : la ligne naît au premier enregistrement.
pub async fn ecrire_besoins(
    conn: &mut PgConnection,
    dossier: ProposalId,
    par: Uuid,
    besoins: &TechnicalNeeds,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO programme.proposal_technical_needs
             (proposal_id, video, remote, remote_details, accessibility,
              accessibility_details, other, updated_by)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         ON CONFLICT (proposal_id) DO UPDATE
            SET video = EXCLUDED.video,
                remote = EXCLUDED.remote,
                remote_details = EXCLUDED.remote_details,
                accessibility = EXCLUDED.accessibility,
                accessibility_details = EXCLUDED.accessibility_details,
                other = EXCLUDED.other,
                updated_by = EXCLUDED.updated_by",
        dossier.as_uuid(),
        besoins.video,
        besoins.remote,
        besoins.remote_details,
        besoins.accessibility,
        besoins.accessibility_details,
        besoins.other,
        par
    )
    .execute(conn)
    .await?;
    Ok(())
}

/// Les publications, de la plus ancienne à la plus récente.
pub async fn publications<'e>(
    executor: impl PgExecutor<'e>,
    dossier: ProposalId,
) -> Result<Vec<(Uuid, String, OffsetDateTime)>> {
    let lignes = sqlx::query!(
        r#"SELECT id, url::text AS "url!", created_at
             FROM programme.proposal_social_posts
            WHERE proposal_id = $1
            ORDER BY created_at, id"#,
        dossier.as_uuid()
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| (l.id, l.url, l.created_at))
        .collect())
}

pub async fn nombre_de_publications<'e>(
    executor: impl PgExecutor<'e>,
    dossier: ProposalId,
) -> Result<i64> {
    let n = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM programme.proposal_social_posts WHERE proposal_id = $1"#,
        dossier.as_uuid()
    )
    .fetch_one(executor)
    .await?;
    Ok(n)
}

/// Ajouter un lien. Faux : il y était déjà.
pub async fn ajouter_publication(
    conn: &mut PgConnection,
    dossier: ProposalId,
    par: Uuid,
    url: &str,
) -> Result<bool> {
    let ajoutees = sqlx::query!(
        "INSERT INTO programme.proposal_social_posts (proposal_id, url, created_by)
         VALUES ($1, $2::text::platform.url, $3)
         ON CONFLICT (proposal_id, url) DO NOTHING",
        dossier.as_uuid(),
        url,
        par
    )
    .execute(conn)
    .await?
    .rows_affected();
    Ok(ajoutees == 1)
}

/// Retirer un lien. Faux : il n'est pas à ce dossier.
pub async fn retirer_publication(
    conn: &mut PgConnection,
    dossier: ProposalId,
    publication: Uuid,
) -> Result<bool> {
    let retirees = sqlx::query!(
        "DELETE FROM programme.proposal_social_posts WHERE id = $1 AND proposal_id = $2",
        publication,
        dossier.as_uuid()
    )
    .execute(conn)
    .await?
    .rows_affected();
    Ok(retirees == 1)
}

pub async fn date_limite_des_besoins<'e>(
    executor: impl PgExecutor<'e>,
    dossier: ProposalId,
) -> Result<Option<OffsetDateTime>> {
    let date = sqlx::query_scalar!(
        r#"SELECT programme.proposal_needs_deadline($1) AS "date""#,
        dossier.as_uuid()
    )
    .fetch_one(executor)
    .await?;
    Ok(date)
}
