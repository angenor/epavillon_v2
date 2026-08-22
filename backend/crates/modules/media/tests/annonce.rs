//! **L'annonce est une question, pas une tentative.**
//!
//! Elle rend le verdict que rendrait le dépôt — et **n'écrit rien** : ni ligne,
//! ni octet sur le stockage, ni réservation d'aucune sorte. Sans envoi qui
//! suive, il ne reste aucune trace (FR-016).

mod commun;

use commun::Bac;
use kernel::ErrorCode;
use media::service::upload::{self, UploadDeclaration};
use uuid::Uuid;

fn declaration(fichier: &commun::Fichier) -> UploadDeclaration {
    UploadDeclaration {
        filename: fichier.nom.to_owned(),
        mime_type: fichier.mime.to_owned(),
        byte_size: fichier.octets.len() as i64,
        owner_schema: None,
        owner_table: None,
        owner_id: None,
        role: None,
        checksum_sha256: None,
    }
}

fn pour(
    fichier: &commun::Fichier,
    schema: &str,
    table: &str,
    id: Uuid,
    role: &str,
) -> UploadDeclaration {
    UploadDeclaration {
        owner_schema: Some(schema.to_owned()),
        owner_table: Some(table.to_owned()),
        owner_id: Some(id),
        role: Some(role.to_owned()),
        ..declaration(fichier)
    }
}

