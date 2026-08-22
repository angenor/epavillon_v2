//! **Le fichier devient servable sans faire attendre personne.**
//!
//! Ce que ces tests éprouvent et qu'aucune relecture de code ne remplace : le
//! travail est **réservé sur la file que le déclencheur du modèle a nommée** —
//! une file qu'aucun worker n'écoute laisserait les travaux s'empiler sans
//! erreur et sans trace —, et **une reprise ne fabrique jamais deux fois la
//! même déclinaison**.

mod commun;

use async_trait::async_trait;
use commun::Bac;
use kernel::jobs::JobHandler;
use media::scan::{Scanner, Verdict};
use std::sync::Arc;

/// Le nombre de déclinaisons attendues d'une image de 1920 px : les trois
/// tailles configurées lui sont toutes inférieures.
const DECLINAISONS_ATTENDUES: i64 = 3;

/// **L'original est servi avant les déclinaisons.**
///
/// Juste après le dépôt, l'adresse est là et la liste des déclinaisons est
/// **vide mais présente**. Un écran qui n'afficherait que les déclinaisons
/// laisserait un trou entre le dépôt et le passage du worker — ce qui est
/// exactement ce que les six colonnes d'URL de la v1 masquaient en imposant un
/// traitement synchrone.
#[tokio::test]
async fn loriginal_est_servi_avant_les_declinaisons() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let fichier = commun::couverture_16_9();

    let depose = commun::deposer(
        &bac,
        terrain.referente,
        &fichier,
        commun::metadonnees(&fichier),
    )
    .await
    .expect("dépôt");

    assert!(depose.asset.url.starts_with("http"));
    assert_eq!(
        depose.asset.sources,
        serde_json::json!({}),
        "la liste des déclinaisons doit être vide MAIS présente"
    );

    let avancement = media::service::read::avancement(&bac.state, depose.asset.id)
        .await
        .expect("avancement");
    assert_eq!(avancement.status, "uploaded");
    assert_eq!(avancement.renditions_ready, 0);
    // Rien n'est encore connu comme attendu : le relevé n'a pas eu lieu, et
    // annoncer trois attendues serait inventer un dénominateur.
    assert_eq!(avancement.renditions_expected, 0);
    assert_eq!(avancement.scan_verdict, "pending");
}

/// **Après le passage du worker : dimensions relevées, déclinaisons écrites,
/// objet servable.**
#[tokio::test]
async fn le_passage_du_worker_rend_lobjet_servable() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let fichier = commun::couverture_16_9();

    let depose = commun::deposer(
        &bac,
        terrain.referente,
        &fichier,
        commun::metadonnees(&fichier),
    )
    .await
    .expect("dépôt");

    let issues = commun::passer_le_worker(&bac).await;
    assert_eq!(issues.len(), 1, "un travail, et un seul, était en file");
    assert!(issues[0].is_ok(), "traitement : {:?}", issues[0]);

    let objet = media::service::read::objet(&bac.state, depose.asset.id)
        .await
        .expect("lecture");
    assert_eq!(objet.status, "ready");
    assert_eq!((objet.width, objet.height), (Some(1920), Some(1080)));

    let avancement = media::service::read::avancement(&bac.state, depose.asset.id)
        .await
        .expect("avancement");
    assert_eq!(avancement.renditions_expected, DECLINAISONS_ATTENDUES);
    assert_eq!(avancement.renditions_ready, DECLINAISONS_ATTENDUES);
    assert!(avancement.last_error.is_none());

    // Les déclinaisons sont **réellement** sur le stockage, et la lecture les
    // rend sous la forme qu'un `<picture>` attend : `<variante>_<format>`.
    let sources = objet.sources.as_object().expect("un objet de déclinaisons");
    assert_eq!(sources.len(), DECLINAISONS_ATTENDUES as usize);
    for code in ["lg_jpeg", "md_jpeg", "thumb_jpeg"] {
        let source = sources
            .get(code)
            .unwrap_or_else(|| panic!("{code} absente"));
        assert!(source["url"].as_str().expect("adresse").starts_with("http"));
        assert!(source["bytes"].as_i64().expect("poids") > 0);
    }
    assert_eq!(sources["md_jpeg"]["width"], 800);

    let cle = format!("_renditions/{}/thumb.jpeg", depose.asset.id.simple());
    assert!(
        commun::lire_sur_le_stockage(&bac, &cle).await.is_some(),
        "la déclinaison doit être sur le stockage, pas seulement en base"
    );
}

