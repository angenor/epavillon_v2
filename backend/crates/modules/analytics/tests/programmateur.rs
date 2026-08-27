//! **Un programmateur détaché obtient son tableau de bord.**
//!
//! C'est ce que l'attribution ajoutée au modèle le 27/08 rend possible, et c'est
//! le compte avec lequel la règle métier n° 8 a été vérifiée sur cet écran le
//! 17/08. Sans cette ligne, l'écran aurait été refusé au rôle qui pilote une
//! édition — et le défaut ne se serait vu qu'en se connectant avec un tel
//! compte.
//!
//! **Ce n'est pas une élévation** : un programmateur lit déjà, écran par écran
//! et pour sa seule édition, tout ce que le tableau de bord agrège.

mod commun;

use commun::*;
use kernel::error::ErrorCode;

#[tokio::test]
async fn le_programmateur_detache_obtient_son_tableau_de_bord() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let programmateur = personne(&bac, "prog@ifdd.francophonie.org", "Awa", "Sow").await;
    attribuer(
        &bac,
        programmateur,
        "programmer",
        "event",
        Some(decor.event_id),
    )
    .await;
    rafraichir(&bac).await;

    let perimetre = perimetre_de(&bac, programmateur).await;
    let ecran = analytics::service::dashboard::ecran(&bac.state, &perimetre, decor.event_id)
        .await
        .expect("le tableau de bord s'ouvre — c'est l'attribution du 27/08");

    assert_eq!(ecran.edition.id, decor.event_id);
}

#[tokio::test]
async fn et_pas_celui_dune_autre_edition() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let programmateur = personne(&bac, "prog@ifdd.francophonie.org", "Awa", "Sow").await;
    attribuer(
        &bac,
        programmateur,
        "programmer",
        "event",
        Some(decor.event_id),
    )
    .await;

    let perimetre = perimetre_de(&bac, programmateur).await;
    let refus = analytics::service::dashboard::ecran(&bac.state, &perimetre, decor.autre_event_id)
        .await
        .expect_err("règle métier n° 8");

    assert_eq!(refus.code, ErrorCode::NotFound);
}
