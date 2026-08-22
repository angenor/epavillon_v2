//! **Le droit de déposer est le droit d'écrire sur ce que le fichier illustre.**
//!
//! Aucune permission `media.*` n'existe dans le modèle (écart n° 127). La garde
//! vit donc dans `domain/guards.rs`, et ces tests éprouvent deux choses qu'une
//! relecture ne remplace pas : **le refus prend la forme d'une absence**, et
//! **aucune ligne de la table blanche n'est sans garde**.

mod commun;

use commun::Bac;
use kernel::ErrorCode;
use uuid::Uuid;

/// **Un hors-périmètre se refuse comme un inexistant.**
///
/// C'est la règle du principe IX : un 403 dirait à qui forge une URL que
/// l'organisation existe. Les deux refus doivent être **indiscernables**.
#[tokio::test]
async fn un_hors_perimetre_recoit_le_meme_refus_quun_inexistant() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let logo = commun::vignette_1_1();

    // Une organisation réelle, mais dont l'appelante n'est pas membre.
    let autre = commun::organisation(&bac, "Institut de la Francophonie", "IFDD").await;
    let hors_perimetre = commun::deposer(
        &bac,
        terrain.etrangere,
        &logo,
        commun::metadonnees_pour(&logo, "org", "organizations", autre, "logo"),
    )
    .await
    .expect_err("une étrangère ne pose pas de logo");

    // Une organisation qui n'existe pas.
    let inexistante = commun::deposer(
        &bac,
        terrain.etrangere,
        &logo,
        commun::metadonnees_pour(&logo, "org", "organizations", Uuid::now_v7(), "logo"),
    )
    .await
    .expect_err("une organisation inexistante");

    assert_eq!(hors_perimetre.code, ErrorCode::NotFound);
    assert_eq!(inexistante.code, ErrorCode::NotFound);
    assert_eq!(hors_perimetre.message, inexistante.message);
    assert_eq!(hors_perimetre.field, inexistante.field);
}

/// **Le logo exige un référent, pas un simple membre.** Il engage la fiche
/// publique de l'organisation.
#[tokio::test]
async fn le_logo_exige_un_referent_et_non_un_simple_membre() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let logo = commun::vignette_1_1();

    commun::deposer(
        &bac,
        terrain.referente,
        &logo,
        commun::metadonnees_pour(&logo, "org", "organizations", terrain.organisation, "logo"),
    )
    .await
    .expect("la référente pose le logo");

    let mut autre = commun::vignette_1_1();
    autre.nom = "autre-logo.png";
    let erreur = commun::deposer(
        &bac,
        terrain.membre,
        &autre,
        commun::metadonnees_pour(&autre, "org", "organizations", terrain.organisation, "logo"),
    )
    .await
    .expect_err("un simple membre ne pose pas le logo");

    assert_eq!(erreur.code, ErrorCode::NotFound);
}

/// **Une photo de profil appartient à qui elle représente.**
#[tokio::test]
async fn un_avatar_est_a_soi_meme_et_a_personne_dautre() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let avatar = commun::vignette_1_1();

    commun::deposer(
        &bac,
        terrain.referente,
        &avatar,
        commun::metadonnees_pour(&avatar, "identity", "people", terrain.referente, "avatar"),
    )
    .await
    .expect("chacun pose sa propre photo");

    let mut autre = commun::vignette_1_1();
    autre.nom = "photo-volee.png";
    let erreur = commun::deposer(
        &bac,
        terrain.etrangere,
        &autre,
        commun::metadonnees_pour(&autre, "identity", "people", terrain.referente, "avatar"),
    )
    .await
    .expect_err("on ne pose pas la photo de quelqu'un d'autre");

    assert_eq!(erreur.code, ErrorCode::NotFound);
}

/// **Une édition exige la permission ET le périmètre** — règle métier n° 8.
///
/// Une référente d'organisation n'administre rien : elle ne pose pas le bandeau
/// d'une COP, quelle que soit son adhésion.
#[tokio::test]
async fn une_edition_exige_la_permission_et_le_perimetre() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let bandeau = commun::bandeau_32_9();

    let erreur = commun::deposer(
        &bac,
        terrain.referente,
        &bandeau,
        commun::metadonnees_pour(&bandeau, "event", "events", terrain.edition, "banner"),
    )
    .await
    .expect_err("une organisation n'administre aucune édition");

    assert_eq!(erreur.code, ErrorCode::NotFound);
}

/// **Une combinaison sans garde est REFUSÉE, jamais autorisée par défaut.**
///
/// C'est ce qui fait qu'une table blanche qui s'allonge ne devient jamais une
/// porte ouverte : le jour où un module y ajoute une ligne sans passer par
/// `domain/guards.rs`, le rattachement est refusé — bruyamment, et non
/// silencieusement permis.
#[tokio::test]
async fn une_combinaison_sans_garde_est_refusee() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let image = commun::vignette_1_1();

    let erreur = commun::deposer(
        &bac,
        terrain.referente,
        &image,
        commun::metadonnees_pour(&image, "negotiation", "documents", Uuid::now_v7(), "cover"),
    )
    .await
    .expect_err("une combinaison non déclarée n'est jamais autorisée");

    assert_eq!(erreur.code, ErrorCode::NotFound);
}

/// **Le module Formations porte une garde FERMÉE, et son motif est écrit.**
///
/// Il ne déclare aucune permission dans le modèle : aucune garde ne peut
/// s'écrire tant qu'il n'en porte pas. Le refus est **une décision**, et le
/// distinguer d'une garde oubliée est tout l'intérêt de la variante.
#[tokio::test]
async fn la_garde_fermee_refuse_en_disant_pourquoi() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let image = commun::couverture_16_9();

    let erreur = commun::deposer(
        &bac,
        terrain.referente,
        &image,
        commun::metadonnees_pour(&image, "training", "trainings", Uuid::now_v7(), "cover"),
    )
    .await
    .expect_err("le module Formations n'a aucune permission");

    assert_eq!(erreur.code, ErrorCode::NotFound);
    assert!(
        erreur
            .detail
            .as_deref()
            .is_some_and(|d| d.contains("aucune permission")),
        "le motif du refus doit partir dans la trace : {:?}",
        erreur.detail
    );
}

/// **LA TABLE BLANCHE N'A AUCUNE LIGNE SANS GARDE.**
///
/// Lu **en base**, et non déduit du code : une table blanche est faite pour
/// s'allonger, et rien à la compilation ne signalerait une garde manquante.
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

/// **Sans entité porteuse, aucune garde ne s'applique** : un objet peut être
/// déposé puis rattaché plus tard, et c'est le rattachement qui l'éprouvera.
#[tokio::test]
async fn un_depot_sans_entite_porteuse_est_libre() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let image = commun::vignette_1_1();

    let resultat = commun::deposer(&bac, terrain.etrangere, &image, commun::metadonnees(&image))
        .await
        .expect("un dépôt sans rôle visé n'a rien à garder");

    // Il n'appartient alors qu'à la personne : aucune organisation, aucun quota.
    assert_eq!(resultat.asset.owner_organization_id, None);
    assert_eq!(resultat.asset.owner_person_id, Some(terrain.etrangere));
}
