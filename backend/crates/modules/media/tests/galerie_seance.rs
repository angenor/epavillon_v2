//! **La galerie d'une activité (D7) est posée par l'équipe, pas par
//! l'organisation qui l'anime.**
//!
//! La couverture d'une séance s'ouvre à l'adhésion ; sa galerie, non. Ces tests
//! éprouvent la garde de rôle sur les trois gestes (poser, remplacer, retirer),
//! la légende propre à l'usage, et la lecture publique, qui ne sert que le
//! publié et le prêt.

mod commun;

use commun::Bac;
use kernel::ErrorCode;
use media::domain::attachment::{AttachmentAssignment, AttachmentBatch, AttachmentPayload};
use media::service::attach;
use serde_json::json;
use uuid::Uuid;

async fn objet_servable(bac: &Bac, acteur: Uuid, fichier: &commun::Fichier) -> Uuid {
    let depose = commun::deposer(bac, acteur, fichier, commun::metadonnees(fichier))
        .await
        .expect("dépôt");
    commun::passer_le_worker(bac).await;
    depose.asset.id
}

/// Une séance issue d'un dossier retenu de l'organisation du terrain : sa
/// couverture s'ouvre donc à ses membres.
async fn seance_de_lorganisation(bac: &Bac, terrain: &commun::Terrain) -> Uuid {
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

    let dossier = sqlx::query_scalar!(
        r#"INSERT INTO programme.proposals
               (event_id, call_id, organization_id, slug, title,
                objectives, detailed_presentation, format, submitted_by, status, submitted_at)
           VALUES ($1, $2, $3, 'mangroves'::platform.slug,
                   '{"fr":"Les mangroves"}'::jsonb, '{"fr":"Des objectifs."}'::jsonb,
                   '{"fr":"<p>Une présentation.</p>"}'::jsonb,
                   'hybrid', $4, 'accepted', now())
        RETURNING id"#,
        terrain.edition,
        appel,
        terrain.organisation,
        terrain.referente
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion du dossier");

    sqlx::query_scalar!(
        r#"INSERT INTO programme.sessions
               (event_id, proposal_id, title, slug, status, format, starts_at, ends_at, timezone)
           VALUES ($1, $2, '{"fr":"Les mangroves"}'::jsonb, 'mangroves', 'scheduled', 'hybrid',
                   timestamp '2027-11-12 10:00' AT TIME ZONE 'America/Belem',
                   timestamp '2027-11-12 11:30' AT TIME ZONE 'America/Belem',
                   'America/Belem'::platform.timezone_name)
        RETURNING id"#,
        terrain.edition,
        dossier
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion de la séance")
}

fn photo(seance: Uuid, role: &str, asset_id: Uuid) -> AttachmentPayload {
    AttachmentPayload {
        owner_schema: "programme".to_owned(),
        owner_table: "sessions".to_owned(),
        owner_id: seance,
        role: role.to_owned(),
        asset_id,
        sort_order: None,
        alt_text_override: None,
        caption_override: None,
    }
}

async fn galerie_publique(bac: &Bac, seance: Uuid) -> Vec<serde_json::Value> {
    let galerie: serde_json::Value = sqlx::query_scalar!(
        r#"SELECT media.attached_images('programme', 'sessions', $1, 'gallery') AS "galerie!""#,
        seance
    )
    .fetch_one(bac.pool())
    .await
    .expect("lecture de la galerie");
    galerie.as_array().cloned().unwrap_or_default()
}

/// Le membre pose la couverture de son activité, pas une photo de sa galerie ;
/// l'administratrice de l'édition pose la photo.
#[tokio::test]
async fn la_galerie_est_reservee_a_lequipe_de_ledition() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let seance = seance_de_lorganisation(&bac, &terrain).await;

    let couverture = objet_servable(&bac, terrain.membre, &commun::couverture_16_9()).await;
    attach::poser(
        &bac.state,
        &bac.ctx(),
        terrain.membre,
        &photo(seance, "cover", couverture),
    )
    .await
    .expect("le membre pose la couverture de son activité");

    let image = objet_servable(&bac, terrain.membre, &commun::vignette_1_1()).await;
    let refus = attach::poser(
        &bac.state,
        &bac.ctx(),
        terrain.membre,
        &photo(seance, "gallery", image),
    )
    .await
    .expect_err("une organisation n'ajoute pas de photo à la galerie");
    assert_eq!(refus.code, ErrorCode::NotFound);

    let lot = AttachmentBatch {
        owner_schema: "programme".to_owned(),
        owner_table: "sessions".to_owned(),
        owner_id: seance,
        assignments: vec![AttachmentAssignment {
            role: "gallery".to_owned(),
            asset_id: Some(image),
            alt_text_override: None,
            caption_override: None,
        }],
    };
    let refus = attach::remplacer(&bac.state, &bac.ctx(), terrain.membre, &lot)
        .await
        .expect_err("ni en lot");
    assert_eq!(refus.code, ErrorCode::NotFound);

    let pose = attach::poser(
        &bac.state,
        &bac.ctx(),
        terrain.administratrice,
        &AttachmentPayload {
            caption_override: Some(json!({ "fr": "Ouverture du panel" })),
            ..photo(seance, "gallery", image)
        },
    )
    .await
    .expect("l'administratrice de l'édition pose la photo");
    assert_eq!(pose.caption, Some(json!({ "fr": "Ouverture du panel" })));

    let refus = attach::detacher(&bac.state, &bac.ctx(), terrain.membre, pose.attachment_id)
        .await
        .expect_err("ni la retirer");
    assert_eq!(refus.code, ErrorCode::NotFound);

    let galerie = galerie_publique(&bac, seance).await;
    assert_eq!(galerie.len(), 1);
    assert_eq!(galerie[0]["caption"], json!({ "fr": "Ouverture du panel" }));
}

/// Un texte alternatif d'usage vide masquerait celui de l'objet : refusé avant
/// l'écriture, sous son nom.
#[tokio::test]
async fn un_texte_alternatif_dusage_vide_est_refuse() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let seance = seance_de_lorganisation(&bac, &terrain).await;
    let image = objet_servable(&bac, terrain.administratrice, &commun::vignette_1_1()).await;

    let refus = attach::poser(
        &bac.state,
        &bac.ctx(),
        terrain.administratrice,
        &AttachmentPayload {
            alt_text_override: Some(json!({ "fr": "  " })),
            ..photo(seance, "gallery", image)
        },
    )
    .await
    .expect_err("un texte alternatif vide");
    assert_eq!(refus.code, ErrorCode::MediaAltTextRequired);
}

