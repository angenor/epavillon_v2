//! **Le point de contrôle du stockage réel — celui que le quickstart demande à
//! la main, outillé.**
//!
//! # Pourquoi il est ignoré par défaut
//!
//! `make check-db` exécute `down -v`, ce qui efface le layout de Garage. Un test
//! qui le frapperait à chaque `cargo test` échouerait après chaque vérification
//! complète, et l'on prendrait l'habitude de le sauter — *« une commande de
//! vérification qui échoue toujours de la même façon finit par se lire comme du
//! bruit »* (B6, R7). Il ne tourne donc que sur demande :
//!
//! ```bash
//! make garage-init                       # layout, bucket, clé
//! cargo test -p media --test stockage_s3 -- --ignored --nocapture
//! ```
//!
//! # Ce qu'il tranche, et qu'aucun autre test ne tranche
//!
//! **Le critère de bascule de R8.** La signature est éprouvée ailleurs contre
//! les vecteurs d'exemple d'AWS — ce qui prouve le calcul, pas l'entente avec
//! Garage : en-têtes réellement transmis, style de chemin, longueur de corps.
//! Contre un vrai stockage, un 403 laisse trois causes également plausibles, et
//! seul cet aller-retour les départage.
//!
//! S'il ne passe pas **en une demi-journée**, la décision écrite d'avance est de
//! prendre `aws-sdk-s3` et de le consigner.

use kernel::config::{MediaStorage, S3Config, Secret};
use media::service::stream;
use media::storage::{s3::S3Store, ObjectStore};
use uuid::Uuid;

/// La configuration du `.env`, telle que `make garage-init` l'a importée dans
/// Garage. Sans elle, le test s'arrête en le disant plutôt qu'en échouant sur un
/// 403 qui ferait chercher ailleurs.
fn stockage() -> Option<S3Store> {
    let _ = dotenvy::dotenv();
    let config = kernel::config::Config::from_env().ok()?;
    if config.media.storage != MediaStorage::S3 {
        eprintln!("MEDIA_STORAGE ne vaut pas s3 : rien à éprouver ici.");
        return None;
    }
    Some(S3Store::new(&S3Config {
        endpoint: config.media.s3.endpoint.clone(),
        region: config.media.s3.region.clone(),
        bucket: config.media.s3.bucket.clone(),
        access_key_id: config.media.s3.access_key_id.clone(),
        secret_access_key: Secret::from(config.media.s3.secret_access_key.expose().to_owned()),
        force_path_style: config.media.s3.force_path_style,
    }))
}

/// **Les cinq verbes, contre Garage.** C'est l'aller-retour qui tranche.
#[tokio::test]
#[ignore = "exige Garage démarré et `make garage-init` — point de contrôle manuel du quickstart"]
async fn les_cinq_verbes_repondent_contre_garage() {
    let Some(store) = stockage() else { return };

    let jeton = Uuid::now_v7().simple().to_string();
    let cle = format!("_epreuve/{jeton}/original.txt");
    let cle_deplacee = format!("_epreuve/{jeton}/deplace.txt");
    let contenu = format!("ePavillon v2 — épreuve de signature {jeton}").into_bytes();

    // PUT en flux — le chemin réel d'un dépôt.
    let ecrits = store
        .put_stream(
            &cle,
            "text/plain",
            stream::flux_en_tranches(contenu.clone(), 7),
        )
        .await
        .expect("PUT en flux refusé par Garage : la signature ou la configuration est en cause");
    assert_eq!(ecrits, contenu.len() as u64);

    // HEAD.
    let info = store.head(&cle).await.expect("HEAD refusé");
    assert_eq!(info.byte_size, contenu.len() as i64);

    // GET.
    assert_eq!(store.get(&cle).await.expect("GET refusé"), contenu);

    // RENAME — une copie côté serveur, puis une suppression.
    store
        .rename(&cle, &cle_deplacee)
        .await
        .expect("copie côté serveur refusée");
    assert!(store.head(&cle).await.is_err());
    assert_eq!(store.get(&cle_deplacee).await.expect("GET refusé"), contenu);

    // DELETE, deux fois : supprimer ce qui n'existe pas est un succès, sans quoi
    // la purge mourrait sur un objet déjà parti.
    store.delete(&cle_deplacee).await.expect("DELETE refusé");
    store
        .delete(&cle_deplacee)
        .await
        .expect("un second DELETE doit rester un succès");

    eprintln!("Signature S3 au vert contre Garage — le critère de bascule de R8 est levé.");
}

/// Un contenu de quelques mégaoctets, pour éprouver que le tampon local et la
/// relecture par tranches se comportent comme sur un petit fichier.
#[tokio::test]
#[ignore = "exige Garage démarré et `make garage-init`"]
async fn un_contenu_de_plusieurs_mebioctets_traverse() {
    let Some(store) = stockage() else { return };

    let cle = format!("_epreuve/{}/gros.bin", Uuid::now_v7().simple());
    let contenu: Vec<u8> = (0..4 * 1024 * 1024).map(|i| (i % 251) as u8).collect();

    let ecrits = store
        .put_stream(
            &cle,
            "application/octet-stream",
            stream::flux_en_tranches(contenu.clone(), 64 * 1024),
        )
        .await
        .expect("PUT en flux refusé");
    assert_eq!(ecrits, contenu.len() as u64);
    assert_eq!(store.get(&cle).await.expect("GET refusé"), contenu);

    store.delete(&cle).await.expect("DELETE refusé");
}