/// **Worker arrêté puis relancé : le traitement se fait une seule fois.**
///
/// C'est le test qui compte de cette histoire, et le point de contrôle du
/// quickstart. La file est « au moins une fois », jamais « exactement une
/// fois » : un worker tué **entre l'exécution et son marquage** laisse le
/// travail réservé, et le worker suivant le rend à la file avec sa charge utile
/// intacte. `count(*)` sur `media.renditions` doit rendre le nombre de
/// déclinaisons configurées, **jamais le double**.
///
/// Le rejeu est éprouvé **deux fois**, parce que deux gardes différentes le
/// tiennent : sur un objet déjà servable, la sortie immédiate ; sur un objet
/// resté « en traitement » — ce qu'une mort avant la mise en service produit —,
/// le relevé de ce qui a déjà été fait.
#[tokio::test]
async fn un_worker_relance_ne_fabrique_pas_deux_fois_les_declinaisons() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let fichier = commun::couverture_16_9();

    let depose = commun::deposer(
        &bac,
        terrain.referente,
        &fichier,
        commun::metadonnees(&fichier),
    )
    .await
    .expect("dépôt");

    // Premier passage, worker tué avant d'avoir marqué : le travail reste
    // réservé, et les déclinaisons sont bel et bien là.
    let issues = commun::worker_tue_apres_le_travail(&bac).await;
    assert!(issues[0].is_ok(), "traitement : {:?}", issues[0]);
    assert_eq!(
        commun::compter_declinaisons(&bac, depose.asset.id).await,
        DECLINAISONS_ATTENDUES
    );

    // Relance : le travail revient à la file, charge utile intacte.
    commun::worker_relance(&bac).await;
    let issues = commun::worker_tue_apres_le_travail(&bac).await;
    assert_eq!(issues.len(), 1, "le travail a bien été rendu à la file");
    assert!(issues[0].is_ok(), "reprise : {:?}", issues[0]);
    assert_eq!(
        commun::compter_declinaisons(&bac, depose.asset.id).await,
        DECLINAISONS_ATTENDUES,
        "une reprise a doublé les déclinaisons"
    );

    // Seconde relance, plus dure : l'objet est laissé « en traitement », comme
    // après une mort survenue avant la mise en service. La sortie immédiate ne
    // joue plus, et c'est le relevé des déclinaisons déjà faites qui protège.
    sqlx::query!(
        "UPDATE media.assets SET status = 'processing' WHERE id = $1",
        depose.asset.id
    )
    .execute(bac.pool())
    .await
    .expect("retour en traitement");

    commun::worker_relance(&bac).await;
    let issues = commun::passer_le_worker(&bac).await;
    assert!(issues[0].is_ok(), "reprise : {:?}", issues[0]);
    assert_eq!(
        commun::compter_declinaisons(&bac, depose.asset.id).await,
        DECLINAISONS_ATTENDUES,
        "une reprise à mi-chemin a refabriqué des déclinaisons déjà écrites"
    );

    let objet = media::service::read::objet(&bac.state, depose.asset.id)
        .await
        .expect("lecture");
    assert_eq!(objet.status, "ready");
}

