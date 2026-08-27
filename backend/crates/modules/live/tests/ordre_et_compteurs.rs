//! **L'ordre est celui de la fonction, et les compteurs précèdent tout
//! filtrage.**
//!
//! `live.event_incidents()` rend les actifs d'abord, puis ce qui va parler, puis
//! ce qui attend une décision, puis l'historique — l'ordre dans lequel l'équipe
//! agit. L'API ne réordonne pas : le faire produirait un second ordre, qui
//! divergerait du premier au premier ajustement du SQL.

mod commun;

use commun::*;

#[tokio::test]
async fn lordre_rendu_est_celui_de_la_fonction() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;

    // Posés dans le désordre, exprès.
    poser(
        &bac,
        comptes.globale,
        "event",
        Some(decor.event_id),
        "unpublished",
    )
    .await;
    poser(
        &bac,
        comptes.globale,
        "event",
        Some(decor.event_id),
        "draft",
    )
    .await;
    poser(
        &bac,
        comptes.globale,
        "event",
        Some(decor.event_id),
        "active",
    )
    .await;
    poser(
        &bac,
        comptes.globale,
        "event",
        Some(decor.event_id),
        "scheduled",
    )
    .await;

    let ecran = live::service::list::composer(bac.pool(), decor.event_id, "fr")
        .await
        .expect("composition de l'écran");

    let etats: Vec<&str> = ecran.rows.iter().map(|r| r.state.as_str()).collect();
    assert_eq!(
        etats,
        vec!["active", "scheduled", "draft", "unpublished"],
        "actifs, puis programmés, puis brouillons, puis historique"
    );
}

#[tokio::test]
async fn les_compteurs_portent_les_cinq_etats_et_precedent_tout_filtrage() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;

    poser(
        &bac,
        comptes.globale,
        "event",
        Some(decor.event_id),
        "active",
    )
    .await;
    poser(
        &bac,
        comptes.globale,
        "event",
        Some(decor.event_id),
        "active",
    )
    .await;
    poser(
        &bac,
        comptes.globale,
        "event",
        Some(decor.event_id),
        "draft",
    )
    .await;

    let ecran = live::service::list::composer(bac.pool(), decor.event_id, "fr")
        .await
        .expect("composition de l'écran");

    assert_eq!(
        ecran.counts.len(),
        5,
        "les cinq états sont toujours présents"
    );
    assert_eq!(ecran.counts.get("active"), Some(&2));
    assert_eq!(ecran.counts.get("draft"), Some(&1));
    // **Un état sans ligne vaut zéro, jamais absent** : l'écran attend un
    // décompte, et un état manquant ferait afficher un tiret.
    assert_eq!(ecran.counts.get("expired"), Some(&0));
    assert_eq!(ecran.counts.get("scheduled"), Some(&0));
    assert_eq!(ecran.counts.get("unpublished"), Some(&0));

    let total: i64 = ecran.counts.values().sum();
    assert_eq!(
        total,
        ecran.rows.len() as i64,
        "les compteurs comptent exactement les lignes rendues"
    );
}
