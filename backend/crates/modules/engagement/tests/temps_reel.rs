//! **Un envoi annonce ses changements à l'écran (31/10).**
//!
//! Écouté sur le canal de `platform.signal()` comme l'API l'écoute : chaque
//! lot avance la fiche et la liste ; la personne qui a lancé l'envoi n'est
//! prévenue qu'à la fin ou au problème, une fois.

mod commun;

use commun::Bac;
use engagement::domain::mailing::{CreateMailingPayload, MailingPayload};
use engagement::jobs::send_mailing::SEND_MAILING;
use engagement::service::mailings;
use serde_json::Value;
use sqlx::postgres::PgListener;
use std::time::Duration;
use uuid::Uuid;

async fn ecouter(bac: &Bac) -> PgListener {
    let mut ecoute = PgListener::connect(bac.base.url()).await.expect("écoute");
    ecoute
        .listen(kernel::realtime::SIGNAL_CHANNEL)
        .await
        .expect("LISTEN");
    ecoute
}

/// Les signaux reçus jusqu'à une accalmie : `(sujet, charge)`.
async fn recus(ecoute: &mut PgListener) -> Vec<(String, Value)> {
    let mut vus = Vec::new();
    while let Ok(Ok(n)) = tokio::time::timeout(Duration::from_millis(400), ecoute.recv()).await {
        let lu: Value = serde_json::from_str(n.payload()).expect("signal JSON");
        vus.push((
            lu["topic"].as_str().unwrap_or_default().to_owned(),
            lu["payload"].clone(),
        ));
    }
    vus
}

fn compte(vus: &[(String, Value)], sujet: &str) -> usize {
    vus.iter().filter(|(t, _)| t == sujet).count()
}

async fn envoi_parti(bac: &Bac, edition: Uuid, auteur: Uuid, adresses: usize) -> Uuid {
    let envoi = mailings::creer(
        &bac.state,
        &bac.ctx(),
        auteur,
        &CreateMailingPayload {
            event_id: edition,
            draft: MailingPayload {
                subject: Some("Consignes".into()),
                body_html: Some("<p>Bonjour,</p>".into()),
                audience: Some(vec![]),
                extra_addresses: Some(
                    (0..adresses)
                        .map(|i| format!("signal{i}@example.org"))
                        .collect(),
                ),
                cc: Some(vec![]),
                ..MailingPayload::default()
            },
        },
    )
    .await
    .expect("brouillon");
    mailings::envoyer(&bac.state, &bac.ctx(), auteur, envoi.id)
        .await
        .expect("envoi");
    envoi.id
}

#[tokio::test]
async fn chaque_lot_avance_lecran_et_la_fin_previent_lauteur() {
    let bac = Bac::monter().await;
    let t = commun::terrain(&bac).await;
    let mut ecoute = ecouter(&bac).await;

    let id = envoi_parti(&bac, t.edition, t.administratrice, 15).await;
    let fiche = format!("mailing:{id}");
    let liste = format!("event:{}:mailings", t.edition);
    let auteur = format!("person:{}", t.administratrice);

    let au_depart = recus(&mut ecoute).await;
    assert_eq!(
        compte(&au_depart, &fiche),
        1,
        "mise en file : {au_depart:?}"
    );
    assert_eq!(compte(&au_depart, &liste), 1);
    assert_eq!(compte(&au_depart, &auteur), 0, "le départ se voit au clic");

    for issue in commun::passer_le_worker(&bac).await {
        issue.expect("passage du worker");
    }
    let envoi = recus(&mut ecoute).await;
    // Quinze adresses, lots de dix : deux passages, deux signaux.
    assert_eq!(compte(&envoi, &fiche), 2, "{envoi:?}");
    assert_eq!(compte(&envoi, &liste), 2);
    assert_eq!(compte(&envoi, &auteur), 1, "l'auteur, une fois, à la fin");
    let charge = &envoi.iter().find(|(s, _)| s == &auteur).expect("auteur").1;
    assert_eq!(charge["kind"], "mailing");

    // Supprimé : la liste et la fiche le savent, l'auteur n'est pas dérangé.
    mailings::supprimer(&bac.state, &bac.ctx(), t.administratrice, id)
        .await
        .expect("suppression");
    let apres = recus(&mut ecoute).await;
    assert_eq!(compte(&apres, &fiche), 1);
    assert_eq!(compte(&apres, &liste), 1);
    assert_eq!(compte(&apres, &auteur), 0);
}

#[tokio::test]
async fn lechec_previent_lauteur_une_fois_et_la_relance_se_voit() {
    let bac = Bac::monter().await;
    let t = commun::terrain(&bac).await;
    let id = envoi_parti(&bac, t.edition, t.administratrice, 15).await;
    let mut ecoute = ecouter(&bac).await;
    let fiche = format!("mailing:{id}");
    let auteur = format!("person:{}", t.administratrice);

    bac.boite.tomber_apres(Some(4));
    for _ in 0..10 {
        let _ = commun::passer_le_worker(&bac).await;
        let en_attente = sqlx::query(
            "UPDATE platform.jobs SET run_at = now() WHERE task = $1 AND status = 'queued'",
        )
        .bind(SEND_MAILING)
        .execute(bac.pool())
        .await
        .expect("reprises avancées")
        .rows_affected();
        if en_attente == 0 {
            break;
        }
    }
    let panne = recus(&mut ecoute).await;
    // Chaque essai interrompu se relit ; l'auteur n'apprend que l'échec.
    assert!(compte(&panne, &fiche) >= 5, "{panne:?}");
    assert_eq!(compte(&panne, &auteur), 1, "{panne:?}");
    let detail = mailings::detail(&bac.state, t.administratrice, id)
        .await
        .expect("détail");
    assert_eq!(detail.status, "failed");

    bac.boite.tomber_apres(None);
    mailings::relancer(&bac.state, &bac.ctx(), t.administratrice, id)
        .await
        .expect("relance");
    let relance = recus(&mut ecoute).await;
    assert_eq!(compte(&relance, &fiche), 1);
    assert_eq!(compte(&relance, &auteur), 0);

    mailings::annuler(&bac.state, &bac.ctx(), t.administratrice, id)
        .await
        .expect("arrêt");
    let arret = recus(&mut ecoute).await;
    assert_eq!(compte(&arret, &fiche), 1);
    assert_eq!(compte(&arret, &auteur), 0);
}

#[tokio::test]
async fn le_filet_du_releve_previent_lauteur() {
    let bac = Bac::monter().await;
    let t = commun::terrain(&bac).await;
    let id = envoi_parti(&bac, t.edition, t.administratrice, 3).await;
    sqlx::query(
        "UPDATE platform.jobs SET status = 'dead', attempts = max_attempts,
                last_error = 'serveur d''envoi injoignable' WHERE task = $1",
    )
    .bind(SEND_MAILING)
    .execute(bac.pool())
    .await
    .expect("travail mort");
    let mut ecoute = ecouter(&bac).await;

    mailings::ecran(&bac.state, t.administratrice, t.edition)
        .await
        .expect("historique");
    let vus = recus(&mut ecoute).await;
    assert_eq!(compte(&vus, &format!("mailing:{id}")), 1, "{vus:?}");
    assert_eq!(compte(&vus, &format!("person:{}", t.administratrice)), 1);
}
