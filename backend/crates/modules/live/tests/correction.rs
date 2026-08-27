//! **Corriger, et republier — ce qui EFFACE la dépublication.**
//!
//! Le comportement n'est pas recomposé : `live.publish_incident()` efface
//! l'instant, l'auteur et le motif du retrait, et c'est elle qu'on appelle.

mod commun;

use commun::*;
use live::domain::incident::IncidentWriteStatus;

#[tokio::test]
async fn corriger_rend_updated() {
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

    let mut valeurs = payload("session");
    valeurs.session_id = Some(decor.session_id);
    valeurs.severity = "critical".to_owned();

    let resultat = live::service::write::corriger(
        &bac.state,
        &bac.ctx(comptes.detache),
        &perimetre,
        decor.event_id,
        id,
        &valeurs,
    )
    .await
    .expect("correction");

    assert_eq!(resultat.status, IncidentWriteStatus::Updated);
    assert_eq!(resultat.incident.expect("ligne").severity, "critical");
}

#[tokio::test]
async fn republier_efface_le_retrait() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;
    let perimetre = perimetre_de(&bac, comptes.detache).await;

    let id = poser(
        &bac,
        comptes.detache,
        "session",
        Some(decor.session_id),
        "unpublished",
    )
    .await;

    let mut valeurs = payload("session");
    valeurs.session_id = Some(decor.session_id);
    valeurs.publish = true;

    let resultat = live::service::write::corriger(
        &bac.state,
        &bac.ctx(comptes.detache),
        &perimetre,
        decor.event_id,
        id,
        &valeurs,
    )
    .await
    .expect("republication");

    assert_eq!(resultat.status, IncidentWriteStatus::Published);
    let ligne = resultat.incident.expect("ligne");
    assert_eq!(ligne.state, "active");
    assert!(
        ligne.unpublished_at.is_none(),
        "l'instant du retrait est effacé"
    );
    assert!(ligne.unpublish_reason.is_none(), "et son motif aussi");
    assert!(ligne.unpublished_by_name.is_none(), "et son auteur aussi");
}
