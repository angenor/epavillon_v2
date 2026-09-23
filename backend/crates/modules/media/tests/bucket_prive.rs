//! **Un objet privé ne touche jamais le bucket ouvert au web** (specs/011 R4).
//!
//! Le bucket public est servi sans droits par le relais média : un PDF réservé
//! qui y atterrirait serait lisible par quiconque obtient son adresse.

mod commun;

use commun::Bac;
use media::service::upload::MetadonneesDepot;

const BUCKET_PRIVE: &str = "epavillon-prive";

fn privees(fichier: &commun::Fichier) -> MetadonneesDepot {
    MetadonneesDepot {
        visibility: Some("private".to_owned()),
        ..commun::metadonnees(fichier)
    }
}

#[tokio::test]
async fn un_objet_prive_atterrit_dans_le_bucket_prive_et_nulle_part_ailleurs() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let fichier = commun::document_pdf();

    let resultat = commun::deposer(&bac, terrain.referente, &fichier, privees(&fichier))
        .await
        .expect("dépôt");

    assert_eq!(resultat.asset.bucket, BUCKET_PRIVE);
    assert_eq!(resultat.asset.visibility, "private");
    let cle = &resultat.asset.object_key;
    assert_eq!(
        bac.state
            .stockage(BUCKET_PRIVE)
            .get(cle)
            .await
            .expect("dans le bucket privé"),
        fichier.octets
    );
    assert!(
        bac.state.storage().get(cle).await.is_err(),
        "l'objet privé ne doit pas être dans le bucket public"
    );
}

#[tokio::test]
async fn un_objet_public_reste_dans_le_bucket_par_defaut() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let fichier = commun::document_pdf();

    let resultat = commun::deposer(
        &bac,
        terrain.referente,
        &fichier,
        commun::metadonnees(&fichier),
    )
    .await
    .expect("dépôt");

    assert_eq!(resultat.asset.bucket, "epavillon");
    assert!(bac
        .state
        .storage()
        .get(&resultat.asset.object_key)
        .await
        .is_ok());
    assert!(bac
        .state
        .stockage(BUCKET_PRIVE)
        .get(&resultat.asset.object_key)
        .await
        .is_err());
}

/// Le traitement relit l'objet là où il a été déposé : sans cela, un PDF privé
/// resterait en analyse pour toujours.
#[tokio::test]
async fn le_traitement_lit_lobjet_prive_dans_son_bucket() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let fichier = commun::document_pdf();

    let resultat = commun::deposer(&bac, terrain.referente, &fichier, privees(&fichier))
        .await
        .expect("dépôt");
    let issues = commun::passer_le_worker(&bac).await;
    assert!(issues.iter().all(Result::is_ok), "{issues:?}");

    let etat: String = sqlx::query_scalar("SELECT status::text FROM media.assets WHERE id = $1")
        .bind(resultat.asset.id)
        .fetch_one(bac.pool())
        .await
        .expect("état");
    assert_eq!(etat, "ready");
}
