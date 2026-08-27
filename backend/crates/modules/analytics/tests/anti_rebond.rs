//! **Cent demandes dans la même tranche ne produisent qu'un travail.**
//!
//! C'est le mécanisme qui évite qu'une clôture d'appel — donc une rafale de
//! décisions — ne déclenche cent recalculs complets.
//!
//! **Et le piège qu'il tend** : le conflit porte sur `(task, idempotency_key)`
//! quel que soit l'état du travail, `cancelled` excepté. Un travail **déjà
//! réussi** bloque donc une nouvelle mise en file de la même tranche.

mod commun;

use commun::*;

#[tokio::test]
async fn cent_demandes_dans_la_meme_tranche_ne_produisent_quun_travail() {
    let bac = Bac::monter().await;

    for _ in 0..100 {
        sqlx::query_scalar!(
            r#"SELECT analytics.enqueue_refresh(true, interval '0', 300) AS "id?""#
        )
        .fetch_one(bac.pool())
        .await
        .expect("mise en file");
    }

    let travaux = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM platform.jobs
            WHERE task = 'analytics.refresh_all'"#
    )
    .fetch_one(bac.pool())
    .await
    .expect("lecture de la file");

    assert_eq!(travaux, 1);
}

#[tokio::test]
async fn un_travail_deja_reussi_bloque_encore_sa_tranche() {
    let bac = Bac::monter().await;

    let premier = sqlx::query_scalar!(
        r#"SELECT analytics.enqueue_refresh(true, interval '0', 300) AS "id?""#
    )
    .fetch_one(bac.pool())
    .await
    .expect("mise en file")
    .expect("un travail est né");

    sqlx::query!(
        "UPDATE platform.jobs SET status = 'succeeded', completed_at = now() WHERE id = $1",
        premier
    )
    .execute(bac.pool())
    .await
    .expect("réussite simulée");

    let second = sqlx::query_scalar!(
        r#"SELECT analytics.enqueue_refresh(true, interval '0', 300) AS "id?""#
    )
    .fetch_one(bac.pool())
    .await
    .expect("mise en file");

    assert!(
        second.is_none(),
        "**C'EST LE PIÈGE** : un intervalle plus court que cette fenêtre ferait se dédoublonner la chaîne contre elle-même, et elle s'arrêterait sans erreur ni trace"
    );
}
