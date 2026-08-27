//! **Enregistrer sans publier ne fait parler personne.**
//!
//! Enregistrer et publier sont deux actes distincts en base : un brouillon se
//! relit avant de s'adresser à toute une COP.

mod commun;

use commun::*;
use live::domain::incident::IncidentWriteStatus;

#[tokio::test]
async fn enregistrer_sans_publier_rend_created_et_reste_brouillon() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;
    let perimetre = perimetre_de(&bac, comptes.detache).await;

    let mut valeurs = payload("session");
    valeurs.session_id = Some(decor.session_id);

    let resultat = live::service::write::creer(
        &bac.state,
        &bac.ctx(comptes.detache),
        &perimetre,
        decor.event_id,
        &valeurs,
    )
    .await
    .expect("écriture");

    assert_eq!(resultat.status, IncidentWriteStatus::Created);
    let ligne = resultat.incident.expect("la ligne relue");
    assert_eq!(ligne.state, "draft");
    assert!(ligne.published_at.is_none());

    // Et il n'apparaît dans aucune lecture active.
    let actifs = live::repo::active::pour_ledition(bac.pool(), decor.event_id)
        .await
        .expect("lecture publique");
    assert!(actifs.is_empty(), "un brouillon ne parle à personne");
}
