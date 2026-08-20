//! **Détenir la permission de consultation sans périmètre n'ouvre pas le
//! back-office** (écart n° 73).
//!
//! `org.organization.read` est détenue par le rôle d'utilisateur ordinaire —
//! c'est ce qui rend la recherche accessible à tous. La liste du back-office
//! exige donc la permission **ET** un périmètre non vide : la tester seule
//! ouvrirait le référentiel entier à n'importe quel inscrit.

mod commun;

use commun::{perimetres, Bac};
use org::domain::permissions::ORGANIZATION_READ;

#[tokio::test]
async fn la_permission_de_consultation_seule_nouvre_pas_le_back_office() {
    let bac = Bac::monter().await;
    let p = perimetres(&bac).await;

    // La personne détient bien la permission — le rôle d'utilisateur ordinaire
    // la porte.
    let detient =
        kernel::auth::has_permission_anywhere(bac.pool(), p.sans_droit, ORGANIZATION_READ)
            .await
            .expect("lecture de la permission");
    assert!(
        detient,
        "le rôle d'utilisateur ordinaire détient bien la permission de consultation"
    );

    // Et pourtant le back-office lui est fermé : le périmètre est vide.
    let perimetre = kernel::auth::require_perimeter(bac.pool(), p.sans_droit).await;
    assert!(
        perimetre.is_err(),
        "la permission seule ne suffit pas : sans périmètre, le back-office refuse"
    );
}

/// L'inverse est vrai aussi : un périmètre sans la permission ne suffit pas
/// davantage. Les deux gardes sont indépendantes, et c'est voulu.
#[tokio::test]
async fn les_deux_gardes_sont_independantes() {
    let bac = Bac::monter().await;
    let p = perimetres(&bac).await;

    for personne in [p.globale, p.detachee] {
        assert!(
            kernel::auth::has_permission_anywhere(bac.pool(), personne, ORGANIZATION_READ)
                .await
                .expect("lecture"),
            "les deux administrateurs détiennent la permission"
        );
        assert!(kernel::auth::require_perimeter(bac.pool(), personne)
            .await
            .is_ok());
    }
}
