//! **Les séries portent leurs jours vides AVEC zéro, et la variation
//! hebdomadaire est nulle sous quatorze jours.**
//!
//! La continuité est garantie **en base** : aucun trou n'est rebouché ici. Un
//! composant de courbe qui trouverait un trou signalerait une requête fautive,
//! pas une donnée manquante.

mod commun;

use analytics::domain::figures::DashboardKpiKey;
use commun::*;

#[tokio::test]
async fn la_courbe_des_depots_est_continue_et_porte_ses_jours_vides_avec_zero() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;

    // **Un appel ouvert il y a trente jours que personne n'a encore vu.** La
    // fenêtre de la projection court alors de l'ouverture à aujourd'hui, et
    // chacun de ces jours porte un zéro. C'est exactement ce que la v1 rendait
    // illisible en n'émettant pas les jours sans dépôt : le frontend
    // reconstituait les trous, chaque écran à sa manière, et deux graphiques de
    // la même donnée finissaient par diverger.
    let _ = comptes;
    appel(&bac, decor.event_id, "cop31_appel", 30).await;
    rafraichir(&bac).await;

    let serie = analytics::service::dashboard::composer(bac.pool(), decor.event_id)
        .await
        .expect("composition")
        .figures
        .submissions;

    assert!(
        serie.len() > 20,
        "la fenêtre court de l'ouverture de l'appel à aujourd'hui : {} jours",
        serie.len()
    );

    // Aucun jour manquant entre le premier et le dernier.
    for paire in serie.windows(2) {
        let ecart = (paire[1].jour - paire[0].jour).whole_days();
        assert_eq!(
            ecart, 1,
            "un jour sans dépôt est une information, pas une absence"
        );
    }

    assert!(
        serie.iter().all(|p| p.valeur == 0),
        "les jours vides valent ZÉRO — aucun trou n'est rebouché côté API"
    );
    assert_eq!(
        serie.last().expect("dernier jour").cumul,
        0,
        "le cumul est porté par la projection, pas recalculé ici"
    );
}

#[tokio::test]
async fn la_variation_hebdomadaire_est_nulle_sous_quatorze_jours_de_serie() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;

    // Un appel ouvert il y a peu : la série est courte.
    let appel_id = sqlx::query_scalar!(
        r#"INSERT INTO event.calls_for_proposals
               (event_id, code, title, status, opens_at, closes_at)
           VALUES ($1, 'appel_court', '{"fr":"Appel","en":"Call"}'::jsonb, 'open',
                   now() - interval '3 days', now() + interval '30 days')
        RETURNING id"#,
        decor.event_id
    )
    .fetch_one(bac.pool())
    .await
    .expect("appel court");

    dossier_depose(
        &bac,
        decor.event_id,
        Some(appel_id),
        decor.organization_id,
        comptes.globale,
        "Un dépôt",
    )
    .await;
    rafraichir(&bac).await;

    let figures = analytics::service::dashboard::composer(bac.pool(), decor.event_id)
        .await
        .expect("composition")
        .figures;

    assert!(
        figures.submissions.len() < 14,
        "série volontairement courte"
    );
    let depots = figures
        .kpis
        .iter()
        .find(|k| k.key == DashboardKpiKey::Submissions)
        .expect("les dépôts");
    assert!(
        depots.delta.is_none(),
        "une variation calculée sur quatre jours de série est un artefact, pas une tendance"
    );
}
