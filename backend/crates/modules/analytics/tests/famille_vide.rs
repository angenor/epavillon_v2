//! **Une famille sans élément n'émet aucune ligne.**
//!
//! Une édition où tout va bien rend `actions: []`, et l'écran doit rester
//! lisible ainsi : un back-office calme ne doit pas ressembler à un écran cassé.
//! Une ligne à zéro serait pire qu'inutile — elle ferait chercher ce qui n'existe
//! pas.

mod commun;

use commun::*;

#[tokio::test]
async fn une_edition_ou_tout_va_bien_rend_aucune_action() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    rafraichir(&bac).await;

    let ecran = analytics::service::dashboard::composer(bac.pool(), decor.event_id)
        .await
        .expect("composition");

    assert!(
        ecran.actions.is_empty(),
        "aucune famille ne s'allume : {:?}",
        ecran.actions.iter().map(|a| a.kind).collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn un_message_actif_allume_sa_famille_et_elle_seule() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;

    poser(
        &bac,
        comptes.globale,
        "session",
        Some(decor.session_id),
        "active",
    )
    .await;
    rafraichir(&bac).await;

    let ecran = analytics::service::dashboard::composer(bac.pool(), decor.event_id)
        .await
        .expect("composition");

    assert_eq!(ecran.actions.len(), 1);
    let ligne = &ecran.actions[0];
    assert_eq!(
        ligne.kind,
        analytics::domain::action::AdminActionKind::ActiveIncidents
    );
    assert_eq!(ligne.count, 1);
    assert_eq!(
        ligne.severity,
        analytics::domain::action::AdminActionSeverity::High,
        "un message actif est VU DU PUBLIC"
    );
    assert_eq!(ligne.target, "/admin/incidents");
    assert_eq!(
        ligne.examples[0].label, "Atelier de négociation",
        "l'exemple NOMME son sujet"
    );
}

#[tokio::test]
async fn un_brouillon_nallume_rien() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;

    poser(
        &bac,
        comptes.globale,
        "session",
        Some(decor.session_id),
        "draft",
    )
    .await;
    rafraichir(&bac).await;

    let ecran = analytics::service::dashboard::composer(bac.pool(), decor.event_id)
        .await
        .expect("composition");

    assert!(ecran.actions.is_empty(), "rien n'est encore dit à personne");
}
