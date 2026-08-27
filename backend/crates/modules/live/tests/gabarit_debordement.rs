//! **Le gabarit du raccourci « Signaler un débordement ».**
//!
//! Il existe pour éviter une saisie pendant que la salle attend : l'activité,
//! son titre **résolu**, son créneau et son édition. Le titre est ici résolu et
//! non brut, à la différence du reste de l'écran — c'est une valeur de
//! pré-remplissage de champ, que le site pose telle quelle.

mod commun;

use commun::*;
use kernel::error::ErrorCode;
use time::macros::time;

#[tokio::test]
async fn le_gabarit_porte_lactivite_son_titre_resolu_et_son_creneau() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;
    let perimetre = perimetre_de(&bac, comptes.detache).await;

    let gabarit =
        live::service::list::gabarit_de_debordement(&bac.state, &perimetre, decor.session_id, "fr")
            .await
            .expect("le gabarit s'ouvre");

    assert_eq!(gabarit.session_id, decor.session_id);
    assert_eq!(gabarit.event_id, decor.event_id);
    assert_eq!(
        gabarit.title, "Atelier de négociation",
        "résolu, pas un document multilingue"
    );
    assert!(gabarit.ends_at > gabarit.starts_at);
}

#[tokio::test]
async fn une_activite_hors_perimetre_rend_404() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;
    let perimetre = perimetre_de(&bac, comptes.detache).await;

    let ailleurs = activite(
        &bac,
        decor.autre_event_id,
        None,
        None,
        "Séance d'ailleurs",
        "seance-ailleurs",
        aujourdhui_a(&bac, decor.autre_event_id, time!(09:00)).await,
        aujourdhui_a(&bac, decor.autre_event_id, time!(10:00)).await,
    )
    .await;

    let refus = live::service::list::gabarit_de_debordement(&bac.state, &perimetre, ailleurs, "fr")
        .await
        .expect_err("hors périmètre");

    assert_eq!(refus.code, ErrorCode::NotFound);
}

#[tokio::test]
async fn une_activite_inexistante_rend_le_meme_404() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;
    let perimetre = perimetre_de(&bac, comptes.detache).await;

    let refus = live::service::list::gabarit_de_debordement(
        &bac.state,
        &perimetre,
        uuid::Uuid::now_v7(),
        "fr",
    )
    .await
    .expect_err("inexistante");

    assert_eq!(
        refus.code,
        ErrorCode::NotFound,
        "inexistante et hors périmètre sont indiscernables"
    );
}
