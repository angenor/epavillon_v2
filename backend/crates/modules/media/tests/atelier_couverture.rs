//! **La photo de départ de l'atelier de couverture** (`cover_source`, 29/10) se
//! pose sur le dossier sous la garde du dossier, à part de l'image fabriquée.

mod commun;

use commun::Bac;
use kernel::ErrorCode;
use media::domain::attachment::AttachmentPayload;
use media::service::attach;
use uuid::Uuid;

async fn dossier(bac: &Bac, terrain: &commun::Terrain) -> Uuid {
    let appel = sqlx::query_scalar!(
        r#"INSERT INTO event.calls_for_proposals
               (event_id, code, title, status, opens_at, closes_at, required_reviews)
           VALUES ($1, 'principal', '{"fr":"Appel à propositions"}'::jsonb,
                   'open', now() - interval '1 day', now() + interval '30 days', 2)
        RETURNING id"#,
        terrain.edition
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion de l'appel");

    sqlx::query_scalar!(
        r#"INSERT INTO programme.proposals
               (event_id, call_id, organization_id, slug, title,
                objectives, detailed_presentation, format, submitted_by)
           VALUES ($1, $2, $3, 'atelier'::platform.slug,
                   '{"fr":"Les mangroves"}'::jsonb, '{"fr":"Des objectifs."}'::jsonb,
                   '{"fr":"<p>Une présentation.</p>"}'::jsonb,
                   'hybrid', $4)
        RETURNING id"#,
        terrain.edition,
        appel,
        terrain.organisation,
        terrain.referente
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion du dossier")
}

async fn objet_servable(bac: &Bac, acteur: Uuid) -> Uuid {
    let fichier = commun::vignette_1_1();
    let depose = commun::deposer(bac, acteur, &fichier, commun::metadonnees(&fichier))
        .await
        .expect("dépôt");
    commun::passer_le_worker(bac).await;
    depose.asset.id
}

fn sur_le_dossier(dossier: Uuid, role: &str, asset_id: Uuid) -> AttachmentPayload {
    AttachmentPayload {
        owner_schema: "programme".to_owned(),
        owner_table: "proposals".to_owned(),
        owner_id: dossier,
        role: role.to_owned(),
        asset_id,
        sort_order: None,
        alt_text_override: None,
        caption_override: None,
    }
}

#[tokio::test]
async fn un_membre_pose_la_photo_de_depart_a_part_de_la_couverture() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let dossier = dossier(&bac, &terrain).await;

    let source = objet_servable(&bac, terrain.membre).await;
    let rendu = objet_servable(&bac, terrain.membre).await;
    attach::poser(
        &bac.state,
        &bac.ctx(),
        terrain.membre,
        &sur_le_dossier(dossier, "cover_source", source),
    )
    .await
    .expect("le membre actif pose la photo de départ");
    attach::poser(
        &bac.state,
        &bac.ctx(),
        terrain.membre,
        &sur_le_dossier(dossier, "cover", rendu),
    )
    .await
    .expect("puis l'image fabriquée");

    let (lue_source, lue_couverture): (Option<Uuid>, Option<Uuid>) = sqlx::query_as(
        "SELECT (media.attached_image('programme', 'proposals', $1, 'cover_source') ->> 'asset_id')::uuid,
                (media.attached_image('programme', 'proposals', $1, 'cover') ->> 'asset_id')::uuid",
    )
    .bind(dossier)
    .fetch_one(bac.pool())
    .await
    .expect("lecture des deux rôles");
    assert_eq!(lue_source, Some(source), "la source reste la sienne");
    assert_eq!(lue_couverture, Some(rendu));

    let photo = objet_servable(&bac, terrain.etrangere).await;
    let refus = attach::poser(
        &bac.state,
        &bac.ctx(),
        terrain.etrangere,
        &sur_le_dossier(dossier, "cover_source", photo),
    )
    .await
    .expect_err("une non-membre ne pose pas la photo de départ");
    assert_eq!(refus.code, ErrorCode::NotFound);
}