/// **Un document n'a ni dimension ni déclinaison — et devient servable.**
///
/// Décliner un PDF n'aurait aucun sens, et l'absence de dimensions n'est pas un
/// défaut : `tg_validate_attachment` ne contrôle la forme que des objets
/// mesurés, précisément pour cela.
#[tokio::test]
async fn un_document_devient_servable_sans_dimension_ni_declinaison() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let fichier = commun::document_pdf();

    let depose = commun::deposer(
        &bac,
        terrain.referente,
        &fichier,
        commun::metadonnees(&fichier),
    )
    .await
    .expect("dépôt");

    commun::passer_le_worker(&bac).await;

    let objet = media::service::read::objet(&bac.state, depose.asset.id)
        .await
        .expect("lecture");
    assert_eq!(objet.status, "ready");
    assert_eq!((objet.width, objet.height), (None, None));
    assert_eq!(objet.duration_seconds, None);
    assert_eq!(commun::compter_declinaisons(&bac, depose.asset.id).await, 0);

    let avancement = media::service::read::avancement(&bac.state, depose.asset.id)
        .await
        .expect("avancement");
    assert_eq!(avancement.renditions_expected, 0);
}

/// **Un média temporel a sa durée, et aucune déclinaison d'image.**
#[tokio::test]
async fn un_media_temporel_a_sa_duree_et_aucune_declinaison() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let fichier = commun::video_mp4(92.5);

    let depose = commun::deposer(
        &bac,
        terrain.referente,
        &fichier,
        commun::metadonnees(&fichier),
    )
    .await
    .expect("dépôt");

    commun::passer_le_worker(&bac).await;

    let objet = media::service::read::objet(&bac.state, depose.asset.id)
        .await
        .expect("lecture");
    assert_eq!(objet.status, "ready");
    assert_eq!((objet.width, objet.height), (None, None));
    // La durée traverse en texte : `numeric(10,3)` n'a pas d'équivalent flottant
    // sans perte, et un arrondi silencieux ferait diverger la valeur écrite de
    // la valeur relevée.
    assert_eq!(objet.duration_seconds.as_deref(), Some("92.500"));
    assert_eq!(commun::compter_declinaisons(&bac, depose.asset.id).await, 0);
}

/// Un moteur d'épreuve qui **trouve** quelque chose. Aucun moteur de la
/// configuration ne le peut : `none` ne trouve rien par construction, et
/// brancher `clamd` dans un test ferait dépendre la suite d'un démon externe.
struct MoteurQuiTrouve;

#[async_trait]
impl Scanner for MoteurQuiTrouve {
    async fn analyser(&self, _contenu: &[u8]) -> Verdict {
        Verdict::infecte("moteur-epreuve", "Eicar-Test-Signature")
    }

    fn engine(&self) -> &'static str {
        "moteur-epreuve"
    }
}

/// **Un objet en quarantaine n'est rendu par aucune lecture publique, et son
/// rattachement est refusé.**
///
/// Les deux gardes ne sont pas dans le code : `media.asset_sources()` et
/// `media.attached_image()` ne rendent que l'état servable, et
/// `tg_validate_attachment` refuse tout rattachement visant un objet en
/// quarantaine. Le test les éprouve **à la base**, pas sur une relecture.
#[tokio::test]
async fn un_objet_en_quarantaine_nest_servi_par_rien_et_ne_se_rattache_pas() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let fichier = commun::couverture_16_9();

    let depose = commun::deposer(
        &bac,
        terrain.referente,
        &fichier,
        commun::metadonnees_pour(
            &fichier,
            "org",
            "organizations",
            terrain.organisation,
            "logo",
        ),
    )
    .await
    .expect("dépôt");

    let gestionnaires: Vec<Arc<dyn JobHandler>> =
        vec![Arc::new(media::jobs::process::ProcessAsset::new(
            bac.db(),
            bac.state.storage().clone(),
            Arc::new(MoteurQuiTrouve),
        ))];
    let issues = commun::executer_les_travaux(&bac, &gestionnaires).await;
    assert!(issues[0].is_ok(), "la quarantaine n'est pas un échec");

    let objet = media::service::read::objet(&bac.state, depose.asset.id)
        .await
        .expect("lecture");
    assert_eq!(objet.status, "quarantined");
    assert_eq!(objet.scan_verdict, "infected");
    assert_eq!(objet.scan_engine.as_deref(), Some("moteur-epreuve"));
    // Aucune déclinaison : le traitement s'est arrêté avant.
    assert_eq!(commun::compter_declinaisons(&bac, depose.asset.id).await, 0);
    assert_eq!(objet.sources, serde_json::json!({}));

    let refus = sqlx::query!(
        "INSERT INTO media.attachments (owner_schema, owner_table, owner_id, asset_id, role)
         VALUES ('org', 'organizations', $1, $2, 'logo')",
        terrain.organisation,
        depose.asset.id
    )
    .execute(bac.pool())
    .await
    .expect_err("le rattachement d'un objet en quarantaine doit être refusé");
    assert!(
        refus.to_string().contains("quarantaine"),
        "le refus doit nommer la quarantaine : {refus}"
    );

    let image = sqlx::query_scalar!(
        r#"SELECT media.attached_image('org', 'organizations', $1, 'logo') AS image"#,
        terrain.organisation
    )
    .fetch_one(bac.pool())
    .await
    .expect("lecture publique");
    assert!(image.is_none(), "aucune lecture publique ne le rend");
}

