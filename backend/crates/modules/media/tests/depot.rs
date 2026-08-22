//! **Un fichier entre dans la plateforme, et il n'occupe la place qu'une fois.**
//!
//! Ce que ces tests éprouvent et qu'aucune relecture de code ne remplace : le
//! fichier est **réellement** sur le stockage, et un contenu déjà connu n'y
//! écrit **rien** de plus.

mod commun;

use commun::Bac;

/// **Le dépôt écrit l'objet, et le fichier est là.**
///
/// « L'objet est décrit en base » ne prouve rien : une description qui pointe
/// vers une clé absente est exactement le défaut qu'on veut voir. On relit donc
/// le stockage lui-même, à la clé que l'objet annonce.
#[tokio::test]
async fn un_depot_ecrit_lobjet_et_le_fichier_est_reellement_la() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let fichier = commun::couverture_16_9();

    let resultat = commun::deposer(
        &bac,
        terrain.referente,
        &fichier,
        commun::metadonnees(&fichier),
    )
    .await
    .expect("dépôt");

    assert!(!resultat.deduplique);
    assert_eq!(resultat.asset.byte_size, fichier.octets.len() as i64);
    assert_eq!(resultat.asset.mime_type, "image/png");
    assert_eq!(resultat.asset.status, "uploaded");
    assert_eq!(resultat.asset.owner_person_id, Some(terrain.referente));

    let contenu = commun::lire_sur_le_stockage(&bac, &resultat.asset.object_key)
        .await
        .expect("le fichier est sur le stockage");
    assert_eq!(contenu, fichier.octets);
}

/// **L'adresse est composée, jamais la clé nue** (FR-021).
#[tokio::test]
async fn lobjet_rend_une_adresse_composee_et_jamais_sa_cle() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let fichier = commun::vignette_1_1();

    let resultat = commun::deposer(
        &bac,
        terrain.referente,
        &fichier,
        commun::metadonnees(&fichier),
    )
    .await
    .expect("dépôt");

    assert!(resultat.asset.url.starts_with("http"));
    assert!(resultat.asset.url.ends_with(&resultat.asset.object_key));
    // La clé suit la convention du modèle : ni barre initiale, ni espace.
    assert!(!resultat.asset.object_key.starts_with('/'));
    assert!(!resultat.asset.object_key.contains(char::is_whitespace));
    // Les déclinaisons sont **vides mais présentes** : l'écran affiche
    // l'original, pas un trou.
    assert_eq!(resultat.asset.sources, serde_json::json!({}));
}

/// **Le même contenu sous un autre nom n'écrit aucun second objet.**
///
/// C'est la promesse de la déduplication, et elle se mesure sur deux choses : le
/// nombre de lignes, et l'identifiant rendu.
#[tokio::test]
async fn le_meme_contenu_sous_un_autre_nom_necrit_aucun_second_objet() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    let premier = commun::couverture_16_9();
    let depot = commun::deposer(
        &bac,
        terrain.referente,
        &premier,
        commun::metadonnees(&premier),
    )
    .await
    .expect("premier dépôt");

    let mut second = commun::couverture_16_9();
    second.nom = "tout-autre-nom.png";
    // **Une autre personne, d'une autre organisation** : la déduplication
    // traverse les propriétaires (écart n° 128), et c'est ce qui rend la
    // suppression d'un objet rattaché refusable.
    let repris = commun::deposer(
        &bac,
        terrain.etrangere,
        &second,
        commun::metadonnees(&second),
    )
    .await
    .expect("second dépôt");

    assert!(repris.deduplique);
    assert_eq!(repris.asset.id, depot.asset.id);
    assert_eq!(repris.asset.owner_person_id, Some(terrain.referente));

    let objets = compter_les_objets(&bac).await;
    assert_eq!(objets, 1, "un second objet a été écrit");
}

/// Et **aucun octet supplémentaire n'est conservé** : le temporaire du second
/// dépôt a bien été retiré du stockage.
#[tokio::test]
async fn le_second_depot_ne_laisse_rien_sur_le_stockage() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let fichier = commun::bandeau_32_9();

    commun::deposer(
        &bac,
        terrain.referente,
        &fichier,
        commun::metadonnees(&fichier),
    )
    .await
    .expect("premier dépôt");

    commun::deposer(
        &bac,
        terrain.membre,
        &fichier,
        commun::metadonnees(&fichier),
    )
    .await
    .expect("second dépôt");

    let racine = std::path::PathBuf::from(&bac.config.media.fs_root);
    let restants = fichiers_sous(&racine.join("_incoming"));
    assert!(
        restants.is_empty(),
        "des temporaires traînent sur le stockage : {restants:?}"
    );
}

