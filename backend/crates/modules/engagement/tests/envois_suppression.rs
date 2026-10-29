//! **Supprimer un envoi, compléter une adresse.**
//!
//! Un brouillon, un envoi terminé ou arrêté disparaissent avec leurs
//! destinataires et leurs rattachements ; un envoi en cours se refuse. La
//! saisie d'une adresse propose des personnes connues, sans égard aux accents
//! ni à la casse, à qui peut écrire aux participants de l'édition, et à elle
//! seule.

mod commun;

use commun::Bac;
use engagement::domain::mailing::{CreateMailingPayload, MailingPayload};
use engagement::service::mailings;
use kernel::ErrorCode;
use uuid::Uuid;

fn brouillon(edition: Uuid) -> CreateMailingPayload {
    CreateMailingPayload {
        event_id: edition,
        draft: MailingPayload {
            subject: Some("Programme".into()),
            body_html: Some("<p>Bonjour {{prenom}},</p>".into()),
            extra_addresses: Some(vec!["libre@example.org".into()]),
            ..MailingPayload::default()
        },
    }
}

async fn joindre_un_fichier(bac: &Bac, envoi: Uuid, auteur: Uuid) -> Uuid {
    let asset = sqlx::query_scalar!(
        r#"INSERT INTO media.assets
               (bucket, object_key, checksum_sha256, mime_type, byte_size, original_filename,
                owner_person_id, visibility, status)
           VALUES ('epavillon-prive', $1, repeat('b', 64), 'application/pdf', 10,
                   'programme.pdf', $2, 'private', 'processing')
        RETURNING id"#,
        format!("essais/{}.pdf", Uuid::now_v7()),
        auteur
    )
    .fetch_one(bac.pool())
    .await
    .expect("objet");
    sqlx::query!(
        "INSERT INTO media.attachments (owner_schema, owner_table, owner_id, asset_id, role)
         VALUES ('engagement', 'mailings', $1, $2, 'attachment')",
        envoi,
        asset
    )
    .execute(bac.pool())
    .await
    .expect("rattachement");
    asset
}

async fn rattachements(bac: &Bac, envoi: Uuid) -> i64 {
    sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM media.attachments
            WHERE owner_schema = 'engagement' AND owner_table = 'mailings' AND owner_id = $1"#,
        envoi
    )
    .fetch_one(bac.pool())
    .await
    .expect("compte")
}

async fn destinataires(bac: &Bac, envoi: Uuid) -> i64 {
    sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM engagement.mailing_recipients WHERE mailing_id = $1"#,
        envoi
    )
    .fetch_one(bac.pool())
    .await
    .expect("compte")
}

#[tokio::test]
async fn le_brouillon_disparait_avec_ses_pieces_jointes() {
    let bac = Bac::monter().await;
    let t = commun::terrain(&bac).await;
    let envoi = mailings::creer(
        &bac.state,
        &bac.ctx(),
        t.administratrice,
        &brouillon(t.edition),
    )
    .await
    .expect("brouillon");
    let asset = joindre_un_fichier(&bac, envoi.id, t.administratrice).await;

    mailings::supprimer(&bac.state, &bac.ctx(), t.administratrice, envoi.id)
        .await
        .expect("suppression");

    let refus = mailings::detail(&bac.state, t.administratrice, envoi.id)
        .await
        .expect_err("disparu");
    assert_eq!(refus.code, ErrorCode::NotFound);
    assert_eq!(rattachements(&bac, envoi.id).await, 0);
    // Le fichier reste : il peut servir ailleurs, la purge des orphelins décide.
    let reste = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM media.assets WHERE id = $1"#,
        asset
    )
    .fetch_one(bac.pool())
    .await
    .expect("compte");
    assert_eq!(reste, 1);
}

#[tokio::test]
async fn lenvoi_en_cours_se_refuse_puis_arrete_se_supprime() {
    let bac = Bac::monter().await;
    let t = commun::terrain(&bac).await;
    let envoi = mailings::creer(
        &bac.state,
        &bac.ctx(),
        t.administratrice,
        &brouillon(t.edition),
    )
    .await
    .expect("brouillon");
    mailings::envoyer(&bac.state, &bac.ctx(), t.administratrice, envoi.id)
        .await
        .expect("envoi");

    let refus = mailings::supprimer(&bac.state, &bac.ctx(), t.administratrice, envoi.id)
        .await
        .expect_err("en cours");
    assert_eq!(refus.code, ErrorCode::EngagementMailingInProgress);
    assert!(refus.message.contains("en cours"), "{}", refus.message);
    assert_eq!(destinataires(&bac, envoi.id).await, 1);

    mailings::annuler(&bac.state, &bac.ctx(), t.administratrice, envoi.id)
        .await
        .expect("arrêt");
    mailings::supprimer(&bac.state, &bac.ctx(), t.administratrice, envoi.id)
        .await
        .expect("suppression après l'arrêt");
    assert_eq!(destinataires(&bac, envoi.id).await, 0);
}

