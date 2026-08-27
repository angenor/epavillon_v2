//! **L'échéance applicable n'est pas celle de l'appel quand le dossier est
//! confié.**
//!
//! Un dossier confié porte `min(review_assignments.due_at)` sur ses affectations
//! non déportées ; un dossier **sans aucune affectation** entre dans la famille
//! quelle que soit l'échéance — c'est un oubli d'affectation, qui n'a pas
//! d'heure.

mod commun;

use analytics::domain::action::AdminActionKind;
use commun::*;

#[tokio::test]
async fn un_dossier_sans_affectation_entre_quelle_que_soit_lecheance() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;

    // Échéance d'appel très lointaine : sans affectation, le dossier s'allume
    // quand même.
    let appel_id = appel(&bac, decor.event_id, "cop31_appel", 300).await;
    dossier_depose(
        &bac,
        decor.event_id,
        Some(appel_id),
        decor.organization_id,
        comptes.globale,
        "Dossier orphelin",
    )
    .await;
    rafraichir(&bac).await;

    let actions = analytics::service::dashboard::composer(bac.pool(), decor.event_id)
        .await
        .expect("composition")
        .actions;

    assert!(
        actions
            .iter()
            .any(|a| a.kind == AdminActionKind::ProposalsUnreviewed),
        "personne ne lui a été affecté : c'est un oubli, pas une urgence de calendrier"
    );
}

#[tokio::test]
async fn un_dossier_confie_dont_lecheance_est_lointaine_nentre_pas() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;

    let appel_id = appel(&bac, decor.event_id, "cop31_appel", 300).await;
    let dossier = dossier_depose(
        &bac,
        decor.event_id,
        Some(appel_id),
        decor.organization_id,
        comptes.globale,
        "Dossier confié",
    )
    .await;
    sqlx::query!(
        "INSERT INTO programme.review_assignments (proposal_id, reviewer_id, due_at)
         VALUES ($1, $2, now() + interval '200 days')",
        dossier,
        comptes.globale
    )
    .execute(bac.pool())
    .await
    .expect("affectation");
    rafraichir(&bac).await;

    let actions = analytics::service::dashboard::composer(bac.pool(), decor.event_id)
        .await
        .expect("composition")
        .actions;

    assert!(
        !actions
            .iter()
            .any(|a| a.kind == AdminActionKind::ProposalsUnreviewed),
        "un dossier déposé la veille d'un appel qui ferme dans dix mois est le fonctionnement normal"
    );
}

#[tokio::test]
async fn lecheance_dune_affectation_prime_celle_de_lappel() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;

    let appel_id = appel(&bac, decor.event_id, "cop31_appel", 300).await;
    let dossier = dossier_depose(
        &bac,
        decor.event_id,
        Some(appel_id),
        decor.organization_id,
        comptes.globale,
        "Dossier confié",
    )
    .await;
    sqlx::query!(
        "INSERT INTO programme.review_assignments (proposal_id, reviewer_id, due_at)
         VALUES ($1, $2, now() + interval '3 days')",
        dossier,
        comptes.globale
    )
    .execute(bac.pool())
    .await
    .expect("affectation");
    rafraichir(&bac).await;

    let ligne = analytics::service::dashboard::composer(bac.pool(), decor.event_id)
        .await
        .expect("composition")
        .actions
        .into_iter()
        .find(|a| a.kind == AdminActionKind::ProposalsUnreviewed)
        .expect("la famille s'allume sur l'échéance de la REVUE, pas celle de l'appel");

    let due = ligne.due_at.expect("l'échéance applicable");
    let jours = (due - time::OffsetDateTime::now_utc()).whole_days();
    assert!(
        (0..=4).contains(&jours),
        "l'échéance rendue est celle de l'affectation ({jours} jours), pas les 300 de l'appel"
    );
}
