//! **Le travail d'extraction, de bout en bout** : l'objet est lu dans le bucket
//! privé, la forme lisible va en base page par page, et les images de page
//! retournent dans le bucket privé. Les pages, données dérivées, ne sont pas
//! auditées ; l'état d'extraction l'est, puisque le choix « tel quel » y vit :
//! le travail écrit au nom de la personne qui a attaché le fichier.

mod commun;
mod fabrique;

use commun::documents::{
    brouillon, demander_lextraction, entrepots, extracteur, objet_pdf, pages, passer_lextraction,
    rendu, BUCKET_PRIVE, PETIT,
};
use commun::{personne, Bac};
use kernel::jobs::{self, JobHandler};
use kernel::storage::StorageError;
use negotiation::repo::renditions;
use uuid::Uuid;

/// Un seul travail, le plus ancien : de quoi intercaler une relance.
async fn un_seul_passage(bac: &Bac) -> kernel::error::Result<()> {
    let gestionnaire = extracteur(bac);
    let mut tx = bac
        .db()
        .write(&bac.ctx_anonyme())
        .await
        .expect("transaction");
    let travail = jobs::claim(&mut tx, gestionnaire.queue(), "test-worker", 1)
        .await
        .expect("réservation")
        .pop()
        .expect("un travail en file");
    tx.commit().await.expect("validation");
    let issue = gestionnaire.run(&travail).await;
    let mut tx = bac
        .db()
        .write(&bac.ctx_anonyme())
        .await
        .expect("transaction");
    match &issue {
        Ok(()) => jobs::succeed(&mut tx, travail.id).await.expect("succès"),
        Err(e) => jobs::fail(&mut tx, travail.id, &e.to_string())
            .await
            .expect("échec"),
    }
    tx.commit().await.expect("validation");
    issue
}

async fn image_existe(bac: &Bac, cle: &str) -> bool {
    match entrepots(bac).du_bucket(BUCKET_PRIVE).head(cle).await {
        Ok(_) => true,
        Err(StorageError::NotFound(_)) => false,
        Err(e) => panic!("stockage : {e}"),
    }
}

#[tokio::test]
async fn un_pdf_pret_devient_des_pages_et_des_images() {
    let bac = Bac::monter().await;
    let ifdd = personne(&bac, "ifdd@example.org").await;
    let document = brouillon(&bac, "petit-guide").await;
    let asset = objet_pdf(&bac, ifdd, PETIT, "ready").await;

    demander_lextraction(&bac, document, asset, ifdd).await;
    let issues = passer_lextraction(&bac).await;
    assert!(issues.iter().all(Result::is_ok), "{issues:?}");

    let etat = rendu(&bac, document).await;
    assert_eq!(etat.status, "ready");
    assert_eq!(etat.page_count, Some(4));
    assert_eq!(etat.is_reflowable, Some(true));
    assert!(etat.reading_bytes.is_some_and(|o| o > 0));

    let pages = pages(&bac, document).await;
    assert_eq!(pages.iter().map(|p| p.0).collect::<Vec<_>>(), [1, 2, 3, 4]);
    assert_eq!(
        pages.iter().map(|p| p.1.as_str()).collect::<Vec<_>>(),
        ["1", "2", "3", "4"]
    );
    assert_eq!(
        pages.iter().map(|p| p.3).collect::<Vec<_>>(),
        [false, false, true, false]
    );
    let stockage = entrepots(&bac).du_bucket(BUCKET_PRIVE);
    for (index, _, cle, _) in &pages {
        let cle = cle.as_deref().expect("clé d'image");
        assert!(
            cle.starts_with(&format!("documents/{document}/{asset}/")),
            "{cle}"
        );
        let image = stockage.get(cle).await.expect("image dans le bucket privé");
        assert!(image.starts_with(&[0xFF, 0xD8]), "page {index}");
    }
}

