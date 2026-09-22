//! **Une demande en attente survit à un changement de mode** — FR-029.
//!
//! Une personne qui a demandé l'accès en mode « approbation » ne doit pas voir
//! sa demande s'évaporer parce que l'IFDD rouvre l'entrée par code le
//! lendemain : elle attend une réponse, et elle doit l'obtenir. La bascule est
//! un réglage d'entrée, pas une purge.

mod commun;

use commun::{
    a_lacces, admettre, attribuer, basculer_le_mode, decor, demander, etat_de_la_demande,
    file_des_demandes, Bac,
};

#[tokio::test]
async fn la_demande_reste_en_attente_et_traitable_apres_la_bascule() {
    let bac = Bac::monter().await;
    let d = decor(&bac).await;
    attribuer(&bac, d.admin_id, "admin", "global", None).await;

    basculer_le_mode(&bac, d.admin_id, "approval")
        .await
        .expect("bascule vers l'approbation");

    let demande = demander(
        &bac,
        d.person_id,
        Some(d.space_id),
        Some("atelier de Dakar"),
    )
    .await
    .expect("demande");

    // On revient au code seul : la demande ne disparaît pas.
    basculer_le_mode(&bac, d.admin_id, "code")
        .await
        .expect("retour au code seul");

    assert_eq!(etat_de_la_demande(&bac, demande.id).await, "pending");

    let file = file_des_demandes(&bac).await;
    assert_eq!(file.pending, 1);
    assert!(file.rows.iter().any(|r| r.id == demande.id));

    admettre(&bac, d.admin_id, demande.id, None)
        .await
        .expect("elle reste traitable dans le mode nouveau");

    assert!(a_lacces(&bac, d.person_id, Some(d.space_id)).await);
}
