//! **Les trois refus du tableau de bord, et ils ne se confondent pas.**
//!
//! Périmètre vide → **403**. Édition hors périmètre → **404**, jamais 403.
//! `analytics.dashboard.read` absente sur l'édition → **403**.
//!
//! Le tableau de bord n'a **pas d'issue de contrat** : il s'ouvre ou il se
//! refuse — à la différence des écritures d'incident, dont les dix issues
//! sortent en 200.

mod commun;

use commun::*;
use kernel::error::ErrorCode;

#[tokio::test]
async fn un_perimetre_vide_recoit_403() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;

    let refus = analytics::service::dashboard::ecran(
        &bac.state,
        &perimetre_vide(comptes.sans_droit),
        decor.event_id,
    )
    .await
    .expect_err("aucun droit d'administration");

    assert_eq!(refus.code, ErrorCode::Forbidden);
}

#[tokio::test]
async fn une_edition_hors_perimetre_recoit_404() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;
    let perimetre = perimetre_de(&bac, comptes.detache).await;

    let refus = analytics::service::dashboard::ecran(&bac.state, &perimetre, decor.autre_event_id)
        .await
        .expect_err("URL forgée");

    assert_eq!(
        refus.code,
        ErrorCode::NotFound,
        "hors périmètre et inexistante sont indiscernables"
    );
}

#[tokio::test]
async fn la_permission_absente_sur_ledition_recoit_403() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;

    // Le compte administre l'édition, mais on lui retire la permission du
    // tableau de bord : la garde tient sur la PERMISSION, pas sur le périmètre.
    retirer_la_permission(&bac, "admin", "analytics.dashboard.read").await;
    let perimetre = perimetre_de(&bac, comptes.detache).await;

    let refus = analytics::service::dashboard::ecran(&bac.state, &perimetre, decor.event_id)
        .await
        .expect_err("permission absente");

    assert_eq!(refus.code, ErrorCode::Forbidden);
}

#[tokio::test]
async fn avec_la_permission_sur_son_edition_lecran_souvre() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;
    let perimetre = perimetre_de(&bac, comptes.detache).await;
    rafraichir(&bac).await;

    let ecran = analytics::service::dashboard::ecran(&bac.state, &perimetre, decor.event_id)
        .await
        .expect("son édition s'ouvre");

    assert_eq!(ecran.edition.id, decor.event_id);
}