#[tokio::test]
async fn un_objet_pas_encore_analyse_fait_attendre_sans_echouer() {
    let bac = Bac::monter().await;
    let ifdd = personne(&bac, "ifdd@example.org").await;
    let document = brouillon(&bac, "en-analyse").await;
    let asset = objet_pdf(&bac, ifdd, PETIT, "scanning").await;

    demander_lextraction(&bac, document, asset, ifdd).await;
    let issues = passer_lextraction(&bac).await;
    assert_eq!(issues.len(), 1);
    assert!(issues[0].is_ok());
    assert_eq!(rendu(&bac, document).await.status, "pending");

    let attente = sqlx::query!(
        r#"SELECT payload->>'attente' AS "attente", run_at > now() AS "plus_tard!"
             FROM platform.jobs
            WHERE task = 'negotiation.document.extract' AND status = 'queued'"#
    )
    .fetch_one(bac.pool())
    .await
    .expect("le travail repasse plus tard");
    assert_eq!(attente.attente.as_deref(), Some("1"));
    assert!(attente.plus_tard);
}

#[tokio::test]
async fn un_objet_refuse_par_lanalyse_conclut_en_echec_avec_son_motif() {
    let bac = Bac::monter().await;
    let ifdd = personne(&bac, "ifdd@example.org").await;
    let document = brouillon(&bac, "quarantaine").await;
    let asset = objet_pdf(&bac, ifdd, PETIT, "quarantined").await;

    demander_lextraction(&bac, document, asset, ifdd).await;
    passer_lextraction(&bac).await;

    let etat = rendu(&bac, document).await;
    assert_eq!(etat.status, "failed");
    assert!(etat.failure_reason.is_some_and(|m| m.contains("antivirus")));
}

#[tokio::test]
async fn un_fichier_qui_ne_souvre_pas_conclut_en_echec() {
    let bac = Bac::monter().await;
    let ifdd = personne(&bac, "ifdd@example.org").await;
    let document = brouillon(&bac, "illisible").await;
    let asset = objet_pdf(&bac, ifdd, b"%PDF-1.4 tronque", "ready").await;

    demander_lextraction(&bac, document, asset, ifdd).await;
    passer_lextraction(&bac).await;

    let etat = rendu(&bac, document).await;
    assert_eq!(etat.status, "failed");
    assert!(
        etat.failure_reason.is_some_and(|m| m.contains("illisible")),
        "motif lisible"
    );
    assert!(pages(&bac, document).await.is_empty());
}

/// Le document a changé de fichier entre la mise en file et le passage : le
/// travail du premier fichier ne doit rien écrire.
#[tokio::test]
async fn le_travail_dun_fichier_remplace_nécrit_rien() {
    let bac = Bac::monter().await;
    let ifdd = personne(&bac, "ifdd@example.org").await;
    let document = brouillon(&bac, "remplace").await;
    let ancien = objet_pdf(&bac, ifdd, PETIT, "ready").await;
    let nouveau = objet_pdf(&bac, ifdd, PETIT, "scanning").await;

    demander_lextraction(&bac, document, ancien, ifdd).await;
    demander_lextraction(&bac, document, nouveau, ifdd).await;
    passer_lextraction(&bac).await;

    assert!(
        pages(&bac, document).await.is_empty(),
        "l'ancien fichier n'a rien écrit"
    );
    assert_eq!(rendu(&bac, document).await.status, "pending");
}

#[tokio::test]
async fn un_pdf_sans_page_echoue_des_le_premier_essai() {
    let bac = Bac::monter().await;
    let ifdd = personne(&bac, "ifdd@example.org").await;
    let document = brouillon(&bac, "vide").await;
    let asset = objet_pdf(&bac, ifdd, &fabrique::pages(0, 200, 200), "ready").await;

    demander_lextraction(&bac, document, asset, ifdd).await;
    let issues = passer_lextraction(&bac).await;
    assert_eq!(issues.len(), 1, "un seul passage, sans épuiser les essais");

    let etat = rendu(&bac, document).await;
    assert_eq!(etat.status, "failed");
    assert!(
        etat.failure_reason
            .as_deref()
            .is_some_and(|m| m.contains("illisible") && m.contains("aucune page")),
        "{:?}",
        etat.failure_reason
    );
}

