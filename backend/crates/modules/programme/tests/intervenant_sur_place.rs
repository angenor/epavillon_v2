//! **Corriger un intervenant garde sa ligne, et sa photo avec elle.**
//!
//! La relecture rend l'identifiant de chaque ligne (`speaker_id`) ; renvoyé,
//! il fait corriger la ligne sur place. Sans lui, un nom corrigé ou un rôle
//! changé créait une ligne nouvelle, et la photo restait sur l'ancienne.

mod commun;

use commun::{Bac, Terrain};
use kernel::error::ErrorCode;
use programme::domain::draft::{DraftSpeaker, ProposalDraft, SaveDraftPayload};
use programme::domain::ids::ProposalId;
use programme::service::{draft_read, draft_write};
use uuid::Uuid;

fn sans_adresse(prenom: &str, nom: &str) -> DraftSpeaker {
    let mut intervenant = commun::intervenant("", prenom, nom);
    intervenant.civility = Some("mme".to_owned());
    intervenant
}

fn brouillon(terrain: &Terrain, intervenants: Vec<DraftSpeaker>) -> ProposalDraft {
    let mut brouillon = commun::brouillon(terrain, "Atelier mangroves");
    brouillon.speakers = intervenants;
    brouillon
}

async fn enregistrer(
    bac: &Bac,
    terrain: &Terrain,
    dossier: Option<Uuid>,
    intervenants: Vec<DraftSpeaker>,
) -> kernel::error::Result<Uuid> {
    let charge = SaveDraftPayload {
        proposal_id: dossier,
        ..commun::charge(terrain, brouillon(terrain, intervenants))
    };
    draft_write::enregistrer(&bac.state, &bac.ctx(), terrain.deposante, charge)
        .await
        .map(|l| l.proposal_id)
}

async fn lignes(bac: &Bac, dossier: Uuid) -> Vec<(Uuid, String, Option<String>)> {
    sqlx::query!(
        r#"SELECT id, role::text AS "role!", first_name
             FROM programme.proposal_speakers WHERE proposal_id = $1
            ORDER BY sort_order"#,
        dossier
    )
    .fetch_all(bac.pool())
    .await
    .expect("lecture des lignes")
    .into_iter()
    .map(|l| (l.id, l.role, l.first_name))
    .collect()
}

/// Une photo servable, rattachée à `(schéma, table, id)` pour le rôle `avatar`.
async fn photo(bac: &Bac, terrain: &Terrain, schema: &str, table: &str, id: Uuid) -> Uuid {
    let asset = sqlx::query_scalar!(
        r#"INSERT INTO media.assets
               (object_key, original_filename, mime_type, byte_size, checksum_sha256,
                status, alt_text, owner_organization_id, scan_verdict, scanned_at)
           VALUES ('intervenants/' || $2::text || '.jpg', 'photo.jpg', 'image/jpeg', 40000,
                   md5($2::text) || md5($2::text), 'ready', '{"fr":"Portrait"}'::jsonb, $1, 'clean', now())
        RETURNING id"#,
        terrain.organisation,
        Uuid::now_v7().to_string()
    )
    .fetch_one(bac.pool())
    .await
    .expect("dépôt de la photo");
    sqlx::query!(
        "INSERT INTO media.attachments (asset_id, owner_schema, owner_table, owner_id, role)
         VALUES ($1, $2, $3, $4, 'avatar')",
        asset,
        schema,
        table,
        id
    )
    .execute(bac.pool())
    .await
    .expect("rattachement de la photo");
    asset
}

fn asset_de(photo: &Option<serde_json::Value>) -> Option<String> {
    photo
        .as_ref()
        .and_then(|p| p["asset_id"].as_str())
        .map(str::to_owned)
}

#[tokio::test]
async fn corriger_le_nom_dun_intervenant_sans_adresse_garde_sa_ligne_et_sa_photo() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    let dossier = enregistrer(&bac, &terrain, None, vec![sans_adresse("Fatou", "Dip")])
        .await
        .expect("brouillon");
    let rouvert = draft_read::rouvrir(&bac.state, terrain.deposante, ProposalId(dossier))
        .await
        .expect("relecture");
    let ligne = rouvert.draft.speakers[0].speaker_id;
    assert!(rouvert.draft.speakers[0].photo_image.is_none());

    let asset = photo(&bac, &terrain, "programme", "proposal_speakers", ligne).await;

    let mut corrige = sans_adresse("Fatou", "Diop");
    corrige.speaker_id = Some(ligne);
    corrige.role = "moderator".to_owned();
    enregistrer(&bac, &terrain, Some(dossier), vec![corrige])
        .await
        .expect("correction");

    assert_eq!(
        lignes(&bac, dossier).await,
        vec![(ligne, "moderator".to_owned(), Some("Fatou".to_owned()))],
        "la même ligne, corrigée"
    );
    let rouvert = draft_read::rouvrir(&bac.state, terrain.deposante, ProposalId(dossier))
        .await
        .expect("relecture");
    let relu = &rouvert.draft.speakers[0];
    assert_eq!(relu.speaker_id, ligne);
    assert_eq!(relu.last_name, "Diop");
    assert_eq!(asset_de(&relu.photo_image), Some(asset.to_string()));
}

