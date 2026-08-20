//! **Créer une édition exige la portée globale, et pas une autre** (FR-011).
//!
//! Une édition qui n'existe pas encore n'offre aucune portée où vérifier un
//! droit. Exiger la permission « sur cette édition » reviendrait à la vérifier
//! sur un identifiant que personne ne détient : le test rendrait faux pour tout
//! le monde, ou vrai pour tout le monde, selon la façon dont il est écrit — et
//! les deux sont des défauts.
//!
//! Le refus porte un code **distinct de celui d'une permission absente**, parce
//! que l'écran sait en tirer une phrase : « demandez des droits sur l'ensemble
//! de la plateforme » n'est pas « vous n'avez pas les droits ».

mod commun;

use commun::{perimetres, seed, Bac};
use event::service::portee_globale_exigee;
use kernel::error::ErrorCode;

#[tokio::test]
async fn un_compte_detache_ne_peut_pas_creer_une_edition() {
    let bac = Bac::monter().await;
    let editions = seed::editions(&bac).await;
    let p = perimetres(&bac, &editions).await;

    let refus = portee_globale_exigee(bac.pool(), p.detache)
        .await
        .expect_err("un compte détaché ne crée pas d'édition");

    assert_eq!(refus.code, ErrorCode::EventGlobalScopeRequired);
}

#[tokio::test]
async fn un_compte_global_peut_creer_une_edition() {
    let bac = Bac::monter().await;
    let editions = seed::editions(&bac).await;
    let p = perimetres(&bac, &editions).await;

    portee_globale_exigee(bac.pool(), p.globale)
        .await
        .expect("un compte global crée");
}

/// **Le compte détaché administre bien son édition** : le refus précédent vient
/// de la portée exigée, pas d'une absence de droits. Sans cette moitié-là, le
/// test passerait aussi sur un compte qui n'a rien.
#[tokio::test]
async fn le_refus_vient_de_la_portee_et_non_dune_absence_de_droits() {
    let bac = Bac::monter().await;
    let editions = seed::editions(&bac).await;
    let p = perimetres(&bac, &editions).await;

    let sur_son_edition = kernel::auth::has_permission(
        bac.pool(),
        p.detache,
        "event.event.manage",
        kernel::auth::Scope::Event(p.edition_detachee),
    )
    .await
    .expect("lecture de la permission");

    assert!(
        sur_son_edition,
        "le compte détaché gère bien son édition : c'est la PORTÉE qui lui manque, pas le droit"
    );
}

/// Un compte **sans aucun droit** reçoit le même refus : le code dit la portée
/// exigée, il ne dit pas que la personne est administratrice.
#[tokio::test]
async fn un_compte_sans_droit_recoit_le_meme_refus() {
    let bac = Bac::monter().await;
    let editions = seed::editions(&bac).await;
    let p = perimetres(&bac, &editions).await;

    let refus = portee_globale_exigee(bac.pool(), p.sans_droit)
        .await
        .expect_err("aucun droit, aucune création");

    assert_eq!(refus.code, ErrorCode::EventGlobalScopeRequired);
}
