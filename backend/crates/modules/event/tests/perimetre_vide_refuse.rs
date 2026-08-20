//! **Un refus, jamais une liste vide.**
//!
//! Les trois cas du périmètre restent distincts jusqu'au bout : global,
//! éditions listées, aucun droit. Les confondre afficherait une liste vide là
//! où il faut un refus d'accès, et personne ne saurait s'il n'y a rien à voir
//! ou s'il n'a pas le droit de voir.
//!
//! Ce fichier éprouve le **garde**, avant que les lectures du back-office
//! existent : c'est la fondation sur laquelle chacune s'appuiera. Les routes
//! s'y ajoutent au fil des phases, et `perimetre_edition_url_forgee.rs` couvre
//! celles qui remontent par un enfant.
//!
//! **Une seule route du module fera exception, et c'est écrit** : `GET /events`,
//! le sélecteur du back-office, est *filtrée* et non refusée sur périmètre
//! vide, parce que le contrat du front le veut ainsi.

mod commun;

use commun::{perimetres, seed, Bac};
use kernel::error::ErrorCode;

#[tokio::test]
async fn un_perimetre_vide_se_refuse_explicitement() {
    let bac = Bac::monter().await;
    let editions = seed::editions(&bac).await;
    let p = perimetres(&bac, &editions).await;

    let refus = kernel::auth::require_perimeter(bac.pool(), p.sans_droit)
        .await
        .expect_err("un périmètre vide se refuse");

    assert_eq!(refus.code, ErrorCode::Forbidden);
}

/// Les trois cas se lisent distinctement, et c'est ce qui permet à l'écran de
/// dire la bonne chose.
#[tokio::test]
async fn les_trois_cas_du_perimetre_restent_distincts() {
    let bac = Bac::monter().await;
    let editions = seed::editions(&bac).await;
    let p = perimetres(&bac, &editions).await;

    let global = kernel::auth::administered_events(bac.pool(), p.globale)
        .await
        .expect("lecture");
    assert!(global.is_global);
    assert!(!global.is_empty());

    let detache = kernel::auth::administered_events(bac.pool(), p.detache)
        .await
        .expect("lecture");
    assert!(!detache.is_global);
    assert_eq!(detache.event_ids, vec![p.edition_detachee]);
    assert!(!detache.is_empty());

    let aucun = kernel::auth::administered_events(bac.pool(), p.sans_droit)
        .await
        .expect("lecture");
    assert!(!aucun.is_global);
    assert!(
        aucun.is_empty(),
        "aucun droit : ce n'est pas « une liste vide », c'est un refus"
    );
}

/// **Un compte détaché sur une édition ne voit pas l'autre**, et le refus qu'il
/// reçoit sur la COP31 est celui d'un identifiant inexistant. C'est le socle du
/// contrôle d'URL forgée que chaque route paramétrée reprendra.
#[tokio::test]
async fn une_edition_hors_perimetre_se_refuse_comme_une_edition_inexistante() {
    let bac = Bac::monter().await;
    let editions = seed::editions(&bac).await;
    let p = perimetres(&bac, &editions).await;

    let perimetre = commun::perimetre_de(&bac, p.detache).await;

    assert!(perimetre.ensure(editions.sans_pavillon).is_ok());

    let refus = perimetre
        .ensure(editions.cop31)
        .expect_err("la COP31 est hors du périmètre détaché");
    assert_eq!(refus.code, ErrorCode::NotFound);
}
