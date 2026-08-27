//! **Le rafraîchissement porte les huit projections, en mode concurrent, et
//! journalise chacune.**
//!
//! Le mode concurrent depuis une fonction a été **mesuré**, pas supposé : le
//! contraire aurait journalisé huit échecs **sans lever** — l'exception étant
//! avalée vue par vue — et le tableau de bord aurait vieilli en silence pendant
//! que le worker croyait travailler.

mod commun;

use commun::*;

#[tokio::test]
async fn le_rafraichissement_touche_les_huit_projections_et_les_journalise() {
    let bac = Bac::monter().await;

    // Premier peuplement hors mode concurrent : `REFRESH … CONCURRENTLY` refuse
    // une vue matérialisée jamais peuplée, et c'est l'état d'une base neuve.
    rafraichir(&bac).await;

    let echecs = analytics::jobs::refresh::rafraichir(bac.pool())
        .await
        .expect("rafraîchissement concurrent");

    assert!(
        echecs.is_empty(),
        "les huit vues passent en mode concurrent : {echecs:?}"
    );

    let journal = sqlx::query!(
        r#"SELECT view_name AS "vue!", succeeded AS "succes!", duration_ms
             FROM analytics.refresh_log
            WHERE was_concurrent
            ORDER BY started_at"#
    )
    .fetch_all(bac.pool())
    .await
    .expect("journal");

    assert_eq!(journal.len(), 8, "une ligne par projection");
    assert!(journal.iter().all(|l| l.succes));
    assert!(
        journal.iter().all(|l| l.duration_ms.is_some()),
        "la durée est journalisée : « le tableau de bord affiche des chiffres d'hier » cesse d'être un signalement invérifiable"
    );
}

#[tokio::test]
async fn les_quatre_projections_non_lues_sont_rafraichies_quand_meme() {
    let bac = Bac::monter().await;
    rafraichir(&bac).await;

    let vues = sqlx::query_scalar!(
        r#"SELECT DISTINCT view_name AS "vue!" FROM analytics.refresh_log ORDER BY view_name"#
    )
    .fetch_all(bac.pool())
    .await
    .expect("journal");

    // Les retirer de la liste serait modifier le modèle pour un gain nul.
    for non_lue in [
        "mv_daily_signups",
        "mv_organization_scorecard",
        "mv_session_attendance",
        "mv_content_popularity",
    ] {
        assert!(
            vues.iter().any(|v| v == non_lue),
            "{non_lue} est rafraîchie"
        );
    }
}
