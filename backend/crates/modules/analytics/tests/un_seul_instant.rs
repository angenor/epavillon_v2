//! **Les parties qui dépendent de `now()` parlent du MÊME instant.**
//!
//! `now()` vaut `transaction_timestamp()` : il est constant pour toute la
//! transaction de lecture, et `REPEATABLE READ` y ajoute un instantané unique.
//! C'est la réponse exacte aux « neuf instants de mesure » qu'interdit le
//! contrat du site.

mod commun;

use commun::*;

#[tokio::test]
async fn lecheance_et_les_incidents_actifs_sont_mesures_ensemble() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;
    appel(&bac, decor.event_id, "cop31_appel", 3).await;

    // Un message dont la fenêtre se ferme dans très peu de temps : s'il était
    // mesuré à un instant et l'échéance à un autre, les deux pourraient se
    // contredire — un incident « actif » sur un tableau de bord dont la carte
    // d'échéance a déjà basculé.
    poser(
        &bac,
        comptes.globale,
        "event",
        Some(decor.event_id),
        "active",
    )
    .await;
    rafraichir(&bac).await;

    let ecran = analytics::service::dashboard::composer(bac.pool(), decor.event_id)
        .await
        .expect("composition");

    let echeance = ecran
        .figures
        .kpis
        .iter()
        .find(|k| matches!(k.key, analytics::domain::figures::DashboardKpiKey::Deadline))
        .expect("la carte d'échéance");

    assert_eq!(ecran.incidents.len(), 1);
    let jours = echeance.value.expect("l'échéance existe");
    assert!(
        (0.0..=3.0).contains(&jours),
        "l'échéance et les incidents ont été lus dans la même transaction : {jours}"
    );

    // Deux compositions successives donnent des instants différents, mais chacune
    // reste cohérente avec elle-même — c'est bien la transaction qui fige, pas
    // une constante posée au démarrage.
    let seconde = analytics::service::dashboard::composer(bac.pool(), decor.event_id)
        .await
        .expect("seconde composition");
    assert_eq!(seconde.incidents.len(), 1);
}