/// Un fichier qui a fait tomber le worker à chaque essai revient une dernière
/// fois, repris par l'expiration de son verrou : il conclut en échec, et le
/// travail meurt au lieu de repartir sans fin.
#[tokio::test]
async fn un_travail_repris_apres_la_chute_du_worker_conclut_en_echec() {
    let bac = Bac::monter().await;
    let ifdd = personne(&bac, "ifdd@example.org").await;
    let document = brouillon(&bac, "fait-tomber").await;
    let asset = objet_pdf(&bac, ifdd, PETIT, "ready").await;

    demander_lextraction(&bac, document, asset, ifdd).await;
    // L'essai tombé avait déposé ses deux premières images.
    let stockage = entrepots(&bac).du_bucket(BUCKET_PRIVE);
    let deposees: Vec<String> = (1..=2)
        .map(|n| format!("documents/{document}/{asset}/{n}.jpg"))
        .collect();
    for cle in &deposees {
        stockage
            .put(cle, "image/jpeg", vec![0xFF, 0xD8, 0xFF])
            .await
            .expect("image déposée");
    }
    sqlx::query!(
        "UPDATE platform.jobs SET attempts = max_attempts
          WHERE task = 'negotiation.document.extract'"
    )
    .execute(bac.pool())
    .await
    .expect("essais épuisés");
    let issues = passer_lextraction(&bac).await;
    assert_eq!(issues.len(), 1);
    assert!(issues[0].is_err());

    let etat = rendu(&bac, document).await;
    assert_eq!(etat.status, "failed");
    assert!(etat
        .failure_reason
        .is_some_and(|m| m.contains("interrompu l'extraction")));
    assert!(pages(&bac, document).await.is_empty(), "rien n'a été lu");
    for cle in &deposees {
        assert!(!image_existe(&bac, cle).await, "{cle} effacée");
    }
    let statut = sqlx::query_scalar!(
        r#"SELECT status::text AS "status!" FROM platform.jobs
            WHERE task = 'negotiation.document.extract'"#
    )
    .fetch_one(bac.pool())
    .await
    .expect("travail");
    assert_eq!(statut, "dead");
}

/// Une relance pendant qu'un travail attend : le travail de la première
/// demande ne conclut plus rien, et ne laisse pas publier avant la relance.
#[tokio::test]
async fn une_relance_rend_caduc_le_travail_de_la_demande_precedente() {
    let bac = Bac::monter().await;
    let ifdd = personne(&bac, "ifdd@example.org").await;
    let document = brouillon(&bac, "relance").await;
    let asset = objet_pdf(&bac, ifdd, PETIT, "ready").await;

    demander_lextraction(&bac, document, asset, ifdd).await;
    demander_lextraction(&bac, document, asset, ifdd).await;
    un_seul_passage(&bac)
        .await
        .expect("travail caduc, sans erreur");
    assert_eq!(rendu(&bac, document).await.status, "pending");
    assert!(pages(&bac, document).await.is_empty());

    let issues = passer_lextraction(&bac).await;
    assert!(issues.iter().all(Result::is_ok), "{issues:?}");
    assert_eq!(rendu(&bac, document).await.status, "ready");
}

