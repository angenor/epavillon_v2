//! **Dix armements dans la même tranche ne posent qu'un travail.**
//!
//! C'est l'anti-rebond du modèle qui le garantit, et non un contrôle écrit ici :
//! `analytics.enqueue_refresh()` porte la clé `refresh_all:<tranche>`, et le
//! conflit de `platform.jobs` la reconnaît.

mod commun;

use commun::*;
use std::time::Duration;

#[tokio::test]
async fn dix_appels_dans_la_meme_tranche_ne_posent_quun_travail() {
    let bac = Bac::monter().await;

    for _ in 0..10 {
        let mut tx = bac
            .db()
            .write(&kernel::context::RequestContext::background("test"))
            .await
            .expect("transaction");
        analytics::jobs::refresh::planifier(
            &mut tx,
            Duration::from_secs(900),
            Duration::from_secs(300),
        )
        .await
        .expect("mise en file");
        tx.commit().await.expect("validation");
    }

    let travaux = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM platform.jobs
            WHERE task = 'analytics.refresh_all'"#
    )
    .fetch_one(bac.pool())
    .await
    .expect("lecture de la file");

    assert_eq!(travaux, 1, "l'anti-rebond confond les dix demandes");
}

#[tokio::test]
async fn le_premier_armement_rend_vrai_et_les_suivants_faux() {
    let bac = Bac::monter().await;

    let mut poses = Vec::new();
    for _ in 0..3 {
        let mut tx = bac
            .db()
            .write(&kernel::context::RequestContext::background("test"))
            .await
            .expect("transaction");
        poses.push(
            analytics::jobs::refresh::planifier(
                &mut tx,
                Duration::from_secs(900),
                Duration::from_secs(300),
            )
            .await
            .expect("mise en file"),
        );
        tx.commit().await.expect("validation");
    }

    assert_eq!(
        poses,
        vec![true, false, false],
        "« faux » n'est pas une erreur : c'est le résultat attendu de l'anti-rebond"
    );
}
