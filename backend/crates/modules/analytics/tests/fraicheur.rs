//! **La fraîcheur est `max(finished_at)` SUR LES SUCCÈS.**
//!
//! Une exécution partielle laisse des lignes en échec **plus récentes** que le
//! dernier succès complet : prendre la dernière ligne ferait avancer la
//! fraîcheur affichée alors que les chiffres, eux, n'ont pas bougé.

mod commun;

use commun::*;

#[tokio::test]
async fn sans_rafraichissement_la_fraicheur_est_nulle() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;

    let figures = analytics::service::dashboard::composer(bac.pool(), decor.event_id)
        .await
        .expect("composition")
        .figures;

    assert!(
        figures.refreshed_at.is_none(),
        "aucun rafraîchissement n'a jamais abouti : l'écran le dit plutôt que d'inventer une date"
    );
}

#[tokio::test]
async fn un_echec_plus_recent_ne_fait_pas_avancer_la_fraicheur() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    rafraichir(&bac).await;

    let apres_succes = analytics::service::dashboard::composer(bac.pool(), decor.event_id)
        .await
        .expect("composition")
        .figures
        .refreshed_at
        .expect("un succès a eu lieu");

    // Une ligne d'échec **postérieure**, comme en laisserait une exécution
    // partielle.
    sqlx::query!(
        "INSERT INTO analytics.refresh_log
             (view_name, started_at, finished_at, succeeded, error_message)
         VALUES ('mv_proposal_funnel', now() + interval '1 hour',
                 now() + interval '1 hour', false, 'échec simulé')"
    )
    .execute(bac.pool())
    .await
    .expect("journal");

    let apres_echec = analytics::service::dashboard::composer(bac.pool(), decor.event_id)
        .await
        .expect("composition")
        .figures
        .refreshed_at
        .expect("le dernier succès reste");

    assert_eq!(
        apres_echec, apres_succes,
        "la fraîcheur se lit sur les SUCCÈS : un échec plus récent ne la fait pas avancer"
    );
}