/// Le même garde, à chaque écriture : un travail commencé avant la relance
/// ne pose ni « prête » ni « échec » par-dessus elle.
#[tokio::test]
async fn un_travail_commence_avant_la_relance_ne_conclut_pas() {
    let bac = Bac::monter().await;
    let ifdd = personne(&bac, "ifdd@example.org").await;
    let document = brouillon(&bac, "commence").await;
    let asset = objet_pdf(&bac, ifdd, PETIT, "ready").await;
    let (premiere, relance) = (Uuid::now_v7(), Uuid::now_v7());

    let mut tx = bac.db().write(&bac.ctx(ifdd)).await.expect("transaction");
    renditions::demander(&mut tx, document, asset, premiere)
        .await
        .expect("demande");
    assert!(renditions::commencer(&mut tx, document, asset, premiere)
        .await
        .expect("commencée"));
    renditions::demander(&mut tx, document, asset, relance)
        .await
        .expect("relance");
    let verdict = renditions::Verdict {
        page_count: 4,
        outline: &serde_json::json!([]),
        is_reflowable: true,
        quality: &serde_json::json!({}),
        reading_bytes: 1,
        extractor: "essai",
    };
    assert!(
        !renditions::reussir(&mut tx, document, asset, premiere, &verdict)
            .await
            .expect("réussite refusée")
    );
    renditions::echouer(&mut tx, document, asset, premiere, "tombé")
        .await
        .expect("échec ignoré");
    tx.commit().await.expect("validation");
    assert_eq!(rendu(&bac, document).await.status, "pending");
}

/// Une erreur après le dépôt des images — ici la base refuse les pages — ne
/// laisse aucune image que rien ne désigne.
#[tokio::test]
async fn une_erreur_apres_le_depot_efface_les_images() {
    let bac = Bac::monter().await;
    let ifdd = personne(&bac, "ifdd@example.org").await;
    let document = brouillon(&bac, "refuse").await;
    let asset = objet_pdf(&bac, ifdd, PETIT, "ready").await;
    sqlx::raw_sql(
        "CREATE FUNCTION public.refuser_les_pages() RETURNS trigger LANGUAGE plpgsql
            AS $$ BEGIN RAISE EXCEPTION 'pages refusées pour l''essai'; END $$;
         CREATE TRIGGER tg_refus BEFORE INSERT ON negotiation.document_pages
            FOR EACH ROW EXECUTE FUNCTION public.refuser_les_pages();",
    )
    .execute(bac.pool())
    .await
    .expect("refus posé");

    demander_lextraction(&bac, document, asset, ifdd).await;
    let issue = un_seul_passage(&bac).await;
    assert!(issue.is_err());
    for n in 1..=4 {
        let cle = format!("documents/{document}/{asset}/{n}.jpg");
        assert!(!image_existe(&bac, &cle).await, "{cle} effacée");
    }
}

/// Le vrai guide de la CdP30, s'il est posé dans `.essais/` (hors Git) : le
/// travail entier — lecture, règles, 90 images déposées — tient sous la minute.
/// `cargo test -p negotiation --test extraction_travail -- --ignored --nocapture`
#[tokio::test]
#[ignore = "demande .essais/guide-cdp30.pdf, hors du dépôt"]
async fn le_vrai_guide_sextrait_en_moins_dune_minute() {
    let chemin = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../.essais/guide-cdp30.pdf"
    );
    let Ok(octets) = std::fs::read(chemin) else {
        eprintln!("guide absent : {chemin}");
        return;
    };
    let bac = Bac::monter().await;
    let ifdd = personne(&bac, "ifdd@example.org").await;
    let document = brouillon(&bac, "guide-cdp30").await;
    let asset = objet_pdf(&bac, ifdd, &octets, "ready").await;
    demander_lextraction(&bac, document, asset, ifdd).await;

    let debut = std::time::Instant::now();
    let issues = passer_lextraction(&bac).await;
    let duree = debut.elapsed();
    assert!(issues.iter().all(Result::is_ok), "{issues:?}");

    let etat = rendu(&bac, document).await;
    eprintln!(
        "guide : {} pages, copie gardée {} octets, en {:.1} s",
        etat.page_count.unwrap_or(0),
        etat.reading_bytes.unwrap_or(0),
        duree.as_secs_f32()
    );
    assert_eq!(etat.page_count, Some(90));
    assert!(duree.as_secs() < 60, "{duree:?}");
}
