//! **Tout l'écran en une réponse, et en un instant.**
//!
//! Lues séparément par le site, les dix parties produiraient dix allers-retours
//! au chargement d'une page qu'on consulte vingt fois par jour, et **dix
//! instants de mesure différents dans un même écran**.

mod commun;

use commun::*;

#[tokio::test]
async fn la_reponse_porte_les_sept_parties() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    appel(&bac, decor.event_id, "cop31_appel", 40).await;
    rafraichir(&bac).await;

    let ecran = analytics::service::dashboard::composer(bac.pool(), decor.event_id)
        .await
        .expect("composition");

    assert_eq!(ecran.edition.id, decor.event_id);
    assert_eq!(ecran.timezone, FUSEAU, "le fuseau de l'ÉDITION");
    assert!(ecran.call.is_some(), "l'appel de l'édition");
    // `actions`, `figures`, `health` et `incidents` sont toujours présents,
    // fût-ce vides : un écran où tout va bien n'est pas un écran cassé.
    assert!(ecran.figures.kpis.len() == 6, "les six indicateurs de tête");
    assert!(
        !ecran.health.is_empty(),
        "la vue de santé porte toujours ses indicateurs"
    );
}
