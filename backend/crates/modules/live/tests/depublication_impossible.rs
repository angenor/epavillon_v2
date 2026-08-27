//! **Retirer un message jamais publié rend `not_published`.**
//!
//! La condition n'est **pas rejouée en amont** : `live.unpublish_incident()`
//! l'exige déjà, sa levée est traduite, et la règle vit à un seul endroit.

mod commun;

use commun::*;
use live::domain::incident::IncidentWriteStatus;

#[tokio::test]
async fn retirer_un_brouillon_rend_not_published() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;
    let perimetre = perimetre_de(&bac, comptes.detache).await;

    let id = poser(
        &bac,
        comptes.detache,
        "session",
        Some(decor.session_id),
        "draft",
    )
    .await;

    let resultat = live::service::write::depublier(
        &bac.state,
        &bac.ctx(comptes.detache),
        &perimetre,
        id,
        None,
    )
    .await
    .expect("la levée est TRADUITE, pas propagée");

    assert_eq!(resultat.status, IncidentWriteStatus::NotPublished);
    assert!(
        resultat.incident.is_none(),
        "sur un refus, la ligne est nulle"
    );
}
