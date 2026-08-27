//! **Les cinq portées remontent, et c'est le test qui tient tout l'écran.**
//!
//! `live.incidents` n'a aucune colonne d'édition pour `session`, `event_day` et
//! `organization` : un filtre écrit à la main — un `WHERE i.event_id = $1` —
//! laisserait fuir trois portées sur cinq. L'écran paraîtrait juste ; il
//! montrerait simplement moins que ce qui existe.

mod commun;

use commun::*;

#[tokio::test]
async fn les_cinq_portees_remontent() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;

    poser(&bac, comptes.globale, "global", None, "active").await;
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
        "event_day",
        Some(decor.jour_id),
        "active",
    )
    .await;
    poser(
        &bac,
        comptes.globale,
        "session",
        Some(decor.session_id),
        "active",
    )
    .await;
    // La portée `organization` n'entre QUE parce que l'organisation anime une
    // activité de l'édition — même critère que le modèle.
    poser(
        &bac,
        comptes.globale,
        "organization",
        Some(decor.organization_id),
        "active",
    )
    .await;
    // Un sixième, dont la fenêtre est close : il doit remonter aussi, en
    // `expired`. La liste du back-office porte les cinq états.
    poser(
        &bac,
        comptes.globale,
        "event",
        Some(decor.event_id),
        "expired",
    )
    .await;

    let ecran = live::service::list::composer(bac.pool(), decor.event_id, "fr")
        .await
        .expect("composition de l'écran");

    assert_eq!(ecran.rows.len(), 6, "six messages posés, six rendus");

    let mut portees: Vec<&str> = ecran.rows.iter().map(|r| r.scope.as_str()).collect();
    portees.sort_unstable();
    portees.dedup();
    assert_eq!(
        portees,
        vec!["event", "event_day", "global", "organization", "session"],
        "les cinq portées, aucune perdue"
    );
}

#[tokio::test]
async fn une_organisation_qui_nanime_rien_nentre_pas() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;

    poser(
        &bac,
        comptes.globale,
        "organization",
        Some(decor.organisation_etrangere),
        "active",
    )
    .await;

    let ecran = live::service::list::composer(bac.pool(), decor.event_id, "fr")
        .await
        .expect("composition de l'écran");

    assert!(
        ecran.rows.is_empty(),
        "une ONG qui n'anime aucune activité de l'édition n'y publie pas de bandeau"
    );
}
