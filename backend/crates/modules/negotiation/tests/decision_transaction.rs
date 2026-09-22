//! **Admettre est une seule transaction** — FR-027, FR-028, principe IV.
//!
//! L'état de la demande, l'attribution du rôle, l'appartenance au réseau,
//! l'événement de domaine et le courriel mis en file naissent ensemble. Les
//! séparer ouvrirait la porte à un courriel annonçant un accès qui n'existe
//! pas — la faute la plus coûteuse de tout ce périmètre, parce qu'elle se
//! découvre chez la personne et non dans un journal.

mod commun;

use commun::{
    a_lacces, admettre, attribuer, courriels_en_file, creer_un_code, decor, demander, evenements,
    file_des_demandes, mon_acces, refuser, reseaux, saisir, Bac,
};
use negotiation::domain::access::{AccessState, RequestStatus};

#[tokio::test]
async fn admettre_ecrit_letat_lacces_levenement_et_le_courriel() {
    let bac = Bac::monter().await;
    let d = decor(&bac).await;
    attribuer(&bac, d.admin_id, "admin", "global", None).await;

    let demande = demander(
        &bac,
        d.person_id,
        Some(d.space_id),
        Some("délégation du Sénégal"),
    )
    .await
    .expect("demande");

    admettre(&bac, d.admin_id, demande.id, None)
        .await
        .expect("admission");

    assert!(a_lacces(&bac, d.person_id, Some(d.space_id)).await);

    let apres = mon_acces(&bac, d.person_id).await;
    assert_eq!(apres.state, AccessState::Granted);
    assert_eq!(
        apres.request.expect("la demande est rendue").status,
        RequestStatus::Approved
    );

    // L'événement part **de la base**, par `tg_access_request_event()`, dans
    // cette même transaction : l'émettre depuis le service le doublerait.
    let emis = evenements(&bac, demande.id).await;
    assert!(
        emis.contains(&"negotiation.access_request.submitted".to_owned()),
        "{emis:?}"
    );
    assert!(
        emis.contains(&"negotiation.access_request.approved".to_owned()),
        "{emis:?}"
    );

    let courriels = courriels_en_file(&bac).await;
    assert_eq!(courriels.len(), 1, "{courriels:?}");
    assert_eq!(courriels[0].0, "negotiation.access_request.approved_email");
    assert_eq!(courriels[0].1, "awa.diallo@example.org");
}

#[tokio::test]
async fn le_reseau_vient_du_code_que_la_demande_portait() {
    let bac = Bac::monter().await;
    let d = decor(&bac).await;
    attribuer(&bac, d.admin_id, "admin", "global", None).await;

    // Mode « les deux » : le code est reconnu, il ouvre une demande qui le
    // porte — et c'est ce code qui donnera l'appartenance à l'admission.
    commun::regler_le_mode(&bac, "code_and_approval").await;

    let code = creer_un_code(
        &bac,
        d.admin_id,
        "Réseau des négociatrices",
        Some(d.space_id),
        Some("women_negotiators"),
    )
    .await;

    let issue = saisir(&bac, d.person_id, &code.code, None).await;
    let demande = issue.request.expect("une demande s'est ouverte");

    assert!(reseaux(&bac, d.person_id).await.is_empty());

    admettre(&bac, d.admin_id, demande.id, None)
        .await
        .expect("admission");

    assert_eq!(
        reseaux(&bac, d.person_id).await,
        vec!["women_negotiators".to_owned()],
        "l'appartenance vient du code, et de rien d'autre (SC-006)"
    );

    let file = file_des_demandes(&bac).await;
    let ligne = file
        .rows
        .iter()
        .find(|r| r.id == demande.id)
        .expect("la file la porte");
    assert_eq!(ligne.invitation_code.as_deref(), Some(code.code.as_str()));
    assert_eq!(ligne.decided_by_name.as_deref(), Some("Awa Diallo"));
}

#[tokio::test]
async fn refuser_met_le_courriel_en_file_avec_son_motif() {
    let bac = Bac::monter().await;
    let d = decor(&bac).await;
    attribuer(&bac, d.admin_id, "admin", "global", None).await;

    let demande = demander(&bac, d.person_id, None, None)
        .await
        .expect("demande");

    refuser(&bac, d.admin_id, demande.id, Some("compte non vérifié"))
        .await
        .expect("refus");

    assert!(!a_lacces(&bac, d.person_id, None).await);

    let courriels = courriels_en_file(&bac).await;
    assert_eq!(courriels.len(), 1, "{courriels:?}");
    assert_eq!(courriels[0].0, "negotiation.access_request.rejected_email");

    let motif = sqlx::query_scalar!(
        r#"SELECT (payload ->> 'reason') AS "motif?"
             FROM platform.jobs
            WHERE task = 'negotiation.access_request.rejected_email'"#
    )
    .fetch_one(bac.pool())
    .await
    .expect("lecture de la charge utile");

    assert_eq!(
        motif.as_deref(),
        Some("compte non vérifié"),
        "le motif voyage avec le travail : le gestionnaire ne relit pas la base"
    );

    let apres = mon_acces(&bac, d.person_id).await;
    assert_eq!(apres.state, AccessState::Rejected);
    assert_eq!(
        apres
            .request
            .expect("la demande est rendue")
            .decision_reason
            .as_deref(),
        Some("compte non vérifié"),
        "« Mon accès » lit le motif, même si le courriel s'est perdu"
    );
}

#[tokio::test]
async fn une_demande_annulee_nenvoie_aucun_courriel() {
    let bac = Bac::monter().await;
    let d = decor(&bac).await;

    let demande = demander(&bac, d.person_id, Some(d.space_id), None)
        .await
        .expect("demande");
    commun::annuler_sa_demande(&bac, d.person_id, demande.id)
        .await
        .expect("annulation");

    assert!(
        courriels_en_file(&bac).await.is_empty(),
        "personne n'a rien à recevoir pour une demande que son auteur vient de refermer"
    );

    let emis = evenements(&bac, demande.id).await;
    assert_eq!(
        emis,
        vec!["negotiation.access_request.submitted".to_owned()],
        "le trigger n'émet que pour approved et rejected : {emis:?}"
    );
}
