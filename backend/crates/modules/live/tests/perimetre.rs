//! **Les trois cas du périmètre restent distincts.**
//!
//! Périmètre vide → **403** explicite, jamais une liste vide : confondre les
//! deux affiche un écran vide là où il faut un refus d'accès. Édition hors
//! périmètre, URL forgée → **404**, jamais 403 — un identifiant hors périmètre
//! se refuse comme un identifiant inexistant, sans quoi la forme de la réponse
//! apprendrait à qui la forge que l'objet existe ailleurs.

mod commun;

use commun::*;
use kernel::error::ErrorCode;

#[tokio::test]
async fn un_perimetre_vide_recoit_403_et_jamais_une_liste_vide() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;

    let refus = live::service::list::ecran(
        &bac.state,
        &perimetre_vide(comptes.sans_droit),
        decor.event_id,
        "fr",
    )
    .await
    .expect_err("un périmètre vide se refuse");

    assert_eq!(refus.code, ErrorCode::Forbidden);
}

#[tokio::test]
async fn une_edition_hors_perimetre_recoit_404_et_jamais_403() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;
    let perimetre = perimetre_de(&bac, comptes.detache).await;

    let refus = live::service::list::ecran(&bac.state, &perimetre, decor.autre_event_id, "fr")
        .await
        .expect_err("l'URL forgée se refuse");

    assert_eq!(
        refus.code,
        ErrorCode::NotFound,
        "hors périmètre et inexistant sont indiscernables"
    );
}

#[tokio::test]
async fn dans_son_perimetre_ladministrateur_detache_obtient_son_ecran() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;
    let perimetre = perimetre_de(&bac, comptes.detache).await;

    let ecran = live::service::list::ecran(&bac.state, &perimetre, decor.event_id, "fr")
        .await
        .expect("son édition s'ouvre");

    assert_eq!(ecran.event_id, decor.event_id);
}
