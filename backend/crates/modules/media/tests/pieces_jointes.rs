//! **Les pièces jointes d'un envoi de courriels (28/10).**
//!
//! Gardées comme l'envoi : `engagement.mailing.send` sur l'édition de l'envoi,
//! avec le périmètre. Toujours privées. Et un poids cumulé par envoi, refusé
//! avant le premier octet comme au rattachement.

mod commun;

use commun::Bac;
use kernel::ErrorCode;
use media::domain::attachment::AttachmentPayload;
use media::service::{attach, upload};
use uuid::Uuid;

async fn envoi(bac: &Bac, edition: Uuid, auteur: Uuid) -> Uuid {
    sqlx::query_scalar!(
        "INSERT INTO engagement.mailings (event_id, created_by) VALUES ($1, $2) RETURNING id",
        edition,
        auteur
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion de l'envoi")
}

fn piece(envoi: Uuid, asset_id: Uuid) -> AttachmentPayload {
    AttachmentPayload {
        owner_schema: "engagement".to_owned(),
        owner_table: "mailings".to_owned(),
        owner_id: envoi,
        role: "attachment".to_owned(),
        asset_id,
        sort_order: None,
        alt_text_override: None,
        caption_override: None,
    }
}

fn declaration(fichier: &commun::Fichier, envoi: Uuid) -> upload::UploadDeclaration {
    upload::UploadDeclaration {
        filename: fichier.nom.to_owned(),
        mime_type: fichier.mime.to_owned(),
        byte_size: fichier.octets.len() as i64,
        owner_schema: Some("engagement".to_owned()),
        owner_table: Some("mailings".to_owned()),
        owner_id: Some(envoi),
        role: Some("attachment".to_owned()),
        checksum_sha256: None,
    }
}

#[tokio::test]
async fn ladministratrice_de_ledition_joint_un_fichier_prive() {
    let bac = Bac::monter().await;
    let t = commun::terrain(&bac).await;
    let id = envoi(&bac, t.edition, t.administratrice).await;
    let pdf = commun::document_pdf();

    let depose = commun::deposer(
        &bac,
        t.administratrice,
        &pdf,
        commun::metadonnees_pour(&pdf, "engagement", "mailings", id, "attachment"),
    )
    .await
    .expect("dépôt");
    // Privé, même sans le demander.
    assert_eq!(depose.asset.visibility, "private");
    commun::passer_le_worker(&bac).await;

    attach::poser(
        &bac.state,
        &bac.ctx(),
        t.administratrice,
        &piece(id, depose.asset.id),
    )
    .await
    .expect("rattachement");

    // Ni une personne sans rôle, ni la référente d'une organisation.
    for intrus in [t.etrangere, t.referente] {
        let refus = commun::deposer(
            &bac,
            intrus,
            &pdf,
            commun::metadonnees_pour(&pdf, "engagement", "mailings", id, "attachment"),
        )
        .await
        .expect_err("refus");
        assert_eq!(refus.code, ErrorCode::NotFound);
    }
}

#[tokio::test]
async fn ladministratrice_dune_autre_edition_ne_voit_pas_lenvoi() {
    let bac = Bac::monter().await;
    let t = commun::terrain(&bac).await;
    let id = envoi(&bac, t.edition, t.administratrice).await;
    let voisine = commun::personne(&bac, "voisine@ifdd.org", "Inès", "Diop").await;
    commun::attribuer(&bac, voisine, "admin", "event", Some(Uuid::now_v7())).await;
    let pdf = commun::document_pdf();

    let refus = commun::deposer(
        &bac,
        voisine,
        &pdf,
        commun::metadonnees_pour(&pdf, "engagement", "mailings", id, "attachment"),
    )
    .await
    .expect_err("refus");
    assert_eq!(refus.code, ErrorCode::NotFound);
}

/// Au-delà du poids cumulé de l'envoi, le refus tombe à l'annonce, au dépôt et
/// au rattachement, avec le même code.
#[tokio::test]
async fn au_dela_du_poids_cumule_le_depot_est_refuse() {
    let bac = Bac::monter().await;
    let t = commun::terrain(&bac).await;
    let id = envoi(&bac, t.edition, t.administratrice).await;
    let pdf = commun::document_pdf();
    let poids = pdf.octets.len() as i64;
    // Place pour un fichier et demi : le second ne passe plus.
    sqlx::query!(
        "UPDATE media.attachable_roles SET max_total_byte_size = $1
          WHERE owner_schema = 'engagement' AND owner_table = 'mailings' AND role = 'attachment'",
        poids + poids / 2
    )
    .execute(bac.pool())
    .await
    .expect("plafond");

    let premier = commun::deposer(
        &bac,
        t.administratrice,
        &pdf,
        commun::metadonnees_pour(&pdf, "engagement", "mailings", id, "attachment"),
    )
    .await
    .expect("dépôt");
    commun::passer_le_worker(&bac).await;
    attach::poser(
        &bac.state,
        &bac.ctx(),
        t.administratrice,
        &piece(id, premier.asset.id),
    )
    .await
    .expect("rattachement");

    let annonce = upload::annoncer(&bac.state, t.administratrice, &declaration(&pdf, id))
        .await
        .expect("annonce");
    assert!(!annonce.accepted);
    assert_eq!(
        annonce.code.as_deref(),
        Some(ErrorCode::MediaTotalTooLarge.as_str())
    );

    let autre = commun::Fichier {
        nom: "annexe.pdf",
        mime: "application/pdf",
        octets: [pdf.octets.clone(), b"% annexe\n".to_vec()].concat(),
    };
    let refus = commun::deposer(
        &bac,
        t.administratrice,
        &autre,
        commun::metadonnees_pour(&autre, "engagement", "mailings", id, "attachment"),
    )
    .await
    .expect_err("refus");
    assert_eq!(refus.code, ErrorCode::MediaTotalTooLarge);

    // Déposé sans entité, puis rattaché : refusé au rattachement.
    let libre = commun::deposer(&bac, t.administratrice, &autre, commun::metadonnees(&autre))
        .await
        .expect("dépôt libre");
    commun::passer_le_worker(&bac).await;
    let refus = attach::poser(
        &bac.state,
        &bac.ctx(),
        t.administratrice,
        &piece(id, libre.asset.id),
    )
    .await
    .expect_err("refus");
    assert_eq!(refus.code, ErrorCode::MediaTotalTooLarge);
}
