//! **Publier dans le même geste : horodaté, attribué, dans la même
//! transaction.**
//!
//! Un message enregistré sans sa publication laisserait un brouillon là où
//! quelqu'un croit avoir parlé.

mod commun;

use commun::*;
use live::domain::incident::IncidentWriteStatus;

#[tokio::test]
async fn enregistrer_avec_publish_rend_published_horodate_et_attribue() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;
    let perimetre = perimetre_de(&bac, comptes.detache).await;

    let mut valeurs = payload("session");
    valeurs.session_id = Some(decor.session_id);
    valeurs.publish = true;

    let resultat = live::service::write::creer(
        &bac.state,
        &bac.ctx(comptes.detache),
        &perimetre,
        decor.event_id,
        &valeurs,
    )
    .await
    .expect("écriture");

    assert_eq!(resultat.status, IncidentWriteStatus::Published);
    let ligne = resultat.incident.expect("la ligne relue");
    assert_eq!(ligne.state, "active");
    assert!(ligne.published_at.is_some(), "horodaté par la fonction");
    assert_eq!(
        ligne.published_by,
        Some(comptes.detache),
        "attribué depuis platform.current_actor_id()"
    );
    assert!(ligne.published_by_name.is_some());

    let actifs = live::repo::active::pour_ledition(bac.pool(), decor.event_id)
        .await
        .expect("lecture publique");
    assert_eq!(actifs.len(), 1, "et il parle immédiatement");
}
