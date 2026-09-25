//! **SC-003, research R5** — une lecture manquée compte et n'écrit aucune
//! session ; au seuil, `import_is_serving` coupe l'affichage, et une lecture
//! réussie le rétablit. Éteint, ou sans réussite récente, l'import ne sert rien.

mod importation;

use importation::Bac;

#[tokio::test]
async fn injoignable_jusqu_au_seuil_coupe_puis_une_reussite_retablit() {
    let bac = Bac::monter().await;
    bac.regler("missed_threshold = 2").await;
    bac.lire().await;
    assert!(bac.sert().await);
    let avant = bac.sessions().await;

    bac.jeu("cop30/injoignable").await;
    bac.lire().await;
    let premiere = bac.etat().await;
    assert_eq!(premiere.missed_reads, 1);
    assert!(premiere.failing_since.is_some());
    assert!(
        premiere
            .last_error
            .as_deref()
            .is_some_and(|e| e.starts_with("Source injoignable")),
        "{:?}",
        premiere.last_error
    );
    assert!(bac.sert().await, "une seule lecture manquée ne coupe pas");

    bac.lire().await;
    let seconde = bac.etat().await;
    assert_eq!(seconde.missed_reads, 2);
    assert_eq!(
        seconde.failing_since, premiere.failing_since,
        "depuis la première"
    );
    assert!(!bac.sert().await, "au seuil, l'affichage se coupe");
    assert_eq!(
        bac.sessions().await,
        avant,
        "une lecture manquée n'écrit aucune session"
    );
    let journal = bac.journal().await;
    assert_eq!(
        journal.iter().map(|l| l.0.as_str()).collect::<Vec<_>>(),
        ["failure", "failure", "success"]
    );
    assert_eq!(
        bac.suivantes().await.len(),
        1,
        "la chaîne continue pendant la panne"
    );

    bac.jeu("cop30/lecture-2").await;
    bac.lire().await;
    let retablie = bac.etat().await;
    assert_eq!((retablie.missed_reads, retablie.failing_since), (0, None));
    assert_eq!(retablie.last_error, None);
    assert!(bac.sert().await);
}

#[tokio::test]
async fn un_contenu_illisible_ou_sans_reunion_est_une_lecture_manquee() {
    let bac = Bac::monter().await;
    bac.lire().await;

    bac.jeu("cop30/illisible").await;
    bac.lire().await;
    let etat = bac.etat().await;
    assert_eq!(etat.missed_reads, 1);
    assert!(etat
        .last_error
        .as_deref()
        .is_some_and(|e| e.starts_with("Contenu de la source illisible")));

    bac.jeu("cop30/lecture-1").await;
    bac.executer(
        "UPDATE reference.taxonomy_terms SET is_active = false
          WHERE taxonomy_code = 'negotiation_meeting_type'",
    )
    .await;
    bac.lire().await;
    let etat = bac.etat().await;
    assert_eq!(
        etat.missed_reads, 2,
        "aucune réunion retenue : la source est en panne"
    );
    assert_eq!(bac.sessions().await, 44, "et rien n'a disparu");
    assert_eq!(
        bac.nombre(
            "SELECT count(*) FROM negotiation.meetings WHERE event_id = $1 AND absent_reads > 0"
        )
        .await,
        0
    );
}

#[tokio::test]
async fn eteint_l_import_ne_sert_rien_ne_lit_rien_et_ne_se_replanifie_pas() {
    let bac = Bac::monter().await;
    bac.lire().await;
    assert!(bac.sert().await);

    bac.regler("is_enabled = false").await;
    assert!(!bac.sert().await, "éteint, l'affichage se coupe aussitôt");
    bac.executer("DELETE FROM platform.jobs").await;
    bac.lire().await;
    assert_eq!(bac.journal().await.len(), 1, "aucune lecture");
    assert!(bac.suivantes().await.is_empty(), "aucune suivante");
}

#[tokio::test]
async fn une_derniere_reussite_trop_ancienne_coupe_meme_sans_lecture_manquee() {
    let bac = Bac::monter().await;
    bac.lire().await;
    bac.regler(
        "last_success_at = now() - make_interval(secs => missed_threshold * interval_seconds + 61)",
    )
    .await;
    assert_eq!(bac.etat().await.missed_reads, 0);
    assert!(
        !bac.sert().await,
        "un worker arrêté ne manque aucune lecture"
    );
}

#[tokio::test]
async fn une_lecture_manuelle_ne_replanifie_jamais() {
    let bac = Bac::monter().await;
    bac.lire_a_la_main().await;
    assert_eq!(bac.sessions().await, 44);
    assert!(bac.suivantes().await.is_empty());
    assert_eq!(
        bac.nombre(
            "SELECT count(*) FROM negotiation.import_runs r
               JOIN negotiation.official_imports i ON i.id = r.import_id
              WHERE i.event_id = $1 AND r.is_manual"
        )
        .await,
        1
    );
}

#[tokio::test]
async fn le_worker_rearme_une_chaine_morte_et_pas_une_vivante() {
    let bac = Bac::monter().await;
    let maintenant = time::OffsetDateTime::now_utc();
    let mut tx = bac
        .base
        .db()
        .write(&kernel::context::RequestContext::background("test"))
        .await
        .expect("transaction");
    assert_eq!(
        negotiation::jobs::import::armer(&mut tx, maintenant)
            .await
            .expect("armement"),
        1
    );
    assert_eq!(
        negotiation::jobs::import::armer(&mut tx, maintenant)
            .await
            .expect("armement"),
        0,
        "une chaîne vivante n'est pas doublée"
    );
    tx.commit().await.expect("validation");
    assert_eq!(bac.suivantes().await.len(), 1);
}
