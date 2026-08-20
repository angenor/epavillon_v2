//! **Un refus, jamais une liste vide** (FR-043).
//!
//! Les trois cas du périmètre restent distincts jusqu'au bout : global, éditions
//! listées, aucun droit. Les confondre afficherait une liste vide là où il faut
//! un refus d'accès, et personne ne saurait s'il n'y a rien à voir ou s'il n'a
//! pas le droit de voir.

mod commun;

use commun::{perimetres, Bac};
use kernel::error::ErrorCode;

#[tokio::test]
async fn un_perimetre_vide_se_refuse_explicitement() {
    let bac = Bac::monter().await;
    let p = perimetres(&bac).await;

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
    let p = perimetres(&bac).await;

    let global = kernel::auth::administered_events(bac.pool(), p.globale)
        .await
        .expect("lecture");
    assert!(global.is_global);
    assert!(!global.is_empty());

    let detache = kernel::auth::administered_events(bac.pool(), p.detachee)
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