#[tokio::test]
async fn lenvoi_termine_quitte_lhistorique() {
    let bac = Bac::monter().await;
    let t = commun::terrain(&bac).await;
    let envoi = mailings::creer(
        &bac.state,
        &bac.ctx(),
        t.administratrice,
        &brouillon(t.edition),
    )
    .await
    .expect("brouillon");
    mailings::envoyer(&bac.state, &bac.ctx(), t.administratrice, envoi.id)
        .await
        .expect("envoi");
    for issue in commun::passer_le_worker(&bac).await {
        issue.expect("passage du worker");
    }
    let fini = mailings::detail(&bac.state, t.administratrice, envoi.id)
        .await
        .expect("détail");
    assert_eq!(fini.status, "sent");

    mailings::supprimer(&bac.state, &bac.ctx(), t.administratrice, envoi.id)
        .await
        .expect("suppression");
    let ecran = mailings::ecran(&bac.state, t.administratrice, t.edition)
        .await
        .expect("historique");
    assert!(ecran.rows.iter().all(|r| r.id != envoi.id));
    assert_eq!(destinataires(&bac, envoi.id).await, 0);
}

#[tokio::test]
async fn hors_perimetre_la_suppression_ne_trouve_rien() {
    let bac = Bac::monter().await;
    let t = commun::terrain(&bac).await;
    let envoi = mailings::creer(
        &bac.state,
        &bac.ctx(),
        t.administratrice,
        &brouillon(t.edition),
    )
    .await
    .expect("brouillon");

    let voisine = commun::personne(&bac, "voisine@ifdd.org", "Inès", "Diop").await;
    commun::attribuer(&bac, voisine, "admin", "event", Some(Uuid::now_v7())).await;
    for intrus in [voisine, t.etrangere] {
        let refus = mailings::supprimer(&bac.state, &bac.ctx(), intrus, envoi.id)
            .await
            .expect_err("hors périmètre");
        assert_eq!(refus.code, ErrorCode::NotFound);
    }
    mailings::detail(&bac.state, t.administratrice, envoi.id)
        .await
        .expect("toujours là");
}

#[tokio::test]
async fn la_saisie_propose_des_personnes_connues() {
    let bac = Bac::monter().await;
    let t = commun::terrain(&bac).await;
    let chercher =
        |q: &'static str| mailings::suggestions(&bac.state, t.administratrice, t.edition, q);

    // Sans accent ni casse : « LEA » trouve Léa.
    let lea = chercher("LEA").await.expect("recherche");
    assert_eq!(lea.len(), 1);
    assert_eq!(lea[0].name, "Léa Perret");
    assert_eq!(lea[0].email, "etrangere@example.org");

    // Plusieurs mots, dans n'importe quel ordre, nom ou adresse.
    let awa = chercher("fall awa").await.expect("recherche");
    assert_eq!(awa.len(), 1);
    assert_eq!(awa[0].email, "animatrice@example.org");
    assert_eq!(chercher("inscrit1@").await.expect("recherche").len(), 1);

    // Moins de deux caractères, ou des jokers : rien.
    assert!(chercher("a").await.expect("recherche").is_empty());
    assert!(chercher("%%").await.expect("recherche").is_empty());

    // Une personne suspendue n'est pas proposée.
    sqlx::query!(
        "UPDATE identity.people SET status = 'suspended', suspended_until = now() + interval '1 day'
          WHERE id = $1",
        t.etrangere
    )
    .execute(bac.pool())
    .await
    .expect("suspension");
    assert!(chercher("perret").await.expect("recherche").is_empty());

    // Huit au plus.
    for i in 0..10 {
        let adresse = Box::leak(format!("homonyme{i}@example.org").into_boxed_str());
        commun::personne(&bac, adresse, "Ousmane", "Homonyme").await;
    }
    assert_eq!(chercher("homonyme").await.expect("recherche").len(), 8);
}

#[tokio::test]
async fn la_saisie_est_reservee_a_qui_peut_ecrire() {
    let bac = Bac::monter().await;
    let t = commun::terrain(&bac).await;
    let voisine = commun::personne(&bac, "voisine@ifdd.org", "Inès", "Diop").await;
    commun::attribuer(&bac, voisine, "admin", "event", Some(Uuid::now_v7())).await;
    for intrus in [voisine, t.etrangere] {
        let refus = mailings::suggestions(&bac.state, intrus, t.edition, "perret")
            .await
            .expect_err("refusé");
        assert_eq!(refus.code, ErrorCode::NotFound);
    }
}
