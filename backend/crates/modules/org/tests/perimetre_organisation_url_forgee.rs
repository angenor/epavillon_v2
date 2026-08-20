//! **Une URL forgée ne mène nulle part** — obligation n° 2 du principe X
//! (SC-009).
//!
//! Le filtrage par périmètre d'administration ne peut pas vivre dans
//! l'interface : quelqu'un qui connaît un identifiant l'écrit dans la barre
//! d'adresse. Le refus est **indiscernable d'une fiche inexistante** — distinguer
//! les deux dirait qu'une organisation existe et qu'on n'y a pas droit, ce qui
//! est une information de plus que le refus.

mod commun;

use commun::{ifdd, perimetres, Bac};
use org::domain::ids::OrganizationId;
use org::service::admin_detail;
use uuid::Uuid;

#[tokio::test]
async fn une_fiche_hors_perimetre_est_indiscernable_dune_fiche_inexistante() {
    let bac = Bac::monter().await;
    let p = perimetres(&bac).await;
    let hors_perimetre = ifdd(&bac).await;

    let perimetre = kernel::auth::require_perimeter(bac.pool(), p.detachee)
        .await
        .expect("périmètre détaché");

    // L'IFDD n'a rien déposé dans l'édition administrée : elle est hors
    // périmètre, et son identifiant est pourtant parfaitement valide.
    let forgee = admin_detail::detail(bac.pool(), &perimetre, OrganizationId(hors_perimetre))
        .await
        .expect("la lecture ne produit pas d'erreur");

    // Un identifiant qui n'existe nulle part.
    let inexistante = admin_detail::detail(bac.pool(), &perimetre, OrganizationId(Uuid::now_v7()))
        .await
        .expect("la lecture ne produit pas d'erreur");

    assert!(forgee.is_none(), "la fiche hors périmètre ne s'ouvre pas");
    assert!(inexistante.is_none());
    assert_eq!(
        forgee.is_none(),
        inexistante.is_none(),
        "les deux refus doivent être indiscernables : c'est ce qui empêche de \
         sonder l'existence d'une fiche en forgeant des adresses"
    );

    // Et la même fiche s'ouvre pour qui a le périmètre global : c'est bien le
    // périmètre qui refuse, pas la fiche qui manque.
    let global = kernel::auth::require_perimeter(bac.pool(), p.globale)
        .await
        .expect("périmètre global");
    assert!(
        admin_detail::detail(bac.pool(), &global, OrganizationId(hors_perimetre))
            .await
            .expect("lecture")
            .is_some()
    );
}