/// L'état du monde, pour prouver qu'une annonce ne l'a pas changé.
async fn empreinte_du_monde(bac: &Bac) -> (i64, i64, usize) {
    let objets = sqlx::query_scalar!(r#"SELECT count(*) AS "n!" FROM media.assets"#)
        .fetch_one(bac.pool())
        .await
        .expect("comptage des objets");
    let travaux = sqlx::query_scalar!(r#"SELECT count(*) AS "n!" FROM platform.jobs"#)
        .fetch_one(bac.pool())
        .await
        .expect("comptage des travaux");
    let fichiers = fichiers(&std::path::PathBuf::from(&bac.config.media.fs_root)).len();
    (objets, travaux, fichiers)
}

/// **Les cinq cas de l'annonce, et aucun n'écrit quoi que ce soit.**
#[tokio::test]
async fn lannonce_rend_son_verdict_sans_rien_ecrire() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let avant = empreinte_du_monde(&bac).await;

    let image = commun::couverture_16_9();
    let pdf = commun::document_pdf();

    // 1. Accepté.
    let accepte = upload::annoncer(&bac.state, terrain.referente, &declaration(&image))
        .await
        .expect("annonce");
    assert!(accepte.accepted);
    assert!(accepte.code.is_none());

    // 2. Type refusé par le rôle visé.
    let type_refuse = upload::annoncer(
        &bac.state,
        terrain.referente,
        &pour(&pdf, "org", "organizations", terrain.organisation, "logo"),
    )
    .await
    .expect("annonce");
    assert!(!type_refuse.accepted);
    assert_eq!(
        type_refuse.code.as_deref(),
        Some(ErrorCode::MediaMimeNotAllowed.as_str())
    );
    assert_eq!(type_refuse.field.as_deref(), Some("file"));

    // 3. Poids refusé par le rôle visé — un logo est plafonné à 5 Mio.
    let mut trop_lourd = declaration(&image);
    trop_lourd.byte_size = 6 * 1024 * 1024;
    trop_lourd.owner_schema = Some("org".to_owned());
    trop_lourd.owner_table = Some("organizations".to_owned());
    trop_lourd.owner_id = Some(terrain.organisation);
    trop_lourd.role = Some("logo".to_owned());
    let poids_refuse = upload::annoncer(&bac.state, terrain.referente, &trop_lourd)
        .await
        .expect("annonce");
    assert!(!poids_refuse.accepted);
    assert_eq!(
        poids_refuse.code.as_deref(),
        Some(ErrorCode::MediaTooLarge.as_str())
    );

    // 4. Quota atteint — et **les trois chiffres sont dans la réponse**, sous
    //    forme structurée : c'est un 200, il porte une valeur.
    commun::quota(&bac, terrain.organisation, 4096, 10).await;
    let quota_refuse = upload::annoncer(
        &bac.state,
        terrain.referente,
        &pour(&image, "org", "organizations", terrain.organisation, "logo"),
    )
    .await
    .expect("annonce");
    assert!(!quota_refuse.accepted);
    assert_eq!(
        quota_refuse.code.as_deref(),
        Some(ErrorCode::MediaQuotaExceeded.as_str())
    );
    let chiffres = quota_refuse.quota.expect("les trois chiffres");
    assert_eq!(chiffres.max_bytes, 4096);
    assert_eq!(chiffres.used_bytes, 0);
    assert_eq!(chiffres.remaining_bytes, 4096);

    // 5. Rôle non déclaré pour cette entité.
    let role_inconnu = upload::annoncer(
        &bac.state,
        terrain.referente,
        &pour(
            &image,
            "org",
            "organizations",
            terrain.organisation,
            "banner",
        ),
    )
    .await
    .expect("annonce");
    assert!(!role_inconnu.accepted);
    assert_eq!(
        role_inconnu.code.as_deref(),
        Some(ErrorCode::MediaRoleNotDeclared.as_str())
    );

    // **Rien n'a bougé.** Ni ligne, ni travail, ni octet.
    assert_eq!(empreinte_du_monde(&bac).await, avant);
}

/// **Une empreinte déjà connue rend l'objet existant, sans qu'un octet parte.**
///
/// C'est la seule situation où la déduplication économise aussi la bande
/// passante — et la seule raison pour laquelle l'annonce accepte une empreinte.
#[tokio::test]
async fn une_empreinte_deja_connue_rend_lobjet_existant() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let image = commun::couverture_16_9();

    let depot = commun::deposer(&bac, terrain.referente, &image, commun::metadonnees(&image))
        .await
        .expect("dépôt");

    let mut declaration = declaration(&image);
    declaration.checksum_sha256 = Some(depot.asset.checksum_sha256.clone());

    let verdict = upload::annoncer(&bac.state, terrain.etrangere, &declaration)
        .await
        .expect("annonce");

    assert!(verdict.accepted, "un doublon est un succès, pas un refus");
    let existant = verdict.existing_asset.expect("l'objet existant est rendu");
    assert_eq!(existant.id, depot.asset.id);
    // L'adresse est là : l'écran peut afficher l'image sans rien envoyer.
    assert!(existant.url.starts_with("http"));
}

/// Une empreinte **inconnue** ne fait rien croire : le verdict porte sur le
/// reste, comme si aucune empreinte n'avait été fournie.
#[tokio::test]
async fn une_empreinte_inconnue_ne_change_rien_au_verdict() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let image = commun::couverture_16_9();

    let mut declaration = declaration(&image);
    declaration.checksum_sha256 = Some("0".repeat(64));

    let verdict = upload::annoncer(&bac.state, terrain.referente, &declaration)
        .await
        .expect("annonce");
    assert!(verdict.accepted);
    assert!(verdict.existing_asset.is_none());
}

/// **Le refus de droit est le seul qui sorte en erreur** : il ne se distingue
/// pas d'une entité inexistante, et n'a donc rien de plus à dire qu'un 404.
#[tokio::test]
async fn le_refus_de_droit_sort_en_erreur_et_non_en_verdict() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let image = commun::vignette_1_1();

    let erreur = upload::annoncer(
        &bac.state,
        terrain.etrangere,
        &pour(&image, "org", "organizations", terrain.organisation, "logo"),
    )
    .await
    .expect_err("une étrangère n'annonce rien sur cette fiche");

    assert_eq!(erreur.code, ErrorCode::NotFound);
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