/// La lecture publique suit l'ordre réglé et ne sert que le public et le prêt.
#[tokio::test]
async fn la_galerie_publique_suit_lordre_et_ne_sert_que_le_publie() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let seance = seance_de_lorganisation(&bac, &terrain).await;

    let premiere = objet_servable(&bac, terrain.administratrice, &commun::vignette_1_1()).await;
    let seconde = objet_servable(&bac, terrain.administratrice, &commun::couverture_16_9()).await;
    let reservee = objet_servable(&bac, terrain.administratrice, &commun::bandeau_32_9()).await;
    sqlx::query!(
        "UPDATE media.assets SET visibility = 'authenticated' WHERE id = $1",
        reservee
    )
    .execute(bac.pool())
    .await
    .expect("objet réservé aux comptes");

    let affectation = |asset_id| AttachmentAssignment {
        role: "gallery".to_owned(),
        asset_id: Some(asset_id),
        alt_text_override: Some(json!({ "fr": "Une photo de l'activité" })),
        caption_override: None,
    };
    attach::remplacer(
        &bac.state,
        &bac.ctx(),
        terrain.administratrice,
        &AttachmentBatch {
            owner_schema: "programme".to_owned(),
            owner_table: "sessions".to_owned(),
            owner_id: seance,
            assignments: vec![
                affectation(seconde),
                affectation(reservee),
                affectation(premiere),
            ],
        },
    )
    .await
    .expect("la galerie s'écrit en un lot");

    let galerie = galerie_publique(&bac, seance).await;
    let ordre: Vec<String> = galerie
        .iter()
        .map(|p| p["asset_id"].as_str().unwrap_or_default().to_owned())
        .collect();
    assert_eq!(ordre, vec![seconde.to_string(), premiere.to_string()]);
    assert_eq!(
        galerie[0]["alt_text"],
        json!({ "fr": "Une photo de l'activité" })
    );
}
