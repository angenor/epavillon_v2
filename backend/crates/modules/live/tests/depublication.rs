//! **Retirer n'est pas supprimer.**
//!
//! La ligne demeure — instant, auteur, motif — et reparaît à l'historique de la
//! liste. C'est ce qui manquait à la v1, dont les bandeaux disparaissaient sans
//! qu'on sache qui les avait retirés ni pourquoi.

mod commun;

use commun::*;
use live::domain::incident::IncidentWriteStatus;

#[tokio::test]
async fn le_retrait_garde_la_ligne_son_auteur_et_son_motif() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;
    let perimetre = perimetre_de(&bac, comptes.detache).await;

    let id = poser(
        &bac,
        comptes.detache,
        "session",
        Some(decor.session_id),
        "active",
    )
    .await;

    let resultat = live::service::write::depublier(
        &bac.state,
        &bac.ctx(comptes.detache),
        &perimetre,
        id,
        Some("La diffusion est rétablie."),
    )
    .await
    .expect("retrait");

    assert_eq!(resultat.status, IncidentWriteStatus::Unpublished);
    let ligne = resultat.incident.expect("la ligne demeure");
    assert_eq!(ligne.state, "unpublished");
    assert!(ligne.unpublished_at.is_some());
    assert_eq!(
        ligne.unpublish_reason.as_deref(),
        Some("La diffusion est rétablie.")
    );
    assert!(
        ligne.unpublished_by_name.is_some(),
        "l'historique nomme qui a retiré — jamais « retiré par — »"
    );

    // Et il reparaît à l'historique de la liste.
    let ecran = live::service::list::composer(bac.pool(), decor.event_id, "fr")
        .await
        .expect("composition");
    assert!(ecran.rows.iter().any(|r| r.incident_id == id));
    assert_eq!(ecran.counts.get("unpublished"), Some(&1));
}