/// **Le poids enregistré est celui reçu**, jamais celui annoncé — et un écart
/// entre les deux refuse le dépôt sans rien écrire (FR-017).
#[tokio::test]
async fn un_poids_annonce_faux_refuse_le_depot_sans_rien_ecrire() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let fichier = commun::vignette_1_1();

    let mut metadonnees = commun::metadonnees(&fichier);
    metadonnees.byte_size = Some(fichier.octets.len() as i64 + 1);

    let erreur = commun::deposer(&bac, terrain.referente, &fichier, metadonnees)
        .await
        .expect_err("le dépôt devait être refusé");

    assert_eq!(erreur.code, kernel::ErrorCode::MediaUploadIncomplete);
    assert_eq!(erreur.field.as_deref(), Some("file"));
    assert_eq!(compter_les_objets(&bac).await, 0);

    let racine = std::path::PathBuf::from(&bac.config.media.fs_root);
    assert!(fichiers_sous(&racine.join("_incoming")).is_empty());
}

/// **Une image sans texte alternatif est refusée sur le champ** (R9, écart
/// n° 129) — et un document sans texte alternatif est accepté.
#[tokio::test]
async fn une_image_sans_texte_alternatif_est_refusee_sur_le_champ() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    let image = commun::vignette_1_1();
    let mut sans_texte = commun::metadonnees(&image);
    sans_texte.alt_text = None;

    let erreur = commun::deposer(&bac, terrain.referente, &image, sans_texte)
        .await
        .expect_err("une image sans texte alternatif devait être refusée");

    assert_eq!(erreur.code, kernel::ErrorCode::MediaAltTextRequired);
    assert_eq!(erreur.field.as_deref(), Some("alt_text"));
    // **Rien n'a été lu, rien n'a été écrit** : le refus tombe avant le flux.
    assert_eq!(compter_les_objets(&bac).await, 0);

    // Un document n'a pas de texte alternatif à porter : la contrainte de la
    // base ne vise que les images.
    let pdf = commun::document_pdf();
    let mut metadonnees = commun::metadonnees(&pdf);
    metadonnees.alt_text = None;
    commun::deposer(&bac, terrain.referente, &pdf, metadonnees)
        .await
        .expect("un document passe sans texte alternatif");
}

/// **Un texte alternatif vide ne compte pas pour un texte alternatif.**
///
/// C'est le contournement qu'un formulaire produit tout seul : un champ laissé
/// blanc arrive comme une chaîne vide, et l'accepter donnerait un objet bloqué
/// en traitement — exactement ce que le refus existe pour éviter.
#[tokio::test]
async fn un_texte_alternatif_vide_ne_passe_pas() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let image = commun::vignette_1_1();

    let mut metadonnees = commun::metadonnees(&image);
    metadonnees.alt_text = Some(serde_json::json!({ "fr": "   " }));

    let erreur = commun::deposer(&bac, terrain.referente, &image, metadonnees)
        .await
        .expect_err("un texte alternatif vide devait être refusé");
    assert_eq!(erreur.code, kernel::ErrorCode::MediaAltTextRequired);
}

