//! **L'échec d'une projection n'arrête pas les sept autres — et ne fait pas
//! avancer la fraîcheur.**
//!
//! C'est la décision du modèle, pas la nôtre : « un tableau de bord
//! partiellement à jour vaut mieux qu'un tableau de bord entièrement périmé
//! parce qu'une seule agrégation a fauté ». Le gestionnaire journalise
//! l'avertissement **sans rendre d'erreur** : rendre une erreur ferait
//! recommencer huit rafraîchissements pour un seul échec.

mod commun;

use commun::*;

#[tokio::test]
async fn sept_succes_un_echec_et_la_fraicheur_ne_bouge_pas_pour_lechec() {
    let bac = Bac::monter().await;
    rafraichir(&bac).await;

    // Rendre UNE vue inaccessible : son index unique disparaît, et le
    // rafraîchissement concurrent la refuse — les sept autres n'en savent rien.
    sqlx::query!("DROP INDEX analytics.ux_mv_proposal_funnel")
        .execute(bac.pool())
        .await
        .expect("index retiré");

    let echecs = analytics::jobs::refresh::rafraichir(bac.pool())
        .await
        .expect("le travail ne rend PAS d'erreur pour autant");

    assert_eq!(
        echecs,
        vec!["mv_proposal_funnel"],
        "un échec nommé, et un seul"
    );

    let concurrent = sqlx::query!(
        r#"SELECT succeeded AS "succes!" FROM analytics.refresh_log WHERE was_concurrent"#
    )
    .fetch_all(bac.pool())
    .await
    .expect("journal");

    assert_eq!(concurrent.len(), 8);
    assert_eq!(
        concurrent.iter().filter(|l| l.succes).count(),
        7,
        "sept succès, un échec"
    );
}
