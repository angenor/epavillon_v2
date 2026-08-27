//! **Le périmètre d'une écriture, et ce que ses refus ne disent jamais.**
//!
//! `from_event_id` hors périmètre → **404**. Périmètre vide → **403**. Et aucun
//! message d'erreur ne **nomme** l'édition, l'activité ou l'organisation d'une
//! cible hors périmètre : citer « COP30 — Bakou » à un compte détaché sur la
//! COP31 lui apprendrait qu'elle existe.

mod commun;

use commun::*;
use kernel::error::ErrorCode;
use live::domain::incident::IncidentWriteStatus;

#[tokio::test]
async fn un_from_event_id_hors_perimetre_rend_404() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;
    let perimetre = perimetre_de(&bac, comptes.detache).await;

    let valeurs = payload("global");
    let refus = live::service::write::creer(
        &bac.state,
        &bac.ctx(comptes.detache),
        &perimetre,
        decor.autre_event_id,
        &valeurs,
    )
    .await
    .expect_err("URL forgée");

    assert_eq!(refus.code, ErrorCode::NotFound);
    assert!(
        !refus.message.contains("Bakou") && !refus.message.contains("COP30"),
        "le refus ne nomme pas ce qui est hors périmètre : {}",
        refus.message
    );
}

#[tokio::test]
async fn un_perimetre_vide_rend_403() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;

    let valeurs = payload("global");
    let refus = live::service::write::creer(
        &bac.state,
        &bac.ctx(comptes.sans_droit),
        &perimetre_vide(comptes.sans_droit),
        decor.event_id,
        &valeurs,
    )
    .await
    .expect_err("aucun droit d'administration");

    assert_eq!(refus.code, ErrorCode::Forbidden);
}

#[tokio::test]
async fn viser_une_activite_dune_autre_edition_rend_missing_target_sans_la_nommer() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;
    let perimetre = perimetre_de(&bac, comptes.globale).await;

    let ailleurs = activite(
        &bac,
        decor.autre_event_id,
        None,
        None,
        "Séance d'ailleurs",
        "seance-ailleurs",
        aujourdhui_a(&bac, decor.autre_event_id, time::macros::time!(09:00)).await,
        aujourdhui_a(&bac, decor.autre_event_id, time::macros::time!(10:00)).await,
    )
    .await;

    let mut valeurs = payload("session");
    valeurs.session_id = Some(ailleurs);

    let resultat = live::service::write::creer(
        &bac.state,
        &bac.ctx(comptes.globale),
        &perimetre,
        decor.event_id,
        &valeurs,
    )
    .await
    .expect("réponse");

    assert_eq!(
        resultat.status,
        IncidentWriteStatus::MissingTarget,
        "la cible n'appartient pas à l'édition depuis laquelle on agit"
    );
}
