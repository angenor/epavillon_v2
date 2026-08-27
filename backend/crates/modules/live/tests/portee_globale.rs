//! **Un message global se voit de toute édition administrée, et rien d'une
//! autre édition ne fuit.**
//!
//! Les deux moitiés comptent autant l'une que l'autre. La première est voulue :
//! une équipe qui pilote un pavillon doit savoir qu'un bandeau d'entretien le
//! couvre. La seconde est la règle métier n° 8 — un administrateur détaché ne
//! doit rien apprendre de l'existence des autres éditions.

mod commun;

use commun::*;

#[tokio::test]
async fn le_message_global_apparait_sur_chaque_edition() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;

    let global = poser(&bac, comptes.globale, "global", None, "active").await;

    for edition in [decor.event_id, decor.autre_event_id] {
        let ecran = live::service::list::composer(bac.pool(), edition, "fr")
            .await
            .expect("composition de l'écran");
        assert!(
            ecran.rows.iter().any(|r| r.incident_id == global),
            "le message global couvre aussi cette édition"
        );
    }
}

#[tokio::test]
async fn un_message_dune_autre_edition_napparait_pas() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;

    // Une activité de l'AUTRE édition, et un message qui la vise.
    let ailleurs = activite(
        &bac,
        decor.autre_event_id,
        None,
        None,
        "Séance d'une autre COP",
        "seance-autre-cop",
        aujourdhui_a(&bac, decor.autre_event_id, time::macros::time!(09:00)).await,
        aujourdhui_a(&bac, decor.autre_event_id, time::macros::time!(10:00)).await,
    )
    .await;
    let etranger = poser(&bac, comptes.globale, "session", Some(ailleurs), "active").await;

    let ecran = live::service::list::composer(bac.pool(), decor.event_id, "fr")
        .await
        .expect("composition de l'écran");

    assert!(
        !ecran.rows.iter().any(|r| r.incident_id == etranger),
        "un message de portée `session` d'une autre édition ne franchit pas la frontière"
    );
}
