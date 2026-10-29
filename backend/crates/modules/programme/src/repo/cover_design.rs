//! L'atelier de l'image de couverture d'un dossier (070 § 4 ter) : ses choix,
//! et les images qu'il compose.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use sqlx::PgExecutor;
use uuid::Uuid;

use crate::domain::cover_design::{CoverDesign, CoverDesignOptions};
use crate::domain::ids::ProposalId;

pub async fn choix<'e>(
    executor: impl PgExecutor<'e>,
    dossier: ProposalId,
) -> Result<Option<CoverDesign>> {
    let ligne = sqlx::query!(
        "SELECT template_code, accent, options, updated_at
           FROM programme.proposal_cover_designs
          WHERE proposal_id = $1",
        dossier.as_uuid()
    )
    .fetch_optional(executor)
    .await?;

    Ok(ligne.map(|l| CoverDesign {
        template_code: l.template_code,
        accent: l.accent,
        options: CoverDesignOptions::depuis_json(&l.options),
        updated_at: l.updated_at,
    }))
}

/// La ligne naît au premier enregistrement.
pub async fn ecrire(
    conn: &mut PgConnection,
    dossier: ProposalId,
    par: Uuid,
    template_code: &str,
    accent: &str,
    options: CoverDesignOptions,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO programme.proposal_cover_designs
             (proposal_id, template_code, accent, options, updated_by)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (proposal_id) DO UPDATE
            SET template_code = EXCLUDED.template_code,
                accent = EXCLUDED.accent,
                options = EXCLUDED.options,
                updated_by = EXCLUDED.updated_by",
        dossier.as_uuid(),
        template_code,
        accent,
        options.en_json(),
        par
    )
    .execute(conn)
    .await?;
    Ok(())
}

/// Les images que l'atelier compose, `AttachedImage` ou nulles.
pub struct ImagesDeLatelier {
    pub source_photo: Option<serde_json::Value>,
    pub edition_cover: Option<serde_json::Value>,
    pub edition_banner: Option<serde_json::Value>,
    pub edition_thumbnail: Option<serde_json::Value>,
    pub edition_logo: Option<serde_json::Value>,
    pub organization_logo: Option<serde_json::Value>,
}

pub async fn images<'e>(
    executor: impl PgExecutor<'e>,
    dossier: ProposalId,
) -> Result<ImagesDeLatelier> {
    let ligne = sqlx::query!(
        r#"SELECT media.attached_image('programme', 'proposals', p.id, 'cover_source') AS "source_photo",
                  media.attached_image('event', 'events', p.event_id, 'cover') AS "edition_cover",
                  media.attached_image('event', 'events', p.event_id, 'banner') AS "edition_banner",
                  media.attached_image('event', 'events', p.event_id, 'thumbnail') AS "edition_thumbnail",
                  media.attached_image('event', 'events', p.event_id, 'logo') AS "edition_logo",
                  media.attached_image('org', 'organizations', p.organization_id, 'logo') AS "organization_logo"
             FROM programme.proposals p
            WHERE p.id = $1"#,
        dossier.as_uuid()
    )
    .fetch_one(executor)
    .await?;

    Ok(ImagesDeLatelier {
        source_photo: ligne.source_photo,
        edition_cover: ligne.edition_cover,
        edition_banner: ligne.edition_banner,
        edition_thumbnail: ligne.edition_thumbnail,
        edition_logo: ligne.edition_logo,
        organization_logo: ligne.organization_logo,
    })
}