/// **Le moteur `none` écrit « non pris en charge » et son nom, jamais « sain »**
/// (R13).
///
/// `ck_assets_scan_before_ready` accepte les deux, et l'objet devient bien
/// servable. Mais « sain » affirmerait qu'un moteur a inspecté le fichier :
/// une plateforme institutionnelle doit pouvoir **prouver** ce qui a été
/// inspecté, et la trace doit dire que personne ne l'a fait.
#[tokio::test]
async fn labsence_de_moteur_ecrit_non_pris_en_charge_et_son_nom() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let fichier = commun::vignette_1_1();

    let depose = commun::deposer(
        &bac,
        terrain.referente,
        &fichier,
        commun::metadonnees(&fichier),
    )
    .await
    .expect("dépôt");

    commun::passer_le_worker(&bac).await;

    let objet = media::service::read::objet(&bac.state, depose.asset.id)
        .await
        .expect("lecture");
    assert_eq!(objet.scan_verdict, "unsupported");
    assert_ne!(objet.scan_verdict, "clean");
    assert_eq!(objet.scan_engine.as_deref(), Some("none"));
    assert!(objet.scanned_at.is_some());
    assert_eq!(objet.status, "ready");

    // Une image de 800 px ne produit que la vignette : agrandir n'ajouterait
    // aucune information, et annoncer trois attendues laisserait l'avancement
    // bloqué à un tiers pour toujours.
    let avancement = media::service::read::avancement(&bac.state, depose.asset.id)
        .await
        .expect("avancement");
    assert_eq!(avancement.renditions_expected, 1);
    assert_eq!(avancement.renditions_ready, 1);
}

/// **L'espace des déclinaisons est compté dans la consommation de
/// l'organisation.**
///
/// Le compteur est tenu par `tg_renditions_storage_usage`, pas par le service —
/// mais rien ne le prouve tant qu'aucune déclinaison n'a été écrite. Sans ce
/// test, une organisation pourrait dépasser son quota du poids de toutes ses
/// déclinaisons.
#[tokio::test]
async fn lespace_des_declinaisons_compte_dans_la_consommation() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let fichier = commun::couverture_16_9();

    let depose = commun::deposer(
        &bac,
        terrain.referente,
        &fichier,
        commun::metadonnees_pour(
            &fichier,
            "org",
            "organizations",
            terrain.organisation,
            "logo",
        ),
    )
    .await
    .expect("dépôt");

    let apres_depot = commun::espace_consomme(&bac, terrain.organisation).await;
    assert_eq!(apres_depot, depose.asset.byte_size);

    commun::passer_le_worker(&bac).await;

    let poids_declinaisons = sqlx::query_scalar!(
        r#"SELECT media.rendition_bytes($1) AS "poids!""#,
        depose.asset.id
    )
    .fetch_one(bac.pool())
    .await
    .expect("poids des déclinaisons");

    assert!(
        poids_declinaisons > 0,
        "les déclinaisons pèsent quelque chose"
    );
    assert_eq!(
        commun::espace_consomme(&bac, terrain.organisation).await,
        apres_depot + poids_declinaisons
    );
}