#[tokio::test]
async fn changer_le_role_dune_personne_garde_sa_ligne() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let awa = commun::intervenant("awa.sow@example.org", "Awa", "Sow");
    let dossier = enregistrer(&bac, &terrain, None, vec![awa.clone()])
        .await
        .expect("brouillon");
    let (ligne, _, _) = lignes(&bac, dossier).await.remove(0);

    let mut moderatrice = awa;
    moderatrice.speaker_id = Some(ligne);
    moderatrice.role = "moderator".to_owned();
    enregistrer(&bac, &terrain, Some(dossier), vec![moderatrice])
        .await
        .expect("changement de rôle");

    assert_eq!(
        lignes(&bac, dossier).await,
        vec![(ligne, "moderator".to_owned(), None)]
    );
}

#[tokio::test]
async fn une_ligne_absente_est_retiree_et_une_ligne_etrangere_ne_designe_rien() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    let autre = enregistrer(&bac, &terrain, None, vec![sans_adresse("Ina", "Autre")])
        .await
        .expect("un autre dossier");
    let (etrangere, _, _) = lignes(&bac, autre).await.remove(0);

    let dossier = enregistrer(
        &bac,
        &terrain,
        None,
        vec![sans_adresse("Fatou", "Diop"), sans_adresse("Awa", "Sow")],
    )
    .await
    .expect("brouillon");
    let avant = lignes(&bac, dossier).await;

    // Fatou est retirée ; la ligne d'un autre dossier n'est pas reprise.
    let mut forgee = sans_adresse("Awa", "Sow");
    forgee.speaker_id = Some(etrangere);
    enregistrer(&bac, &terrain, Some(dossier), vec![forgee])
        .await
        .expect("enregistrement");

    let apres = lignes(&bac, dossier).await;
    assert_eq!(apres.len(), 1);
    assert_eq!(apres[0].0, avant[1].0, "Awa retrouvée par son nom");
    assert_eq!(
        lignes(&bac, autre).await,
        vec![(etrangere, "speaker".to_owned(), Some("Ina".to_owned()))],
        "l'autre dossier est intact"
    );
}

#[tokio::test]
async fn la_meme_personne_deux_fois_avec_le_meme_role_est_refusee() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let awa = commun::intervenant("awa.sow@example.org", "Awa", "Sow");

    let refus = enregistrer(&bac, &terrain, None, vec![awa.clone(), awa])
        .await
        .expect_err("doublon");
    assert_eq!(refus.code, ErrorCode::ValidationFailed);
    assert_eq!(refus.field.as_deref(), Some("speakers"));
}

#[tokio::test]
async fn la_photo_de_la_ligne_passe_avant_lavatar_de_la_personne() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let dossier = enregistrer(
        &bac,
        &terrain,
        None,
        vec![commun::intervenant("awa.sow@example.org", "Awa", "Sow")],
    )
    .await
    .expect("brouillon");
    let (ligne, _, _) = lignes(&bac, dossier).await.remove(0);
    let awa = commun::fiche(&bac, "awa.sow@example.org")
        .await
        .expect("la personne")
        .0;

    let profil = photo(&bac, &terrain, "identity", "people", awa).await;
    let relu = draft_read::rouvrir(&bac.state, terrain.deposante, ProposalId(dossier))
        .await
        .expect("relecture");
    assert_eq!(
        asset_de(&relu.draft.speakers[0].photo_image),
        Some(profil.to_string()),
        "sans photo sur la ligne, l'avatar de la personne"
    );

    let choisie = photo(&bac, &terrain, "programme", "proposal_speakers", ligne).await;
    let relu = draft_read::rouvrir(&bac.state, terrain.deposante, ProposalId(dossier))
        .await
        .expect("relecture");
    assert_eq!(
        asset_de(&relu.draft.speakers[0].photo_image),
        Some(choisie.to_string()),
        "la photo choisie par l'organisation l'emporte"
    );
}
