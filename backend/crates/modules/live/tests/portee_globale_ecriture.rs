//! **Un message de portée globale exige la portée GLOBALE.**
//!
//! Il s'affiche partout : un compte détaché sur une seule édition n'a pas à en
//! poser ni à en retirer un. La différence est portée par
//! `identity.has_permission()`, **sans une ligne de code supplémentaire** — un
//! compte détaché ne détient sa permission que sur `event:<son édition>`.

mod commun;

use commun::*;
use live::domain::incident::IncidentWriteStatus;

#[tokio::test]
async fn un_compte_detache_ne_publie_pas_un_message_global() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;
    let perimetre = perimetre_de(&bac, comptes.detache).await;

    let valeurs = payload("global");
    let resultat = live::service::write::creer(
        &bac.state,
        &bac.ctx(comptes.detache),
        &perimetre,
        decor.event_id,
        &valeurs,
    )
    .await
    .expect("réponse");

    assert_eq!(resultat.status, IncidentWriteStatus::Forbidden);
}

#[tokio::test]
async fn un_compte_global_le_peut() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;
    let perimetre = perimetre_de(&bac, comptes.globale).await;

    let valeurs = payload("global");
    let resultat = live::service::write::creer(
        &bac.state,
        &bac.ctx(comptes.globale),
        &perimetre,
        decor.event_id,
        &valeurs,
    )
    .await
    .expect("réponse");

    assert_eq!(resultat.status, IncidentWriteStatus::Created);
}

#[tokio::test]
async fn un_compte_detache_ne_retire_pas_un_message_global() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;

    let global = poser(&bac, comptes.globale, "global", None, "active").await;

    let perimetre = perimetre_de(&bac, comptes.detache).await;
    let resultat = live::service::write::depublier(
        &bac.state,
        &bac.ctx(comptes.detache),
        &perimetre,
        global,
        Some("Motif."),
    )
    .await
    .expect("réponse");

    assert_eq!(
        resultat.status,
        IncidentWriteStatus::Forbidden,
        "il le VOIT — c'est voulu — mais il ne le retire pas"
    );
}
