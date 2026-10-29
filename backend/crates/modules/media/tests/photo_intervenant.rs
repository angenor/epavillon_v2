//! **La photo d'un intervenant se pose sur sa ligne du dossier, par
//! l'organisation du dossier** (29/10), qu'il ait un compte ou non. L'avatar
//! de la personne reste à elle seule (`gardes.rs`).

mod commun;

use commun::Bac;
use kernel::ErrorCode;
use media::domain::attachment::AttachmentPayload;
use media::service::attach;
use uuid::Uuid;

/// Un dossier de l'organisation du terrain, et deux lignes : une personne, un
/// intervenant sans adresse.
async fn lignes_du_dossier(bac: &Bac, terrain: &commun::Terrain) -> (Uuid, Uuid) {
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
                   'hybrid', $4, 'submitted', now())
        RETURNING id"#,
        terrain.edition,
        appel,
        terrain.organisation,
        terrain.referente
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion du dossier");

    // Une personne de la plateforme : son avatar lui appartient, la photo de sa
    // ligne appartient au dossier.
    let avec_personne = sqlx::query_scalar!(
        "INSERT INTO programme.proposal_speakers (proposal_id, person_id, role)
         VALUES ($1, $2, 'speaker') RETURNING id",
        dossier,
        terrain.etrangere
    )
    .fetch_one(bac.pool())
    .await
    .expect("ligne avec personne");
    let sans_personne = sqlx::query_scalar!(
        "INSERT INTO programme.proposal_speakers (proposal_id, role, civility, first_name, last_name)
         VALUES ($1, 'panelist', 'dr', 'Fatou', 'Diop') RETURNING id",
        dossier
    )
    .fetch_one(bac.pool())
    .await
    .expect("ligne sans personne");

    (avec_personne, sans_personne)
}

async fn objet_servable(bac: &Bac, acteur: Uuid) -> Uuid {
    let fichier = commun::vignette_1_1();
    let depose = commun::deposer(bac, acteur, &fichier, commun::metadonnees(&fichier))
        .await
        .expect("dépôt");
    commun::passer_le_worker(bac).await;
    depose.asset.id
}

fn sur_la_ligne(ligne: Uuid, asset_id: Uuid) -> AttachmentPayload {
    AttachmentPayload {
        owner_schema: "programme".to_owned(),
        owner_table: "proposal_speakers".to_owned(),
        owner_id: ligne,
        role: "avatar".to_owned(),
        asset_id,
        sort_order: None,
        alt_text_override: None,
        caption_override: None,
    }
}

#[tokio::test]
async fn un_membre_actif_pose_la_photo_de_tout_intervenant_du_dossier() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let (avec_personne, sans_personne) = lignes_du_dossier(&bac, &terrain).await;

    for ligne in [avec_personne, sans_personne] {
        let photo = objet_servable(&bac, terrain.membre).await;
        attach::poser(
            &bac.state,
            &bac.ctx(),
            terrain.membre,
            &sur_la_ligne(ligne, photo),
        )
        .await
        .expect("le membre actif pose la photo sur la ligne");
    }

    // Le dépôt qui vise la ligne passe la même garde.
    let fichier = commun::vignette_1_1();
    commun::deposer(
        &bac,
        terrain.membre,
        &fichier,
        commun::metadonnees_pour(
            &fichier,
            "programme",
            "proposal_speakers",
            sans_personne,
            "avatar",
        ),
    )
    .await
    .expect("dépôt visant la ligne");
}

#[tokio::test]
async fn une_etrangere_au_dossier_ne_pose_rien_et_une_ligne_forgee_ne_mene_nulle_part() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let (avec_personne, _) = lignes_du_dossier(&bac, &terrain).await;

    // L'étrangère est la personne de la ligne : être l'intervenante ne donne
    // aucun droit sur le dossier d'une autre organisation.
    let photo = objet_servable(&bac, terrain.etrangere).await;
    let refus = attach::poser(
        &bac.state,
        &bac.ctx(),
        terrain.etrangere,
        &sur_la_ligne(avec_personne, photo),
    )
    .await
    .expect_err("une non-membre ne pose pas la photo");
    assert_eq!(refus.code, ErrorCode::NotFound);

    let forgee = attach::poser(
        &bac.state,
        &bac.ctx(),
        terrain.membre,
        &sur_la_ligne(Uuid::now_v7(), photo),
    )
    .await
    .expect_err("une ligne inexistante");
    assert_eq!(forgee.code, ErrorCode::NotFound);
    assert_eq!(forgee.message, refus.message);
}
