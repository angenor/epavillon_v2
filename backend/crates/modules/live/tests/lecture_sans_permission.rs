//! **Lire n'est pas un privilège.**
//!
//! Un compte qui administre l'édition mais ne détient pas
//! `live.incident.publish` voit la liste : un bandeau publié est de toute façon
//! public, et exiger une permission pour le lire protégerait un texte déjà
//! lisible de tous. Ce qui est gardé, c'est le périmètre — et l'écriture.

mod commun;

use commun::*;

#[tokio::test]
async fn un_compte_sans_droit_de_publication_voit_la_liste() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;

    poser(
        &bac,
        comptes.globale,
        "event",
        Some(decor.event_id),
        "active",
    )
    .await;

    let perimetre = perimetre_de(&bac, comptes.lectrice).await;
    let ecran = live::service::list::ecran(&bac.state, &perimetre, decor.event_id, "fr")
        .await
        .expect("la liste s'ouvre sans permission de publication");

    assert_eq!(ecran.rows.len(), 1);
    assert!(
        !kernel::auth::has_permission(
            bac.pool(),
            comptes.lectrice,
            "live.incident.publish",
            kernel::auth::Scope::Event(decor.event_id),
        )
        .await
        .expect("lecture de la permission"),
        "et ce compte ne peut effectivement pas publier — sans quoi le test ne prouverait rien"
    );
}
