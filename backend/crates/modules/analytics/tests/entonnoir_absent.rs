//! **Une édition sans appel ni dépôt rend `funnel: null`, pas un entonnoir de
//! zéros.**
//!
//! Un entonnoir à zéro partout serait un graphique qui ment sur sa propre
//! existence : il donnerait à voir une campagne qui n'a jamais commencé comme
//! une campagne sans succès.

mod commun;

use commun::*;

#[tokio::test]
async fn sans_appel_ni_depot_lentonnoir_est_nul() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    rafraichir(&bac).await;

    let figures = analytics::service::dashboard::composer(bac.pool(), decor.event_id)
        .await
        .expect("composition")
        .figures;

    assert!(figures.funnel.is_none());
    assert!(figures.deadline.is_none());
    assert!(figures.call_opens_at.is_none());
}

#[tokio::test]
async fn avec_un_appel_lentonnoir_existe() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    appel(&bac, decor.event_id, "cop31_appel", 30).await;
    rafraichir(&bac).await;

    let figures = analytics::service::dashboard::composer(bac.pool(), decor.event_id)
        .await
        .expect("composition")
        .figures;

    let entonnoir = figures.funnel.expect("l'appel a son entonnoir");
    assert_eq!(entonnoir.event_id, decor.event_id);
    assert_eq!(
        entonnoir.deposees, 0,
        "un appel que personne n'a vu est précisément l'information à voir remonter"
    );
    assert!(figures.deadline.is_some(), "l'échéance qui fait foi");
    assert!(figures.call_opens_at.is_some());
}