/// **LE TEST DE L'AVERTISSEMENT N° 1 : le service n'émet rien et n'enfile rien.**
///
/// Insérer une ligne dans `media.assets` déclenche `media.tg_enqueue_processing()`,
/// qui met le traitement en file **et** émet l'annonce de dépôt. Un service zélé
/// produirait **deux traitements par fichier**, et le doublon ne se verrait qu'en
/// production.
///
/// **On compte les lignes.** C'est la règle établie depuis B4 : un décompte,
/// jamais une relecture de code.
#[tokio::test]
async fn le_service_nemet_rien_et_nenfile_rien() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let fichier = commun::couverture_16_9();

    let resultat = commun::deposer(
        &bac,
        terrain.referente,
        &fichier,
        commun::metadonnees(&fichier),
    )
    .await
    .expect("dépôt");

    let evenements = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM platform.outbox_events
            WHERE aggregate_schema = 'media' AND aggregate_id = $1"#,
        resultat.asset.id
    )
    .fetch_one(bac.pool())
    .await
    .expect("comptage de l'outbox");
    assert_eq!(evenements, 1, "l'annonce de dépôt a été émise deux fois");

    let types: Vec<String> = sqlx::query_scalar!(
        "SELECT event_type FROM platform.outbox_events WHERE aggregate_id = $1",
        resultat.asset.id
    )
    .fetch_all(bac.pool())
    .await
    .expect("lecture de l'outbox");
    assert_eq!(types, vec!["media.asset.uploaded".to_owned()]);

    let travaux = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM platform.jobs
            WHERE task = 'media.process_asset' AND idempotency_key = $1"#,
        resultat.asset.id.to_string()
    )
    .fetch_one(bac.pool())
    .await
    .expect("comptage de la file");
    assert_eq!(travaux, 1, "le traitement a été mis en file deux fois");
}

/// Une déduplication **n'enfile rien du tout** : aucune ligne n'est écrite, donc
/// aucun déclencheur ne part. Sans quoi le même fichier serait traité deux fois.
#[tokio::test]
async fn une_deduplication_nenfile_aucun_second_traitement() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let fichier = commun::bandeau_32_9();

    commun::deposer(
        &bac,
        terrain.referente,
        &fichier,
        commun::metadonnees(&fichier),
    )
    .await
    .expect("premier dépôt");
    commun::deposer(
        &bac,
        terrain.membre,
        &fichier,
        commun::metadonnees(&fichier),
    )
    .await
    .expect("second dépôt");

    let travaux = compter_les_travaux(&bac).await;
    assert_eq!(travaux, 1, "un second traitement a été mis en file");
}

/// Le dépôt visant une entité porteuse **appartient à son organisation** : c'est
/// elle dont le quota est opposable.
#[tokio::test]
async fn un_depot_sur_une_fiche_dorganisation_lui_appartient() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let logo = commun::image_transparente("logo.png", 400, 400);

    let resultat = commun::deposer(
        &bac,
        terrain.referente,
        &logo,
        commun::metadonnees_pour(&logo, "org", "organizations", terrain.organisation, "logo"),
    )
    .await
    .expect("dépôt du logo");

    assert_eq!(
        resultat.asset.owner_organization_id,
        Some(terrain.organisation)
    );
    assert_eq!(resultat.asset.owner_person_id, Some(terrain.referente));
}

/// Un type refusé par la table blanche l'est **avant** toute écriture, en
/// nommant le rôle, l'attendu et le reçu.
#[tokio::test]
async fn un_type_refuse_par_le_role_est_nomme() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let pdf = commun::document_pdf();

    let erreur = commun::deposer(
        &bac,
        terrain.referente,
        &pdf,
        commun::metadonnees_pour(&pdf, "org", "organizations", terrain.organisation, "logo"),
    )
    .await
    .expect_err("un PDF n'est pas un logo");

    assert_eq!(erreur.code, kernel::ErrorCode::MediaMimeNotAllowed);
    assert_eq!(erreur.field.as_deref(), Some("file"));
    assert_eq!(compter_les_objets(&bac).await, 0);
}

/// Les deux comptages dont ces tests ont besoin. **Écrits en clair et non
/// composés** : le seul fichier du dépôt qui compose du SQL est le harnais de
/// base, et le portail de vérification l'y surveille.
async fn compter_les_objets(bac: &Bac) -> i64 {
    sqlx::query_scalar!(r#"SELECT count(*) AS "n!" FROM media.assets"#)
        .fetch_one(bac.pool())
        .await
        .expect("comptage des objets")
}

async fn compter_les_travaux(bac: &Bac) -> i64 {
    sqlx::query_scalar!(r#"SELECT count(*) AS "n!" FROM platform.jobs"#)
        .fetch_one(bac.pool())
        .await
        .expect("comptage des travaux")
}

fn fichiers_sous(racine: &std::path::Path) -> Vec<String> {
    let Ok(entrees) = std::fs::read_dir(racine) else {
        return Vec::new();
    };
    entrees
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect()
}
