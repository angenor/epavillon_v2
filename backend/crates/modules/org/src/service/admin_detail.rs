//! La fiche du back-office : huit lectures assemblées, et le refus par
//! périmètre.
//!
//! **Une fiche hors périmètre rend un refus indiscernable d'une fiche
//! inexistante**, URL forgée comprise. Distinguer les deux dirait à qui forge
//! une adresse qu'une organisation existe et qu'il n'y a pas droit — deux
//! informations, et la première n'a pas à sortir.
//!
//! **Une fiche absorbée s'ouvre normalement**, coiffée de son renvoi : c'est la
//! promesse de `org.resolve_organization()`, et les adresses déjà diffusées
//! continuent de mener quelque part.

use kernel::auth::AdminScope;
use kernel::error::Result;
use sqlx::PgPool;

use crate::domain::admin::OrganizationDetail;
use crate::domain::ids::OrganizationId;
use crate::repo::{admin_detail, duplicates};

/// La fiche entière, ou `None` — que l'appelant rend en 200 avec `null`, comme
/// le contrat l'annonce.
pub async fn detail(
    pool: &PgPool,
    perimetre: &AdminScope,
    id: OrganizationId,
) -> Result<Option<OrganizationDetail>> {
    // **Le périmètre d'abord.** Lire la fiche puis décider laisserait fuir son
    // existence par le temps de réponse, et surtout par la tentation de
    // distinguer les deux refus.
    if !dans_le_perimetre(pool, perimetre, id).await? {
        return Ok(None);
    }

    // Les huit lectures dans **une seule transaction de lecture** : la fiche est
    // cohérente avec elle-même, et l'assemblage se fait en Rust.
    let mut conn = pool.acquire().await?;
    sqlx::query("BEGIN ISOLATION LEVEL REPEATABLE READ, READ ONLY")
        .execute(&mut *conn)
        .await?;

    let assemblee = assembler(&mut conn, id).await;

    sqlx::query("COMMIT").execute(&mut *conn).await?;
    assemblee
}

async fn assembler(
    conn: &mut sqlx::postgres::PgConnection,
    id: OrganizationId,
) -> Result<Option<OrganizationDetail>> {
    let Some(mut fiche) = admin_detail::identite(conn, id).await? else {
        return Ok(None);
    };

    fiche.scorecard = admin_detail::scorecard(conn, id).await?;
    fiche.names = admin_detail::denominations(conn, id).await?;
    fiche.domains = admin_detail::domaines(conn, id).await?;
    fiche.members = admin_detail::membres(conn, id).await?;
    fiche.activities = admin_detail::activites(conn, id).await?;
    fiche.history = admin_detail::historique(conn, id).await?;
    fiche.merges = admin_detail::fusions(conn, id).await?;
    fiche.absorbed = admin_detail::absorbees(conn, id).await?;
    fiche.duplicates = duplicates::ouvertes_pour(conn, id).await?;

    Ok(Some(fiche))
}

/// La fiche entre-t-elle dans le périmètre de l'appelant ?
///
/// Une organisation n'appartient à aucune édition : c'est **l'activité déposée
/// ou tenue** qui la rattache à un périmètre — la même condition que la liste,
/// et il n'y en a qu'une.
pub async fn dans_le_perimetre(
    pool: &PgPool,
    perimetre: &AdminScope,
    id: OrganizationId,
) -> Result<bool> {
    if perimetre.is_global {
        return Ok(true);
    }

    let visible = sqlx::query_scalar!(
        r#"SELECT EXISTS (
               SELECT 1 FROM programme.proposals p
                WHERE p.organization_id = $1 AND p.deleted_at IS NULL
                  AND p.event_id = ANY($2::uuid[])
               UNION ALL
               SELECT 1 FROM programme.proposal_organizations po
                JOIN programme.proposals p ON p.id = po.proposal_id
                WHERE po.organization_id = $1 AND p.deleted_at IS NULL
                  AND p.event_id = ANY($2::uuid[])
               UNION ALL
               SELECT 1 FROM programme.sessions s
                WHERE s.organization_id = $1 AND s.event_id = ANY($2::uuid[])
           ) AS "visible!""#,
        id.as_uuid(),
        &perimetre.event_ids
    )
    .fetch_one(pool)
    .await?;

    Ok(visible)
}
