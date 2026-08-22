//! **Le quota est opposable, et le refus dit quoi faire.**
//!
//! Deux refus, un seul code. Le contrôle préalable et le refus de la base sont
//! atteignables tous les deux — la course du cas limite n° 13 le garantit —, et
//! **faire porter deux codes au même refus obligerait l'écran à traiter deux
//! fois le même cas** (R14, écart n° 136).

mod commun;

use commun::Bac;
use kernel::ErrorCode;

/// **Le refus préalable porte le même code que celui de la base**, et ses trois
/// chiffres.
#[tokio::test]
async fn le_refus_de_quota_prealable_porte_le_code_et_les_chiffres() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    // Un plafond de quelques kilo-octets : l'image d'épreuve pèse bien plus.
    commun::quota(&bac, terrain.organisation, 4096, 10).await;

    let logo = commun::couverture_16_9();
    let erreur = commun::deposer(
        &bac,
        terrain.referente,
        &logo,
        commun::metadonnees_pour(&logo, "org", "organizations", terrain.organisation, "logo"),
    )
    .await
    .expect_err("le quota devait refuser");

    assert_eq!(erreur.code, ErrorCode::MediaQuotaExceeded);
    // Les trois chiffres sont **dans le message**, que l'écran affiche tel quel :
    // « l'espace est atteint » sans chiffre ne dit pas quoi faire.
    assert!(
        erreur.message.contains("Kio") || erreur.message.contains("octets"),
        "le message ne porte aucun chiffre : {}",
        erreur.message
    );
    assert!(erreur.message.contains("il reste"));
}

/// **Rien n'est écrit** quand le quota refuse : ni ligne, ni objet sur le
/// stockage.
#[tokio::test]
async fn un_refus_de_quota_necrit_rien() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    commun::quota(&bac, terrain.organisation, 4096, 10).await;

    let logo = commun::couverture_16_9();
    let _ = commun::deposer(
        &bac,
        terrain.referente,
        &logo,
        commun::metadonnees_pour(&logo, "org", "organizations", terrain.organisation, "logo"),
    )
    .await;

    let objets = sqlx::query_scalar!(r#"SELECT count(*) AS "n!" FROM media.assets"#)
        .fetch_one(bac.pool())
        .await
        .expect("comptage");
    assert_eq!(objets, 0);
}

/// **Le refus de la base sort sous le même code que le refus préalable.**
///
/// Il est provoqué en abaissant le plafond **entre** le contrôle préalable et
/// l'écriture — ce que la course du cas limite n° 13 produit naturellement en
/// production, et qu'on reproduit ici en déposant sans annoncer de poids : le
/// contrôle préalable n'a alors rien à vérifier, et c'est `tg_enforce_quota` qui
/// tranche.
#[tokio::test]
async fn le_refus_de_la_base_porte_le_meme_code() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    commun::quota(&bac, terrain.organisation, 4096, 10).await;

    let logo = commun::couverture_16_9();
    let mut metadonnees =
        commun::metadonnees_pour(&logo, "org", "organizations", terrain.organisation, "logo");
    // Sans poids annoncé, le contrôle préalable porte sur zéro octet et passe :
    // seul le déclencheur de la base peut alors refuser.
    metadonnees.byte_size = None;

    let erreur = commun::deposer(&bac, terrain.referente, &logo, metadonnees)
        .await
        .expect_err("la base devait refuser");

    assert_eq!(
        erreur.code,
        ErrorCode::MediaQuotaExceeded,
        "le SQLSTATE 53100 doit sortir sous le code métier, jamais en erreur système"
    );
    assert!(erreur.message.contains("il reste"));
}

/// Et le stockage n'en garde rien : le temporaire comme l'objet définitif sont
/// retirés quand la base refuse.
#[tokio::test]
async fn un_refus_de_la_base_ne_laisse_rien_sur_le_stockage() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    commun::quota(&bac, terrain.organisation, 4096, 10).await;

    let logo = commun::couverture_16_9();
    let mut metadonnees =
        commun::metadonnees_pour(&logo, "org", "organizations", terrain.organisation, "logo");
    metadonnees.byte_size = None;
    let _ = commun::deposer(&bac, terrain.referente, &logo, metadonnees).await;

    let racine = std::path::PathBuf::from(&bac.config.media.fs_root);
    assert!(
        fichiers(&racine).is_empty(),
        "le stockage garde des octets d'un dépôt refusé : {:?}",
        fichiers(&racine)
    );
}

/// **Un objet sans organisation n'oppose aucun quota** : le plafond est porté
/// par l'organisation, et seule elle a de l'espace à consommer.
#[tokio::test]
async fn un_objet_personnel_noppose_aucun_quota() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    commun::quota(&bac, terrain.organisation, 4096, 10).await;

    let avatar = commun::vignette_1_1();
    commun::deposer(
        &bac,
        terrain.referente,
        &avatar,
        commun::metadonnees_pour(&avatar, "identity", "people", terrain.referente, "avatar"),
    )
    .await
    .expect("un avatar personnel ne consomme aucun quota d'organisation");
}

/// Le quota **suit la consommation réelle** : après un dépôt, les compteurs de
/// la base ont bougé — et c'est le déclencheur qui les tient, pas le service.
#[tokio::test]
async fn le_compteur_est_tenu_par_la_base_apres_un_depot() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let logo = commun::couverture_16_9();

    let resultat = commun::deposer(
        &bac,
        terrain.referente,
        &logo,
        commun::metadonnees_pour(&logo, "org", "organizations", terrain.organisation, "logo"),
    )
    .await
    .expect("dépôt");

    let ligne = sqlx::query!(
        r#"SELECT used_bytes AS "used_bytes!", used_files AS "used_files!"
             FROM media.storage_quotas WHERE organization_id = $1"#,
        terrain.organisation
    )
    .fetch_one(bac.pool())
    .await
    .expect("la ligne de quota est créée à la volée par la base");

    assert_eq!(ligne.used_bytes, resultat.asset.byte_size);
    assert_eq!(ligne.used_files, 1);
}

fn fichiers(racine: &std::path::Path) -> Vec<String> {
    let mut trouves = Vec::new();
    let Ok(entrees) = std::fs::read_dir(racine) else {
        return trouves;
    };
    for entree in entrees.flatten() {
        let chemin = entree.path();
        if chemin.is_dir() {
            trouves.extend(fichiers(&chemin));
        } else {
            trouves.push(chemin.to_string_lossy().into_owned());
        }
    }
    trouves
}
