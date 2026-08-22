//! Le harnais monte, et les fichiers qu'il fabrique sont ceux qu'il annonce.
//!
//! Ce fichier n'éprouve **aucune règle métier** : il éprouve la fabrique, dont
//! les neuf tests des histoires utilisateur dépendent tous. Une image dont le
//! rapport serait faux ferait échouer neuf fichiers sur une cause qu'aucun d'eux
//! ne nomme.

mod commun;

use commun::Bac;

/// Le rapport d'un fichier d'épreuve, à la tolérance que la base déclare.
fn rapport(fichier: &commun::Fichier) -> f64 {
    let image = image::load_from_memory(&fichier.octets).expect("image décodable");
    f64::from(image.width()) / f64::from(image.height())
}

#[tokio::test]
async fn le_terrain_pose_ce_quil_annonce() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    let referente =
        media::repo::cross::adhesion(bac.pool(), terrain.referente, terrain.organisation)
            .await
            .expect("lecture de l'adhésion")
            .expect("la référente adhère");
    assert!(referente.active && referente.referent);

    let membre = media::repo::cross::adhesion(bac.pool(), terrain.membre, terrain.organisation)
        .await
        .expect("lecture de l'adhésion")
        .expect("le membre adhère");
    assert!(membre.active && !membre.referent);

    assert!(
        media::repo::cross::adhesion(bac.pool(), terrain.etrangere, terrain.organisation)
            .await
            .expect("lecture de l'adhésion")
            .is_none()
    );

    assert!(
        media::repo::cross::edition_existe(bac.pool(), terrain.edition)
            .await
            .expect("lecture de l'édition")
    );
}

/// **Les trois formes que la table blanche exige d'une édition**, à la tolérance
/// de 2 % que le modèle déclare. Une image d'épreuve mal cadrée ferait tomber le
/// refus de forme dans un test qui vérifie tout autre chose.
#[tokio::test]
async fn les_images_depreuve_ont_les_formes_annoncees() {
    for (fichier, attendu) in [
        (commun::bandeau_32_9(), 32.0 / 9.0),
        (commun::couverture_16_9(), 16.0 / 9.0),
        (commun::vignette_1_1(), 1.0),
    ] {
        let obtenu = rapport(&fichier);
        assert!(
            (obtenu - attendu).abs() <= attendu * 0.02,
            "{} : rapport {obtenu}, attendu {attendu}",
            fichier.nom
        );
    }

    // Celle-ci ne respecte AUCUNE des trois : c'est elle qui fait tomber le
    // refus, avec ses trois chiffres.
    let mal_cadree = rapport(&commun::image_mal_cadree());
    for attendu in [32.0 / 9.0, 16.0 / 9.0, 1.0] {
        assert!((mal_cadree - attendu).abs() > attendu * 0.02);
    }
}

/// Le damier n'est pas un ornement : une image d'une seule couleur se comprime à
/// quelques octets, et les compteurs de quota ne mesureraient plus rien.
#[tokio::test]
async fn les_images_depreuve_pesent_quelque_chose() {
    assert!(commun::bandeau_32_9().octets.len() > 50_000);
    assert!(commun::couverture_16_9().octets.len() > 50_000);
}

/// C'est la transparence qui décide du format des déclinaisons — PNG plutôt que
/// JPEG. Le harnais doit donc pouvoir produire les deux.
#[tokio::test]
async fn la_transparence_est_reellement_portee() {
    let opaque = image::load_from_memory(&commun::couverture_16_9().octets).expect("décodable");
    assert!(!opaque.color().has_alpha());

    let transparente =
        image::load_from_memory(&commun::image_transparente("logo.png", 400, 400).octets)
            .expect("décodable");
    assert!(transparente.color().has_alpha());
}

/// **Le stockage des tests est le système de fichiers**, et il écrit vraiment.
#[tokio::test]
async fn le_stockage_des_tests_ecrit_sur_des_fichiers() {
    let bac = Bac::monter().await;
    let stockage = bac.state.storage();
    assert_eq!(stockage.engine(), "filesystem");

    let fichier = commun::vignette_1_1();
    let cle = "2026/08/018f0000000070008000000000000001/vignette.png";

    stockage
        .put(cle, fichier.mime, fichier.octets.clone())
        .await
        .expect("dépôt");

    let info = stockage.head(cle).await.expect("relecture des métadonnées");
    assert_eq!(info.byte_size, fichier.octets.len() as i64);
    assert_eq!(stockage.get(cle).await.expect("relecture"), fichier.octets);

    // Renommer déplace : sans quoi il faudrait relire et réécrire deux cents
    // mégaoctets pour ranger un fond vidéo.
    let definitive = "2026/08/018f0000000070008000000000000001/definitive.png";
    stockage.rename(cle, definitive).await.expect("renommage");
    assert!(stockage.head(cle).await.is_err());
    assert!(stockage.head(definitive).await.is_ok());

    // Supprimer ce qui n'existe pas est un succès : la purge se rejoue.
    stockage.delete(definitive).await.expect("suppression");
    stockage
        .delete(definitive)
        .await
        .expect("seconde suppression");
}

/// **L'analyseur des tests est `none`, et il ne déclare jamais un fichier sain.**
#[tokio::test]
async fn lanalyseur_des_tests_est_declare() {
    let bac = Bac::monter().await;
    let verdict = bac.state.scanner().analyser(b"peu importe").await;
    assert_eq!(verdict.engine, "none");
    assert_eq!(verdict.verdict, "unsupported");
}

/// **Le semis ne pose aucun objet**, et le quota par défaut est le seul en base.
#[tokio::test]
async fn le_semis_ne_fournit_aucun_objet() {
    let bac = Bac::monter().await;

    let objets = sqlx::query_scalar!(r#"SELECT count(*) AS "n!" FROM media.assets"#)
        .fetch_one(bac.pool())
        .await
        .expect("comptage des objets");
    assert_eq!(objets, 0);

    let quotas = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM media.storage_quotas WHERE organization_id IS NOT NULL"#
    )
    .fetch_one(bac.pool())
    .await
    .expect("comptage des quotas");
    assert_eq!(quotas, 0);
}

/// **LA TABLE DE GARDES N'A AUCUN TROU.**
///
/// La table blanche est faite pour s'allonger : un module qui y ajoute une ligne
/// sans passer par `domain/guards.rs` laisserait une porte **ouverte**, et rien
/// à la compilation ne le signalerait. Ce test lit la table **en base** — c'est
/// la seule façon de le voir (B6, R15).
#[tokio::test]
async fn aucune_ligne_de_la_table_blanche_nest_sans_garde() {
    let bac = Bac::monter().await;

    let couples = sqlx::query!(
        "SELECT DISTINCT owner_schema, owner_table
           FROM media.attachable_roles
          ORDER BY owner_schema, owner_table"
    )
    .fetch_all(bac.pool())
    .await
    .expect("lecture de la table blanche");

    assert!(!couples.is_empty(), "la table blanche est semée");

    let sans_garde: Vec<String> = couples
        .iter()
        .filter(|c| media::domain::guards::garde_pour(&c.owner_schema, &c.owner_table).is_none())
        .map(|c| format!("{}.{}", c.owner_schema, c.owner_table))
        .collect();

    assert!(
        sans_garde.is_empty(),
        "porte ouverte : {sans_garde:?} n'ont pas de garde dans domain/guards.rs"
    );
}
